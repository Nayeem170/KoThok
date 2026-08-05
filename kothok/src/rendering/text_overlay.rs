// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use crate::rendering::common::TEXT_RGB565;
use crate::rendering::layout::{
    block_indent_px, content_h, first_line_indent, pad_left, text_w, GUTTER_PAD, GUTTER_W,
    ROW_FLAG_BQ, ROW_FLAG_CENTER, ROW_FLAG_INDENT, ROW_FLAG_JUSTIFY, ROW_FLAG_MONO,
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
    let (body_px, _head_px, line_h) = (pv.body_px, pv.head_px, pv.line_h);
    let (s, e) = pages.get(page).copied().unwrap_or((0, rows.len()));
    let text_x = pad_left() + GUTTER_W + GUTTER_PAD;
    let buf_bytes = rgb565_as_bytes(buf);
    let mut y = content_top;
    let max_y = content_top + content_h() as usize;
    for (ri, row) in rows.get(s..e).unwrap_or(&[]).iter().enumerate() {
        let row_idx = s + ri;
        if y >= max_y {
            break;
        }
        let row_h = *row_heights.get(row_idx).unwrap_or(&line_h) as usize;
        if row.kind == 1 {
            if let Some(img) = decoded_images.get(&row_idx) {
                let (rgb, iw, ih) = (img.rgb.as_slice(), img.width, img.height);
                // Centre the figure in the text column. `decode_image` fits the
                // image to the column width, but a TALL image is capped by
                // height instead and comes back narrower than the column --
                // left-aligned that leaves a lopsided white gap. Kobo/Kindle
                // centre figures, so do the same.
                //
                // The caption is not drawn here: it is emitted as its own
                // centred text rows so it wraps, reaches read-aloud, and
                // survives an image that fails to decode.
                let img_x = text_x + text_w().saturating_sub(iw) / 2;
                text_render::blit_rgb565_image(buf_bytes, w, rgb, iw, ih, img_x, y, w, h);
            }
        } else if !row.text.is_empty() {
            let px = row_px(row, body_px);
            let lh = text_render::line_height(px);
            let vy = y + (row_h.saturating_sub(lh)) / 2;
            let script = text_render::detect_script(&row.text);
            if script.is_rtl() {
                let tw = text_render::word_width(&row.text, px);
                let right_edge = pad_left() + GUTTER_W + GUTTER_PAD + text_w();
                let render_x = right_edge.saturating_sub(tw as usize).max(text_x);
                text_render::blit_rgb565(buf_bytes, w, row.text.as_str(), px, render_x, vy, w, h);
            } else {
                let (x0, indent) = row_origin_x(row, px, body_px);
                let style = text_render::TextStyle {
                    mono: (row_flags(row) & ROW_FLAG_MONO) != 0,
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
                if (row_flags(row) & ROW_FLAG_JUSTIFY) != 0 {
                    let avail = text_w().saturating_sub(indent);
                    blit_justified(&mut rb, row, px, (x0, vy), avail);
                } else if has_runs {
                    blit_styled_pieces(&mut rb, row, px, x0, vy);
                    draw_link_underlines(buf_bytes, w, h, style_runs, row, x0, vy, px);
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

/// Layout flags carried in `Row::tag`, or 0 for rows that use `tag` otherwise.
///
/// Only body rows (`kind == 0`) pack flags there. A heading stores its *level*
/// in the same field, and the levels alias the flags exactly -- h1 is
/// `ROW_FLAG_JUSTIFY`, h2 is `ROW_FLAG_INDENT`, h4 is `ROW_FLAG_MONO`. Reading
/// `tag` directly would justify every h1 and set every h4 in the monospace
/// face, so every flag read goes through here.
fn row_flags(row: &Row) -> i32 {
    if row.kind == 0 {
        row.tag
    } else {
        0
    }
}

/// Font size a row is drawn at.
///
/// Both the draw and the hit-test resolve it through here. A row measured at
/// one size and drawn at another is the recurring defect in this file: it has
/// produced clipped headings, overflowing code blocks and taps landing on the
/// wrong character, each time because two call sites computed the size apart.
fn row_px(row: &Row, body_px: f32) -> f32 {
    if row.kind == 2 {
        body_px * crate::rendering::layout::heading_scale(row.tag.max(1) as u32)
    } else if (row_flags(row) & ROW_FLAG_MONO) != 0 {
        body_px * crate::rendering::layout::MONO_SCALE
    } else {
        body_px
    }
}

/// Left edge a row starts at, plus the block inset that produced it.
///
/// `indent` is returned alongside because justification needs the inset (to
/// know how much column is left), while the blit needs the origin. Centred
/// rows ignore the inset entirely, which is why the two cannot be derived from
/// one another at the call site.
///
/// Two independent insets feed it: the block indent applies to every line of a
/// code listing, the first-line indent only to a prose paragraph's opening
/// line. They never co-occur (a block indent marks the block as code, which
/// suppresses the prose one) but they add cleanly if that ever changes.
fn row_origin_x(row: &Row, px: f32, body_px: f32) -> (usize, usize) {
    let text_x = pad_left() + GUTTER_W + GUTTER_PAD;
    let indent = block_indent_px(row)
        + if (row_flags(row) & ROW_FLAG_INDENT) != 0 {
            first_line_indent(body_px)
        } else {
            0
        };
    let x0 = if (row_flags(row) & ROW_FLAG_CENTER) != 0 {
        let tw = text_render::word_width(row.text.as_str(), px) as usize;
        text_x + text_w().saturating_sub(tw) / 2
    } else {
        text_x + indent
    };
    (x0, indent)
}

/// The `body` byte offset under a screen point, if it lands on a text row.
///
/// Mirrors the placement `overlay_text` performs -- same row walk, same left
/// inset, same justification gaps -- because a tap has to resolve to the glyph
/// the reader actually sees. Any drift between the two puts the hit-test on a
/// different character than the one under the finger.
pub fn offset_at_point(pv: &PageView, tap_x: usize, tap_y: usize) -> Option<usize> {
    let (rows, page, pages) = (pv.rows, pv.page, pv.pages);
    let (content_top, row_heights) = (pv.content_top, pv.row_heights);
    let (body_px, line_h) = (pv.body_px, pv.line_h);
    let (s, e) = pages.get(page).copied().unwrap_or((0, rows.len()));
    let mut y = content_top;
    let max_y = content_top + content_h() as usize;
    for (ri, row) in rows.get(s..e).unwrap_or(&[]).iter().enumerate() {
        let row_idx = s + ri;
        if y >= max_y {
            break;
        }
        let row_h = *row_heights.get(row_idx).unwrap_or(&line_h) as usize;
        let hit = tap_y >= y && tap_y < y + row_h;
        y += row_h;
        // Body and heading rows both carry byte ranges and can hold links.
        // Image rows (1) and gap rows (3) have none.
        let tappable = row.kind == 0 || row.kind == 2;
        if !hit || !tappable || row.text.is_empty() || row.start >= row.end {
            continue;
        }
        let px = row_px(row, body_px);
        let (x0, indent) = row_origin_x(row, px, body_px);
        if tap_x < x0 {
            return Some(row.start as usize);
        }
        let text = row.text.as_str();
        let base = row.start as usize;
        if (row_flags(row) & ROW_FLAG_JUSTIFY) != 0 {
            let avail = text_w().saturating_sub(indent);
            return Some(justified_offset_at(text, base, px, x0, avail, tap_x));
        }
        let style = text_render::TextStyle {
            mono: (row_flags(row) & ROW_FLAG_MONO) != 0,
            ..Default::default()
        };
        let mut x = x0 as f32;
        for (off, ch) in text.char_indices() {
            let cw = text_render::word_width_styled(&ch.to_string(), px, style);
            if (tap_x as f32) < x + cw {
                return Some(base + off);
            }
            x += cw;
        }
        return Some((row.end as usize).saturating_sub(1));
    }
    None
}

/// Offset under `tap_x` on a justified row, using the same gap arithmetic
/// `blit_justified` draws with.
fn justified_offset_at(
    text: &str,
    base: usize,
    px: f32,
    x0: usize,
    avail_w: usize,
    tap_x: usize,
) -> usize {
    let words = word_spans(text);
    if words.len() < 2 {
        return base;
    }
    let total: f32 = words
        .iter()
        .map(|(a, b)| text_render::word_width(&text[*a..*b], px))
        .sum();
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
        let ww = text_render::word_width(word, px);
        if (tap_x as f32) < x + ww {
            let mut cx = x;
            for (off, ch) in word.char_indices() {
                let cw = text_render::word_width(&ch.to_string(), px);
                if (tap_x as f32) < cx + cw {
                    return base + a + off;
                }
                cx += cw;
            }
            return base + a;
        }
        x += ww + gap;
    }
    base + words.last().map(|(a, _)| *a).unwrap_or(0)
}

/// Does any emphasis run touch this row?
fn row_has_runs(runs: &[StyleRun], row: &Row) -> bool {
    // Headings carry ranges too, so emphasis and links inside one resolve the
    // same way they do in body text.
    if !matches!(row.kind, 0 | 2) || row.start >= row.end {
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
    let lh = text_render::line_height(px);
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
        let ww = width_of(*a, word);
        // Underline here rather than in a second pass: justification decides
        // each word's x as it goes, and every line of a paragraph except the
        // last one is justified -- a link that never landed on a closing line
        // was drawn with no underline at all.
        if st.link {
            draw_underline(
                rb.buf,
                rb.w,
                rb.h,
                y,
                lh,
                x as usize,
                x as usize + ww as usize,
            );
        }
        x += ww + gap;
    }
}

/// Underline a link, given the **top** of its line box and that box's height.
///
/// Two things this gets right that the previous version did not, and both are
/// why links read as ordinary text on the device:
///
/// - It draws in the text colour. A single row of mid-grey (`0x8410`) is the
///   one thing an e-ink panel cannot show: the waveform quantises to a handful
///   of levels and dithers the rest, so a 1px 50% line lands on a checkerboard
///   of white and near-white and disappears. Ink is ink.
/// - It sits *inside* the line box, just under the baseline. It used to be
///   placed at `line_top + line_height + 2`, which is the row below -- so the
///   mark appeared to underline the following line, and on the last row of a
///   page it fell outside the text area entirely.
fn draw_underline(
    buf_bytes: &mut [u8],
    screen_w: usize,
    screen_h: usize,
    line_top: usize,
    line_h: usize,
    seg_x: usize,
    seg_end_x: usize,
) {
    // Just below the baseline: fontdue puts the baseline at the ascent, which
    // is ~80% of the line box for the faces shipped here.
    let base = line_top + (line_h * 87) / 100;
    // Scale with the text so it stays visible at a large body size without
    // turning into a bar at a small one.
    let thickness = (line_h / 24).clamp(2, 4);
    for row in 0..thickness {
        let ul_y = base + row;
        if ul_y >= screen_h || ul_y >= line_top + line_h {
            break;
        }
        for x in seg_x..seg_end_x.min(screen_w) {
            let off = (ul_y * screen_w + x) * 2;
            if off + 2 <= buf_bytes.len() {
                buf_bytes[off] = (TEXT_RGB565 & 0xff) as u8;
                buf_bytes[off + 1] = (TEXT_RGB565 >> 8) as u8;
            }
        }
    }
}

fn draw_link_underlines(
    buf_bytes: &mut [u8],
    screen_w: usize,
    screen_h: usize,
    runs: &[StyleRun],
    row: &Row,
    x0: usize,
    vy: usize,
    px: f32,
) {
    let text = row.text.as_str();
    let row_start = row.start as usize;
    let lh = text_render::line_height(px);
    let mut x = x0 as f32;
    let mut piece_start = 0usize;
    let mut piece_style = style_for(runs, row_start, Default::default());
    for (off, _) in text.char_indices().skip(1) {
        let st = style_for(runs, row_start + off, Default::default());
        if st != piece_style {
            if piece_style.link {
                let piece = &text[piece_start..off];
                let pw = text_render::word_width_styled(piece, px, piece_style) as usize;
                draw_underline(
                    buf_bytes,
                    screen_w,
                    screen_h,
                    vy,
                    lh,
                    x as usize,
                    x as usize + pw,
                );
            }
            x += text_render::word_width_styled(&text[piece_start..off], px, piece_style);
            piece_start = off;
            piece_style = st;
        }
    }
    if piece_style.link {
        let piece = &text[piece_start..];
        let pw = text_render::word_width_styled(piece, px, piece_style) as usize;
        draw_underline(
            buf_bytes,
            screen_w,
            screen_h,
            vy,
            lh,
            x as usize,
            x as usize + pw,
        );
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
        (w - pad_left() - GUTTER_W, w - pad_left())
    } else {
        (pad_left(), pad_left() + GUTTER_W)
    };
    let row_count = rows.get(s..e).unwrap_or(&[]).len();
    for (ri, row) in rows.get(s..e).unwrap_or(&[]).iter().enumerate() {
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
                if (row_flags(row) & ROW_FLAG_BQ) != 0 {
                    let bi = block_indent_px(row);
                    let bx = (pad_left() + GUTTER_W + GUTTER_PAD)
                        .saturating_sub(bi)
                        .saturating_sub(4);
                    for dx in 0..3 {
                        let px = bx + dx;
                        if px < w && (y + ry) < h {
                            buf[row_start + px].0 = TEXT_RGB565;
                        }
                    }
                }
            }
        }
        y += row_h;
    }
}

pub fn word_boundary(text: &str, offset: usize) -> (usize, usize) {
    let bytes = text.as_bytes();
    let len = bytes.len();
    if offset >= len {
        return (len, len);
    }
    let is_delim = |b: u8| -> bool { b == b' ' || b == b'\n' || b == b'\r' || b == b'\t' };
    let mut start = offset;
    while start > 0 && !is_delim(bytes[start - 1]) {
        start -= 1;
    }
    let mut end = offset;
    while end < len && !is_delim(bytes[end]) {
        end += 1;
    }
    if start == end && end < len {
        end += 1;
    }
    (start, end)
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: usize = 64;
    const H: usize = 64;

    fn blank() -> Vec<u8> {
        vec![0xFF; W * H * 2]
    }

    fn inked_rows(buf: &[u8]) -> Vec<usize> {
        (0..H)
            .filter(|y| {
                (0..W).any(|x| {
                    let off = (y * W + x) * 2;
                    u16::from_le_bytes([buf[off], buf[off + 1]]) != 0xFFFF
                })
            })
            .collect()
    }

    /// The mark used to be drawn at `line_top + line_height + 2` -- the row
    /// below -- so it appeared to underline the following line, and on the last
    /// row of a page it fell outside the text area entirely.
    #[test]
    fn underline_stays_inside_its_own_line_box() {
        let mut buf = blank();
        let (top, lh) = (10usize, 20usize);
        draw_underline(&mut buf, W, H, top, lh, 4, 40);
        let rows = inked_rows(&buf);
        assert!(!rows.is_empty(), "nothing was drawn");
        for y in &rows {
            assert!(
                *y >= top && *y < top + lh,
                "row {y} is outside the line box {top}..{}",
                top + lh
            );
        }
        assert!(
            rows[0] > top + lh / 2,
            "must sit below the baseline, not through the text"
        );
    }

    /// A single row of mid-grey is what an e-ink waveform dithers away to
    /// nothing. The mark is drawn in the text colour, at the text's weight.
    #[test]
    fn underline_is_ink_and_thick_enough_to_survive_a_waveform() {
        let mut buf = blank();
        draw_underline(&mut buf, W, H, 10, 20, 4, 40);
        assert!(inked_rows(&buf).len() >= 2, "too thin to render on e-ink");
        let off = (inked_rows(&buf)[0] * W + 5) * 2;
        assert_eq!(
            u16::from_le_bytes([buf[off], buf[off + 1]]),
            TEXT_RGB565,
            "must be drawn in ink, not grey"
        );
    }

    #[test]
    fn underline_clips_at_the_screen_edge() {
        let mut buf = blank();
        // Line box hanging off the bottom, and a run running off the right.
        draw_underline(&mut buf, W, H, H - 4, 20, W - 8, W + 100);
        for y in inked_rows(&buf) {
            assert!(y < H, "row {y} past the screen");
        }
    }
}

/// Magnify the content region of an already-composited page by 2x, centred on
/// `center`. A half-size crop is nearest-neighbour sampled back over the whole
/// content region -- a magnifying-glass crop of the painted page, not a reflow,
/// so pagination and TTS byte offsets are untouched. Nearest-neighbour (no
/// interpolation) keeps glyph edges crisp, which matters more on a 16-level
/// e-ink panel than smoothing.
pub fn apply_zoom(
    buf: &mut [Rgb565Pixel],
    w: usize,
    content_top: usize,
    content_h: usize,
    center: (usize, usize),
) {
    if w == 0 || content_h == 0 || buf.len() < (content_top + content_h) * w {
        return;
    }
    let (cx, cy) = center;
    // Crop is half the content width/height, centred on the tap, clamped so it
    // never reads outside the content region -- a tap near an edge shifts the
    // crop inward rather than reading garbage.
    let crop_w = w / 2;
    let crop_h = content_h / 2;
    if crop_w == 0 || crop_h == 0 {
        return;
    }
    let crop_x = cx.saturating_sub(crop_w / 2).min(w.saturating_sub(crop_w));
    let crop_y = cy
        .saturating_sub(crop_h / 2)
        .max(content_top)
        .min(content_top + content_h - crop_h);

    // Source and destination overlap (we write `buf` in place), so snapshot the
    // content region first.
    let base = content_top * w;
    let scratch: Vec<Rgb565Pixel> = buf[base..base + content_h * w].to_vec();
    let crop_y_rel = crop_y - content_top;
    for row_rel in 0..content_h {
        let dest_row = content_top + row_rel;
        let src_row_rel = crop_y_rel + row_rel / 2;
        let dest_base = dest_row * w;
        let src_base = src_row_rel * w;
        for x in 0..w {
            buf[dest_base + x] = scratch[src_base + crop_x + x / 2];
        }
    }
}

#[cfg(test)]
mod zoom_tests {
    use super::apply_zoom;
    use slint::platform::software_renderer::Rgb565Pixel;

    /// Encode each content pixel as `(row << 8) | col` so a magnified pixel can
    /// be traced back to the exact source it was sampled from.
    fn make_buf(w: usize, top: usize, ch: usize) -> Vec<Rgb565Pixel> {
        let mut buf = vec![Rgb565Pixel(0); w * (top + ch)];
        for r in top..(top + ch) {
            for c in 0..w {
                buf[r * w + c] = Rgb565Pixel(((r as u16) << 8) | (c as u16 & 0xff));
            }
        }
        buf
    }

    #[test]
    fn zoom_is_exact_2x_nearest_neighbour_centred() {
        let (w, top, ch) = (20, 10, 20);
        let mut buf = make_buf(w, top, ch);
        // Content region is rows 10..30. Tap the middle: crop is 10x10 at
        // crop_x=5, crop_y=15 (absolute rows).
        apply_zoom(&mut buf, w, top, ch, (10, 20));
        let (crop_x, crop_y) = (5, 15);
        for r in top..(top + ch) {
            for c in 0..w {
                let row_rel = r - top;
                let sr = crop_y + row_rel / 2;
                let sc = crop_x + c / 2;
                let expected = Rgb565Pixel(((sr as u16) << 8) | (sc as u16 & 0xff));
                assert_eq!(
                    buf[r * w + c],
                    expected,
                    "dest ({c},{r}) sampled wrong source"
                );
            }
        }
    }

    #[test]
    fn zoom_clamps_near_every_edge_without_panicking() {
        let (w, top, ch) = (20, 10, 20);
        for &center in &[(0usize, 10usize), (19, 10), (10, 29), (0, 29), (1000, 1000)] {
            let mut buf = make_buf(w, top, ch);
            apply_zoom(&mut buf, w, top, ch, center);
            // Every magnified pixel must still decode to an in-bounds source.
            for r in top..(top + ch) {
                for c in 0..w {
                    let v = buf[r * w + c].0;
                    let sr = (v >> 8) as usize;
                    let sc = (v & 0xff) as usize;
                    assert!(
                        sr >= top && sr < top + ch,
                        "center {center:?}: oob src row {sr}"
                    );
                    assert!(sc < w, "center {center:?}: oob src col {sc}");
                }
            }
        }
    }

    #[test]
    fn zoom_noops_on_degenerate_dimensions() {
        let mut buf = vec![Rgb565Pixel(7); 100];
        apply_zoom(&mut buf, 0, 0, 10, (0, 0));
        apply_zoom(&mut buf, 10, 0, 0, (0, 0));
        assert_eq!(
            buf[0],
            Rgb565Pixel(7),
            "zero-area calls must not touch the buffer"
        );
    }
}

#[cfg(test)]
mod word_boundary_tests {
    use super::word_boundary;

    #[test]
    fn word_boundary_finds_word() {
        let text = "hello world foo";
        assert_eq!(word_boundary(text, 2), (0, 5));
        assert_eq!(word_boundary(text, 7), (6, 11));
        assert_eq!(word_boundary(text, 13), (12, 15));
        assert_eq!(word_boundary(text, 5), (0, 5));
        assert_eq!(word_boundary(text, 15), (15, 15));
        assert_eq!(word_boundary(text, 0), (0, 5));
        let text2 = "hello";
        assert_eq!(word_boundary(text2, 0), (0, 5));
        assert_eq!(word_boundary(text2, 4), (0, 5));
    }
}
