// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::data::library::EpubEntry;
use crate::rendering::density::{dp, dpf};
use crate::rendering::layout::BODY_PX;
use crate::rendering::text_render;

use crate::rendering::covers::{paint_cover_cached, CoverCache};
use crate::rendering::draw::{
    fill_rounded_rect, measure_text, paint_progress_bar, paint_wrapped_text,
};

use super::filter::{LibraryFilter, FILTERS};
use super::layout::{card_layout, CardLayout, GRID_ROWS};

const PILL_BORDER: u16 = 0x2104;
const CARD_BORDER: u16 = 0x2104;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PillRect {
    pub filter: LibraryFilter,
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

fn pill_label(filter: LibraryFilter, books: &[EpubEntry]) -> String {
    let n = books.iter().skip(1).filter(|b| filter.matches(b)).count();
    format!("{} {}", filter.label(), n)
}

pub fn pill_rects(books: &[EpubEntry]) -> Vec<PillRect> {
    let l = card_layout();
    let px = dpf(BODY_PX * 0.5);
    let lh = text_render::line_height(px);
    let pill_h = dp(38).max(lh as i32 + dp(8));
    let y = l.pills_y + (l.pills_h - pill_h) / 2;
    let pad_x = dp(14) as i32;
    let gap = dp(8);
    let mut x = l.pad;
    let mut out = Vec::new();
    for filter in FILTERS {
        let w = measure_text(&pill_label(filter, books), px) as i32 + 2 * pad_x;
        if x + w > crate::w() as i32 - l.pad {
            break;
        }
        out.push(PillRect {
            filter,
            x,
            y,
            w,
            h: pill_h,
        });
        x += w + gap;
    }
    out
}

pub(super) fn paint_pills(buf_bytes: &mut [u8], books: &[EpubEntry], active: LibraryFilter) {
    let px = dpf(BODY_PX * 0.5);
    let lh = text_render::line_height(px);
    let pad_x = dp(14) as usize;
    for pill in pill_rects(books) {
        let label = pill_label(pill.filter, books);
        let (x, y, w, pill_h) = (pill.x, pill.y, pill.w as usize, pill.h);
        let ty = (y + (pill_h - lh as i32) / 2).max(0) as usize;
        let is_active = pill.filter == active;
        let (fill, fg) = if is_active {
            (0x0000u16, 0xFFFFu16)
        } else {
            (0xFFFFu16, 0x0000u16)
        };
        fill_rounded_rect(
            buf_bytes,
            crate::w(),
            crate::h(),
            x as usize,
            y as usize,
            w,
            pill_h as usize,
            fill,
            PILL_BORDER,
            (pill_h / 2) as usize,
        );
        text_render::blit_rgb565_color(
            buf_bytes,
            crate::w(),
            &label,
            px,
            x as usize + pad_x,
            ty,
            fg,
            crate::w(),
            crate::h(),
        );
    }
}

pub(super) fn paint_empty_filter(buf_bytes: &mut [u8], l: &CardLayout, filter: LibraryFilter) {
    let msg = match filter {
        LibraryFilter::Reading => "No books in progress",
        LibraryFilter::Finished => "No finished books yet",
        LibraryFilter::New => "No unread books",
        LibraryFilter::All => return,
    };
    let px = dpf(BODY_PX * 0.62);
    let lh = text_render::line_height(px);
    let tw = measure_text(msg, px) as i32;
    let band_h = GRID_ROWS * l.row_h + (GRID_ROWS - 1) * l.gap;
    let x = ((crate::w() as i32 - tw) / 2).max(0) as usize;
    let y = (l.grid_y + (band_h - lh as i32) / 2).max(0) as usize;
    text_render::blit_rgb565_color(
        buf_bytes,
        crate::w(),
        msg,
        px,
        x,
        y,
        0x8410,
        crate::w(),
        crate::h(),
    );
}

pub(super) fn paint_hero_card(
    buf_bytes: &mut [u8],
    cover_cache: &mut CoverCache,
    book: &EpubEntry,
    l: &CardLayout,
) {
    fill_rounded_rect(
        buf_bytes,
        crate::w(),
        crate::h(),
        l.hero_x as usize,
        l.hero_y as usize,
        l.hero_w as usize,
        l.hero_h as usize,
        0xFFFF,
        CARD_BORDER,
        dp(6) as usize,
    );
    let inpad = dp(12);
    let cover_x = l.hero_x + inpad;
    let cover_y = l.hero_y + (l.hero_h - l.hero_cover_h) / 2;
    paint_cover_cached(
        buf_bytes,
        cover_cache,
        &book.path,
        &book.cover_bytes,
        cover_x as usize,
        cover_y as usize,
        l.hero_cover_w as usize,
        l.hero_cover_h as usize,
    );
    let text_x = cover_x + l.hero_cover_w + dp(18);
    let text_w = (l.hero_x + l.hero_w - text_x - dp(14)).max(0);
    if text_w < dp(60) {
        return;
    }
    let text_x = text_x as usize;
    let text_w = text_w as usize;
    let text_top = l.hero_y + inpad;
    let text_bottom = l.hero_y + l.hero_h - inpad;
    let mut ty = text_top as usize;
    ty += paint_wrapped_text(
        buf_bytes,
        crate::w(),
        crate::h(),
        "CONTINUE READING",
        text_x,
        ty,
        text_w,
        dpf(BODY_PX * 0.5),
        1,
    );
    ty += dp(6) as usize;
    ty += paint_wrapped_text(
        buf_bytes,
        crate::w(),
        crate::h(),
        &book.title,
        text_x,
        ty,
        text_w,
        dpf(BODY_PX * 1.05),
        2,
    );
    ty += dp(4) as usize;
    if let Some(ref author) = book.author {
        paint_wrapped_text(
            buf_bytes,
            crate::w(),
            crate::h(),
            author,
            text_x,
            ty,
            text_w,
            dpf(BODY_PX * 0.6),
            1,
        );
    }
    if book.progress > 0.005 {
        let pct = (book.progress * 100.0).round() as i32;
        let pct_str = format!("{}%", pct);
        let pct_px = dpf(BODY_PX * 0.62);
        let pct_w = measure_text("100%", pct_px).max(1);
        let bar_h = dp(8) as usize;
        let bar_y = (text_bottom as usize).saturating_sub(bar_h);
        let bar_w = text_w.saturating_sub(pct_w + dp(14) as usize);
        if bar_w > dp(24) as usize {
            paint_progress_bar(
                buf_bytes,
                crate::w(),
                crate::h(),
                text_x,
                bar_y,
                bar_w,
                book.progress,
            );
            let pct_text_w = measure_text(&pct_str, pct_px);
            let lh = text_render::line_height(pct_px);
            let pct_y = (bar_y as i32 + (bar_h as i32 - lh as i32) / 2).max(0) as usize;
            let pct_x = text_x + bar_w + dp(10) as usize + (pct_w - pct_text_w);
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

pub(super) fn paint_book_card(
    buf_bytes: &mut [u8],
    cover_cache: &mut CoverCache,
    book: &EpubEntry,
    x: i32,
    y: i32,
    l: &CardLayout,
) {
    fill_rounded_rect(
        buf_bytes,
        crate::w(),
        crate::h(),
        x as usize,
        y as usize,
        l.cell_w as usize,
        l.row_h as usize,
        0xFFFF,
        CARD_BORDER,
        dp(6) as usize,
    );

    let top_pad = dp(12);
    let cover_x = x + (l.cell_w - l.cover_w) / 2;
    let cover_y = y + top_pad;
    paint_cover_cached(
        buf_bytes,
        cover_cache,
        &book.path,
        &book.cover_bytes,
        cover_x as usize,
        cover_y as usize,
        l.cover_w as usize,
        l.cover_h as usize,
    );

    let inpad = dp(10);
    let text_x = (x + inpad) as usize;
    let text_w = (l.cell_w - 2 * inpad).max(dp(24)) as usize;
    let mut ty = (cover_y + l.cover_h + dp(8)) as usize;
    ty += paint_wrapped_text(
        buf_bytes,
        crate::w(),
        crate::h(),
        &book.title,
        text_x,
        ty,
        text_w,
        dpf(BODY_PX * 0.52),
        1,
    );

    let card_bottom = (y + l.row_h - inpad) as usize;
    if book.progress > 0.005 {
        let bar_h = dp(8) as usize;
        let bar_y = card_bottom.saturating_sub(bar_h);
        if bar_y > ty {
            paint_progress_bar(
                buf_bytes,
                crate::w(),
                crate::h(),
                text_x,
                bar_y,
                text_w,
                book.progress,
            );
        }
    } else {
        let px = dpf(BODY_PX * 0.44);
        let lh = text_render::line_height(px) as usize;
        let ny = card_bottom.saturating_sub(lh);
        if ny > ty {
            text_render::blit_rgb565_color(
                buf_bytes,
                crate::w(),
                "Not started",
                px,
                text_x,
                ny,
                0x8410,
                crate::w(),
                crate::h(),
            );
        }
    }
}
