// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use crate::rendering::common::TEXT_RGB565;
use crate::rendering::layout::{
    block_indent_px, content_h, first_line_indent, text_w, GUTTER_PAD, GUTTER_W, PAD_LEFT,
    ROW_FLAG_INDENT, ROW_FLAG_JUSTIFY, ROW_FLAG_MONO,
};
use crate::rendering::text_render;
use crate::Row;
use kobo_core::html_text::StyleRun;

use crate::rendering::common::{is_rtl, rgb565_as_bytes, ACCENT_BAR_RGB565};

pub struct PageView<'a> {
    pub w: usize,
    pub h: usize,
    pub rows: &'a [Row],
    pub page: usize,
    pub pages: &'a [(usize, usize)],
    pub content_top: usize,
    pub row_heights: &'a [i32],
    pub decoded_images: &'a std::collections::HashMap<usize, text_render::DecodedImage>,
    pub body_px: f32,
    pub head_px: f32,
    pub line_h: i32,
    pub style_runs: &'a [StyleRun],
}

pub fn overlay_text(buf: &mut [Rgb565Pixel], pv: &PageView) {
    let (w, h) = (pv.w, pv.h);
    let (rows, page, pages) = (pv.rows, pv.page, pv.pages);
    let (content_top, row_heights) = (pv.content_top, pv.row_heights);
    let (decoded_images, style_runs) = (pv.decoded_images, pv.style_runs);
    let (body_px, head_px, line_h) = (pv.body_px, pv.head_px, pv.line_h);
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
                // Centre the figure in the text column. `decode_image` fits the
                // image to the column width, but a TALL image is capped by
                // height instead and comes back narrower than the column --
                // left-aligned that leaves a lopsided white gap. Kobo/Kindle
                // centre figures, so do the same, caption included.
                let img_x = text_x + text_w().saturating_sub(iw) / 2;
                text_render::blit_rgb565_image(buf_bytes, w, rgb, iw, ih, img_x, y, w, h);
                if !cap.is_empty() {
                    let cap_y = y + ih + 2;
                    let cap_w = text_render::word_width(cap, body_px) as usize;
                    let cap_x = text_x + text_w().saturating_sub(cap_w) / 2;
                    text_render::blit_rgb565(buf_bytes, w, cap, body_px, cap_x, cap_y, w, h);
                }
            }
        } else if !row.text.is_empty() {
            let px = if row.kind == 2 { head_px } else { body_px };
            let lh = text_render::line_height(px);
            let vy = y + (row_h.saturating_sub(lh)) / 2;
            let script = text_render::detect_script(&row.text);
            if script.is_rtl() {
                let tw = text_render::word_width(&row.text, px);
                let right_edge = PAD_LEFT + GUTTER_W + GUTTER_PAD + text_w();
                let render_x = right_edge.saturating_sub(tw as usize).max(text_x);
                text_render::blit_rgb565(buf_bytes, w, row.text.as_str(), px, render_x, vy, w, h);
            } else {
                // Two independent insets: the block indent applies to every
                // line of a code listing, the first-line indent only to a
                // prose paragraph's opening line. They never co-occur (a block
                // indent marks the block as code, which suppresses the prose
                // one) but they add cleanly if that ever changes.
                let indent = block_indent_px(row)
                    + if row.kind == 0 && (row.tag & ROW_FLAG_INDENT) != 0 {
                        first_line_indent(body_px)
                    } else {
                        0
                    };
                let x0 = text_x + indent;
                let style = text_render::TextStyle {
                    mono: row.kind == 0 && (row.tag & ROW_FLAG_MONO) != 0,
                    ..Default::default()
                };
                // Emphasis lives outside the row (see ChapterState::style_runs),
                // so a row is drawn as one piece per style change. Bold and
                // italic keep the regular advances, so splitting here cannot
                // move a glyph away from where wrapping put it.
                let has_runs = row_has_runs(style_runs, row);
                let mut rb = RowBlit {
                    buf: buf_bytes,
                    w,
                    h,
                    runs: style_runs,
                    base: style,
                };
                if row.kind == 0 && (row.tag & ROW_FLAG_JUSTIFY) != 0 {
                    let avail = text_w().saturating_sub(indent);
                    blit_justified(&mut rb, row, px, (x0, vy), avail);
                } else if has_runs {
                    blit_styled_pieces(&mut rb, row, px, x0, vy);
                } else {
                    text_render::blit_rgb565_styled(
                        buf_bytes,
                        w,
                        row.text.as_str(),
                        px,
                        x0,
                        vy,
                        TEXT_RGB565,
                        style,
                        w,
                        h,
                    );
                }
            }
        }
        y += row_h;
    }
}

/// Does any emphasis run touch this row?
fn row_has_runs(runs: &[StyleRun], row: &Row) -> bool {
    if row.kind != 0 || row.start >= row.end {
        return false;
    }
    let (s, e) = (row.start as usize, row.end as usize);
    runs.iter().any(|r| r.start < e && r.end > s)
}

use text_render::style_for;

/// Byte ranges of the words in a row, as offsets into its own text.
fn word_spans(text: &str) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut start: Option<usize> = None;
    for (i, c) in text.char_indices() {
        if c == ' ' {
            if let Some(s) = start.take() {
                out.push((s, i));
            }
        } else if start.is_none() {
            start = Some(i);
        }
    }
    if let Some(s) = start {
        out.push((s, text.len()));
    }
    out
}

/// Draw a row one piece at a time, switching face wherever the style changes.
struct RowBlit<'a> {
    buf: &'a mut [u8],
    w: usize,
    h: usize,
    runs: &'a [StyleRun],
    base: text_render::TextStyle,
}

fn blit_styled_pieces(rb: &mut RowBlit, row: &Row, px: f32, x0: usize, y: usize) {
    let text = row.text.as_str();
    let row_start = row.start as usize;
    let mut x = x0 as f32;
    let mut piece_start = 0usize;
    let mut piece_style = style_for(rb.runs, row_start, rb.base);
    for (off, _) in text.char_indices().skip(1) {
        let st = style_for(rb.runs, row_start + off, rb.base);
        if st != piece_style {
            let piece = &text[piece_start..off];
            text_render::blit_rgb565_styled(
                rb.buf,
                rb.w,
                piece,
                px,
                x as usize,
                y,
                TEXT_RGB565,
                piece_style,
                rb.w,
                rb.h,
            );
            x += text_render::word_width_styled(piece, px, piece_style);
            piece_start = off;
            piece_style = st;
        }
    }
    let tail = &text[piece_start..];
    if !tail.is_empty() {
        text_render::blit_rgb565_styled(
            rb.buf,
            rb.w,
            tail,
            px,
            x as usize,
            y,
            TEXT_RGB565,
            piece_style,
            rb.w,
            rb.h,
        );
    }
}

/// Draw a line justified: widen the gaps between words so the line fills
/// `avail_w`. Falls back to plain left-alignment for single-word lines and when
/// the required gap is too tight or too wide (which would read as rivers).
///
/// Words are styled individually, so an emphasised word inside a justified
/// paragraph keeps its emphasis without disturbing the gap arithmetic -- bold
/// and italic do not change advances.
fn blit_justified(rb: &mut RowBlit, row: &Row, px: f32, pos: (usize, usize), avail_w: usize) {
    let (x0, y) = pos;
    let text = row.text.as_str();
    let row_start = row.start as usize;
    let words = word_spans(text);
    let width_of = |off: usize, word: &str| {
        text_render::word_width_styled(word, px, style_for(rb.runs, row_start + off, rb.base))
    };
    if words.len() < 2 {
        let st = style_for(rb.runs, row_start, rb.base);
        text_render::blit_rgb565_styled(rb.buf, rb.w, text, px, x0, y, TEXT_RGB565, st, rb.w, rb.h);
        return;
    }
    let total: f32 = words.iter().map(|(a, b)| width_of(*a, &text[*a..*b])).sum();
    let gaps = (words.len() - 1) as f32;
    let gap = (avail_w as f32 - total) / gaps;
    let gap = if (4.0..=28.0).contains(&gap) {
        gap
    } else {
        8.0
    };
    let mut x = x0 as f32;
    for (a, b) in &words {
        let word = &text[*a..*b];
        let st = style_for(rb.runs, row_start + *a, rb.base);
        text_render::blit_rgb565_styled(
            rb.buf,
            rb.w,
            word,
            px,
            x as usize,
            y,
            TEXT_RGB565,
            st,
            rb.w,
            rb.h,
        );
        x += width_of(*a, word) + gap;
    }
}

pub fn refresh_text_cache(cache: &mut [Rgb565Pixel], pv: &PageView) {
    cache.fill(Rgb565Pixel(0xFFFF));
    overlay_text(cache, pv);
}

pub fn composite_text(
    buf: &mut [Rgb565Pixel],
    text_cache: &[Rgb565Pixel],
    pv: &PageView,
    cur_start: i32,
    cur_end: i32,
) {
    let (w, h) = (pv.w, pv.h);
    let (rows, page, pages) = (pv.rows, pv.page, pv.pages);
    let (content_top, row_heights) = (pv.content_top, pv.row_heights);
    let line_h = pv.line_h;
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
