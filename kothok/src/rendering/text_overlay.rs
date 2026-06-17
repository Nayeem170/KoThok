use slint::platform::software_renderer::Rgb565Pixel;

use crate::rendering::layout::{content_h, text_w, GUTTER_PAD, GUTTER_W, PAD_LEFT};
use crate::rendering::text_render;
use crate::Row;

use crate::rendering::common::{is_rtl, rgb565_as_bytes, ACCENT_BAR_RGB565};

pub fn overlay_text(
    buf: &mut [Rgb565Pixel],
    w: usize,
    h: usize,
    rows: &[Row],
    page: usize,
    pages: &[(usize, usize)],
    content_top: usize,
    row_heights: &[i32],
    decoded_images: &std::collections::HashMap<usize, crate::rendering::text_render::DecodedImage>,
    body_px: f32,
    head_px: f32,
    line_h: i32,
) {
    let (s, e) = pages.get(page).copied().unwrap_or((0, rows.len()));
    let text_x = PAD_LEFT + GUTTER_W + GUTTER_PAD;
    let buf_bytes = rgb565_as_bytes(buf);
    let mut y = content_top;
    let max_y = content_top + content_h() as usize;
    for (ri, row) in rows[s..e].iter().enumerate() {
        let row_idx = s + ri;
        if y >= max_y {
            break;
        }
        let row_h = *row_heights.get(row_idx).unwrap_or(&line_h) as usize;
        if row.kind == 1 {
            if let Some(img) = decoded_images.get(&row_idx) {
                let (rgb, iw, ih) = (img.rgb.as_slice(), img.width, img.height);
                let cap = row.text.as_str();
                text_render::blit_rgb565_image(buf_bytes, w, rgb, iw, ih, text_x, y, w, h);
                if !cap.is_empty() {
                    let cap_y = y + ih + 2;
                    text_render::blit_rgb565(buf_bytes, w, cap, body_px, text_x, cap_y, w, h);
                }
            }
        } else if !row.text.is_empty() {
            let px = if row.kind == 2 { head_px } else { body_px };
            let lh = text_render::line_height(px);
            let vy = y + (row_h.saturating_sub(lh)) / 2;
            let script = text_render::detect_script(&row.text);
            let render_x = if script.is_rtl() {
                let tw = text_render::word_width(&row.text, px);
                let right_edge = PAD_LEFT + GUTTER_W + GUTTER_PAD + text_w();
                right_edge.saturating_sub(tw as usize).max(text_x)
            } else {
                text_x
            };
            text_render::blit_rgb565(buf_bytes, w, row.text.as_str(), px, render_x, vy, w, h);
        }
        y += row_h;
    }
}

pub fn refresh_text_cache(
    cache: &mut [Rgb565Pixel],
    w: usize,
    h: usize,
    rows: &[Row],
    page: usize,
    pages: &[(usize, usize)],
    content_top: usize,
    row_heights: &[i32],
    decoded_images: &std::collections::HashMap<usize, crate::rendering::text_render::DecodedImage>,
    body_px: f32,
    head_px: f32,
    line_h: i32,
) {
    cache.fill(Rgb565Pixel(0xFFFF));
    overlay_text(
        cache,
        w,
        h,
        rows,
        page,
        pages,
        content_top,
        row_heights,
        decoded_images,
        body_px,
        head_px,
        line_h,
    );
}

pub fn composite_text(
    buf: &mut [Rgb565Pixel],
    text_cache: &[Rgb565Pixel],
    w: usize,
    h: usize,
    rows: &[Row],
    page: usize,
    pages: &[(usize, usize)],
    content_top: usize,
    row_heights: &[i32],
    line_h: i32,
    cur_start: i32,
    cur_end: i32,
) {
    let (s, e) = pages.get(page).copied().unwrap_or((0, rows.len()));
    let mut y = content_top;
    let content_end = (content_top + content_h() as usize).min(h);
    let accent = cur_start < cur_end;
    let rtl = is_rtl();
    let (gutter_left, gutter_right) = if rtl {
        (w - PAD_LEFT - GUTTER_W, w - PAD_LEFT)
    } else {
        (PAD_LEFT, PAD_LEFT + GUTTER_W)
    };
    let row_count = rows[s..e].len();
    for (ri, row) in rows[s..e].iter().enumerate() {
        let row_idx = s + ri;
        let row_h = *row_heights.get(row_idx).unwrap_or(&line_h) as usize;
        let is_last = ri + 1 == row_count;
        let copy_h = if is_last {
            content_end.saturating_sub(y)
        } else {
            row_h
        };
        let row_accent = accent && row.start < cur_end && row.end > cur_start;
        if !row.text.is_empty() || row.kind == 1 {
            for ry in 0..copy_h {
                if y + ry >= h {
                    break;
                }
                let row_start = (y + ry) * w;
                let src_start = row_start;
                for x in 0..w {
                    let t = text_cache[src_start + x].0;
                    let d = buf[row_start + x].0;
                    if t != 0xFFFF && t != d {
                        buf[row_start + x].0 = t;
                    }
                }
                if row_accent {
                    for gx in gutter_left..gutter_right {
                        buf[row_start + gx].0 = ACCENT_BAR_RGB565;
                    }
                }
            }
        }
        y += row_h;
    }
}
