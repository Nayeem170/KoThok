use log::debug;
use slint::platform::software_renderer::{MinimalSoftwareWindow, Rgb565Pixel};
use slint::{ModelRc, VecModel};

use crate::data::library::EpubEntry;
use crate::rendering::fb::{dump_ppm, Fb, WAVE_GC16};
use crate::rendering::layout::BODY_PX;
use crate::rendering::text_render;
use crate::{Reader, Row};

use crate::rendering::common::{rgb565_as_bytes, rgb565_as_bytes_ref};
use crate::rendering::covers::{paint_cover_cached, CoverCache};
use crate::rendering::draw::{
    fill_rounded_rect, measure_text, paint_progress_bar, paint_wrapped_text,
};

const LOGO_PNG: &[u8] = include_bytes!("../../ui/kothok-logo.png");
const EXIT_PNG: &[u8] = include_bytes!("../../ui/components/assets/exit.png");

pub const GRID_GAP: i32 = 14;
pub const PICKER_PAD: i32 = 10;
pub const PICKER_HEADER_H: i32 = 80;
pub const NAV_BAR_H: i32 = 96;

const GRID_TARGET_CELL_W: usize = 300;

pub fn grid_cols_for_width(avail_w: usize) -> usize {
    let step = GRID_TARGET_CELL_W + GRID_GAP as usize;
    let cols = (avail_w + GRID_GAP as usize) / step;
    cols.clamp(2, 4)
}

pub fn grid_cols() -> usize {
    grid_cols_for_width(picker_avail_w() as usize)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GridCell {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub idx: usize,
}

pub const VISIBLE_ROWS: usize = 3;

fn library_viewport_h() -> i32 {
    crate::h() as i32 - NAV_BAR_H - PICKER_PAD - PICKER_HEADER_H
}

pub fn grid_cell_h() -> i32 {
    ((library_viewport_h() - (VISIBLE_ROWS as i32 - 1) * GRID_GAP) / VISIBLE_ROWS as i32).max(120)
}

pub fn row_pitch() -> i32 {
    grid_cell_h() + GRID_GAP
}

pub fn grid_thumb_h() -> usize {
    (grid_cell_h() as usize * 7 / 10).max(80)
}

pub fn grid_thumb_w() -> usize {
    let avail_w = picker_avail_w() as usize;
    let cols = grid_cols();
    let cell_w = (avail_w - (cols - 1) * GRID_GAP as usize) / cols;
    let by_ratio = grid_thumb_h() * 3 / 4;
    by_ratio.min(cell_w * 8 / 10)
}

pub const PICKER_NAV_TOUCH_MARGIN: i32 = 100;
pub const BEZEL_DEAD_ZONE: i32 = 2;

pub fn picker_avail_w() -> i32 {
    crate::w() as i32 - 2 * PICKER_PAD
}

fn library_total_rows(n: usize) -> usize {
    if n == 0 {
        0
    } else {
        1 + (n - 1).div_ceil(grid_cols())
    }
}

pub fn library_max_scroll(n: usize) -> i32 {
    let extra_rows = library_total_rows(n).saturating_sub(VISIBLE_ROWS) as i32;
    extra_rows.max(0) * row_pitch()
}

pub fn snap_scroll(scroll: i32) -> i32 {
    let pitch = row_pitch();
    if pitch <= 0 {
        return 0;
    }
    let snapped = (scroll + (pitch + 1) / 2) / pitch * pitch;
    snapped.max(0)
}

pub fn picker_scroll_cells(books: &[EpubEntry], scroll: i32) -> Vec<GridCell> {
    let avail_w = picker_avail_w();
    let cols = grid_cols() as i32;
    let cell_w = (avail_w - (cols - 1) * GRID_GAP) / cols;
    let pitch = row_pitch();
    let ch = grid_cell_h();
    let viewport_top = PICKER_PAD + PICKER_HEADER_H;
    let viewport_h = library_viewport_h();
    let mut cells = Vec::new();
    if books.is_empty() {
        return cells;
    }
    let n = books.len();
    let total_rows = library_total_rows(n) as i32;
    let first_row = (scroll / pitch).max(0);
    let visible_rows = VISIBLE_ROWS as i32;
    let last_row = (first_row + visible_rows - 1).min(total_rows - 1);
    for r in first_row..=last_row {
        let screen_y = viewport_top + r * pitch - scroll;
        if screen_y < viewport_top - ch || screen_y > viewport_top + viewport_h {
            continue;
        }
        if r == 0 {
            cells.push(GridCell {
                x: PICKER_PAD,
                y: screen_y,
                w: avail_w,
                h: ch,
                idx: 0,
            });
        } else {
            let book_base = 1 + ((r as usize) - 1) * cols as usize;
            for col in 0..cols {
                let idx = book_base + col as usize;
                if idx >= n {
                    break;
                }
                let x = PICKER_PAD + col * (cell_w + GRID_GAP);
                cells.push(GridCell {
                    x,
                    y: screen_y,
                    w: cell_w,
                    h: ch,
                    idx,
                });
            }
        }
    }
    cells
}

pub fn show_book_picker(
    reader: &Reader,
    fb: &Fb,
    window: &MinimalSoftwareWindow,
    buffer: &mut [Rgb565Pixel],
    text_cache: &mut [Rgb565Pixel],
    cover_cache: &mut CoverCache,
    books: &[EpubEntry],
    scroll: i32,
    clock: &str,
    battery: i32,
    prompt: &str,
) {
    let cells = picker_scroll_cells(books, scroll);
    reader.set_rows(ModelRc::new(VecModel::from(Vec::<Row>::new())));
    reader.set_picker_active(true);
    window.request_redraw();
    // best-effort: Slint draw may be no-op if no redraw pending
    let _ = window.draw_if_needed(|r| {
        r.render(buffer, crate::w());
    });
    buffer.fill(Rgb565Pixel(0xFFFF));
    text_cache.fill(Rgb565Pixel(0xFFFF));
    let buf_bytes = rgb565_as_bytes(text_cache);
    let avail_w = picker_avail_w() as usize;
    for cell in &cells {
        let book = match books.get(cell.idx) {
            Some(b) => b,
            None => continue,
        };
        if cell.w as usize == avail_w {
            paint_hero_cell(
                buf_bytes,
                cover_cache,
                book,
                cell.x as usize,
                cell.y as usize,
                cell.w as usize,
                cell.h as usize,
            );
        } else {
            paint_grid_cell(
                buf_bytes,
                cover_cache,
                book,
                cell.x as usize,
                cell.y as usize,
                cell.w as usize,
            );
        }
    }
    paint_library_header(buf_bytes, crate::w(), crate::h());
    let nav_y = crate::h().saturating_sub(NAV_BAR_H as usize);
    let center = if prompt.is_empty() {
        format!("{} books - swipe up/down", books.len())
    } else {
        prompt.to_string()
    };
    paint_picker_nav_bar(
        buf_bytes,
        crate::w(),
        crate::h(),
        nav_y,
        NAV_BAR_H as usize,
        &center,
        clock,
        battery,
    );

    buffer.copy_from_slice(text_cache);
    if cfg!(feature = "ppm-dump") {
        dump_ppm(
            crate::data::config::PPM_DEBUG,
            rgb565_as_bytes_ref(buffer),
            crate::w(),
            crate::h(),
        );
        // best-effort: debug copy for post-mortem
        let _ = std::fs::copy(
            crate::data::config::PPM_DEBUG,
            crate::data::config::PPM_DEPLOY,
        );
    }
    fb.present(
        rgb565_as_bytes_ref(buffer),
        crate::w(),
        crate::h(),
        false,
        0,
        crate::h(),
        WAVE_GC16,
    );
    debug!("picker: scroll={} ({} books)", scroll, books.len());
}

fn paint_hero_cell(
    buf_bytes: &mut [u8],
    cover_cache: &mut CoverCache,
    book: &EpubEntry,
    cell_x: usize,
    cell_y: usize,
    cell_w: usize,
    cell_h: usize,
) {
    let cover_h = (cell_h.saturating_sub(16)).min(grid_thumb_h() * 3 / 2);
    let cover_w = (cover_h * 3 / 4).min(cell_w / 3);
    let cover_x = cell_x + 10;
    let cover_y = cell_y + (cell_h.saturating_sub(cover_h)) / 2;
    paint_cover_cached(
        buf_bytes,
        cover_cache,
        &book.path,
        &book.cover_bytes,
        cover_x,
        cover_y,
        cover_w,
        cover_h,
    );
    let text_x = cover_x + cover_w + 18;
    let text_w = cell_x + cell_w - text_x - 14;
    if text_w >= 60 {
        let pad_v = 12usize;
        let text_top = cell_y + pad_v;
        let text_bottom = cell_y + cell_h - pad_v;
        let mut ty = text_top;
        ty += paint_wrapped_text(
            buf_bytes,
            crate::w(),
            crate::h(),
            "Continue Reading",
            text_x,
            ty,
            text_w,
            BODY_PX * 0.7,
            1,
        );
        ty += 8;
        ty += paint_wrapped_text(
            buf_bytes,
            crate::w(),
            crate::h(),
            &book.title,
            text_x,
            ty,
            text_w,
            BODY_PX * 1.35,
            2,
        );
        ty += 6;
        if let Some(ref author) = book.author {
            paint_wrapped_text(
                buf_bytes,
                crate::w(),
                crate::h(),
                author,
                text_x,
                ty,
                text_w,
                BODY_PX * 0.82,
                1,
            );
        }
        if book.progress > 0.005 {
            let pct = (book.progress * 100.0).round() as i32;
            let pct_str = format!("{}%", pct);
            let pct_px = BODY_PX * 0.8;
            let pct_w = measure_text("100%", pct_px).max(1);
            let bar_w = text_w.saturating_sub(pct_w + 14);
            let bar_h = 8usize;
            let bar_y = text_bottom.saturating_sub(bar_h);
            if bar_w > 24 {
                paint_progress_bar(
                    buf_bytes,
                    crate::w(),
                    crate::h(),
                    text_x,
                    bar_y,
                    bar_w,
                    book.progress,
                );
            }
            let pct_text_w = measure_text(&pct_str, pct_px);
            let lh = text_render::line_height(pct_px);
            let pct_y = (bar_y as i32 + (bar_h as i32 - lh as i32) / 2).max(0) as usize;
            let pct_x = text_x + bar_w + 10 + (pct_w - pct_text_w);
            text_render::blit_rgb565(
                buf_bytes,
                crate::w(),
                &pct_str,
                pct_px,
                pct_x,
                pct_y,
                crate::w(),
                crate::h(),
            );
        }
    }
}

fn paint_grid_cell(
    buf_bytes: &mut [u8],
    cover_cache: &mut CoverCache,
    book: &EpubEntry,
    x: usize,
    y: usize,
    w: usize,
) {
    let tw = grid_thumb_w();
    let th = grid_thumb_h();
    let cover_x = x + (w.saturating_sub(tw)) / 2;
    let cover_y = y + 4;
    paint_cover_cached(
        buf_bytes,
        cover_cache,
        &book.path,
        &book.cover_bytes,
        cover_x,
        cover_y,
        tw,
        th,
    );
    let text_x = x + 6;
    let text_w = w.saturating_sub(12);
    if text_w < 40 {
        return;
    }
    let mut ty = cover_y + th + 8;
    ty += paint_wrapped_text(
        buf_bytes,
        crate::w(),
        crate::h(),
        &book.title,
        text_x,
        ty,
        text_w,
        BODY_PX * 0.8,
        2,
    );
    if let Some(ref author) = book.author {
        ty += paint_wrapped_text(
            buf_bytes,
            crate::w(),
            crate::h(),
            author,
            text_x,
            ty,
            text_w,
            BODY_PX * 0.62,
            1,
        );
    }
    if book.progress > 0.005 {
        ty += 8;
        paint_progress_bar(
            buf_bytes,
            crate::w(),
            crate::h(),
            text_x,
            ty,
            text_w,
            book.progress,
        );
    }
}

const NAV_BORDER_COLOR: u16 = 0x1082;
const NAV_TEXT_PX: f32 = BODY_PX * 0.66;

fn paint_picker_nav_bar(
    buf_bytes: &mut [u8],
    screen_w: usize,
    screen_h: usize,
    nav_y: usize,
    nav_h: usize,
    center: &str,
    clock: &str,
    battery: i32,
) {
    for ry in 0..nav_h {
        let py = nav_y + ry;
        if py >= screen_h {
            break;
        }
        for rx in 0..screen_w {
            let off = (py * screen_w + rx) * 2;
            if off + 2 > buf_bytes.len() {
                break;
            }
            let v = if ry == 0 { NAV_BORDER_COLOR } else { 0xFFFF };
            buf_bytes[off] = (v & 0xff) as u8;
            buf_bytes[off + 1] = (v >> 8) as u8;
        }
    }
    let lh = text_render::line_height(NAV_TEXT_PX);
    let cy = nav_y + (nav_h.saturating_sub(lh)) / 2;

    if !clock.is_empty() {
        text_render::blit_rgb565(
            buf_bytes, screen_w, clock, NAV_TEXT_PX, 24, cy, screen_w, screen_h,
        );
    }
    if battery > 0 {
        let bat_str = format!("{}%", battery);
        let bw = measure_text(&bat_str, NAV_TEXT_PX);
        let bx = screen_w.saturating_sub(bw + 24);
        text_render::blit_rgb565(
            buf_bytes, screen_w, &bat_str, NAV_TEXT_PX, bx, cy, screen_w, screen_h,
        );
    }
    if !center.is_empty() {
        let cw = measure_text(center, NAV_TEXT_PX);
        let cx = (screen_w / 2).saturating_sub(cw / 2);
        text_render::blit_rgb565(
            buf_bytes, screen_w, center, NAV_TEXT_PX, cx, cy, screen_w, screen_h,
        );
    }
}

const HEADER_LOGO_PX: usize = 48;
const HEADER_LOGO_X: usize = 18;
const HEADER_LOGO_Y: usize = 16;
const HEADER_BTN_PX: usize = 48;
const HEADER_BTN_Y: usize = 16;
const HEADER_ICON_PX: usize = 28;
const HEADER_TEXT_PX: f32 = BODY_PX * 1.1;
const HEADER_SEP_COLOR: u16 = 0xD6BA;

pub fn header_exit_x(screen_w: usize) -> usize {
    screen_w - HEADER_BTN_PX - HEADER_LOGO_X
}

fn paint_library_header(buf_bytes: &mut [u8], screen_w: usize, screen_h: usize) {
    let header_h = PICKER_HEADER_H as usize;
    for ry in 0..header_h {
        if ry >= screen_h {
            break;
        }
        for rx in 0..screen_w {
            let off = (ry * screen_w + rx) * 2;
            if off + 2 > buf_bytes.len() {
                break;
            }
            let v = 0xFFFF;
            buf_bytes[off] = (v & 0xff) as u8;
            buf_bytes[off + 1] = (v >> 8) as u8;
        }
    }
    if let Some(img) = text_render::decode_image_rgba(LOGO_PNG, HEADER_LOGO_PX, HEADER_LOGO_PX) {
        text_render::blit_rgb565_image_alpha(
            buf_bytes,
            screen_w,
            &img.rgba,
            img.width,
            img.height,
            HEADER_LOGO_X,
            HEADER_LOGO_Y,
            screen_w,
            screen_h,
        );
    }
    let lh = text_render::line_height(HEADER_TEXT_PX);
    let ty = (header_h.saturating_sub(lh)) / 2;
    text_render::blit_rgb565(
        buf_bytes,
        screen_w,
        "KoThok",
        HEADER_TEXT_PX,
        HEADER_LOGO_X + HEADER_LOGO_PX + 16,
        ty,
        screen_w,
        screen_h,
    );
    let exit_x = header_exit_x(screen_w);
    const EXIT_RED: u16 = crate::rendering::common::BRAND_RED_RGB565;
    fill_rounded_rect(
        buf_bytes,
        screen_w,
        screen_h,
        exit_x,
        HEADER_BTN_Y,
        HEADER_BTN_PX,
        HEADER_BTN_PX,
        EXIT_RED,
        EXIT_RED,
        HEADER_BTN_PX / 2,
    );
    if let Some(img) = text_render::decode_image_rgba(EXIT_PNG, HEADER_ICON_PX, HEADER_ICON_PX) {
        let icon_x = exit_x + (HEADER_BTN_PX - img.width) / 2;
        let icon_y = HEADER_BTN_Y + (HEADER_BTN_PX - img.height) / 2;
        text_render::blit_rgb565_image_alpha(
            buf_bytes,
            screen_w,
            &img.rgba,
            img.width,
            img.height,
            icon_x,
            icon_y,
            screen_w,
            screen_h,
        );
    }
    let sep_y = header_h.saturating_sub(1);
    for rx in 0..screen_w {
        let off = (sep_y * screen_w + rx) * 2;
        if off + 2 > buf_bytes.len() {
            break;
        }
        buf_bytes[off] = (HEADER_SEP_COLOR & 0xff) as u8;
        buf_bytes[off + 1] = (HEADER_SEP_COLOR >> 8) as u8;
    }
}

#[cfg(test)]
mod tests;
