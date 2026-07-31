// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use crate::data::word_index::{WordHit, MAX_SEARCH_RESULTS};
use crate::rendering::chapter_list::{
    paint_scrollbar, CH_LIST_BOTTOM_PAD, CH_LIST_TOP, CH_ROW_H, CH_ROW_PITCH, CH_ROW_X,
    SB_ROW_SHRINK,
};
use crate::rendering::common::rgb565_as_bytes;
use crate::rendering::draw::fill_rounded_rect;
use crate::rendering::text_render;
use crate::rendering::word_list::{INK, TAB_BORDER};

const WHITE: u16 = 0xFFFF;
const BORDER: u16 = 0x94B2;
const SNIPPET_CHARS: usize = 40;
const CH_LABEL_W: i32 = 60;

pub fn paint_search_results(
    buf: &mut [Rgb565Pixel],
    _word: &str,
    hits: &[WordHit],
    chapters: &[kobo_core::Chapter],
    scroll: i32,
    body_px: f32,
    total_hits: usize,
    selected: usize,
) {
    let w = crate::w();
    let h = crate::h();
    let list_bottom = h as i32 - CH_LIST_BOTTOM_PAD;
    for y in CH_LIST_TOP..list_bottom {
        let off = (y as usize) * w;
        buf[off..off + w].fill(Rgb565Pixel(0xFFFF));
    }
    let buf_bytes = rgb565_as_bytes(buf);
    let display_count = hits.len().min(MAX_SEARCH_RESULTS);
    for (i, hit) in hits.iter().take(display_count).enumerate() {
        let y = CH_LIST_TOP + (i as i32) * CH_ROW_PITCH - scroll;
        if y < CH_LIST_TOP || y + CH_ROW_H > list_bottom {
            continue;
        }
        let is_sel = selected != usize::MAX && i == selected;
        let (fill, border) = if is_sel {
            (INK, TAB_BORDER)
        } else {
            (WHITE, BORDER)
        };
        fill_rounded_rect(
            buf_bytes,
            w,
            h,
            CH_ROW_X as usize,
            y as usize,
            (w as i32 - 2 * CH_ROW_X - SB_ROW_SHRINK) as usize,
            CH_ROW_H as usize,
            fill,
            border,
            8,
        );
        let fg: u16 = if is_sel { WHITE } else { INK };
        let ch_label = if let Some(ch) = chapters.get(hit.chapter as usize) {
            crate::data::library::chapter_display_title(ch, hit.chapter as usize)
        } else {
            format!("Ch{}", hit.chapter + 1)
        };
        let ch_trunc =
            crate::rendering::draw::truncate_to_width(&ch_label, 16.0, CH_LABEL_W as usize);
        let ch_lh = text_render::line_height(16.0) as i32;
        let ch_y = (y + (CH_ROW_H - ch_lh) / 2).max(0) as usize;
        text_render::blit_rgb565_color(
            buf_bytes,
            w,
            &ch_trunc,
            16.0,
            (CH_ROW_X + 10) as usize,
            ch_y,
            fg,
            (CH_ROW_X + 10 + CH_LABEL_W) as usize,
            h,
        );
        let snippet = build_snippet(chapters, hit);
        let snippet_x = (CH_ROW_X + CH_LABEL_W + 10) as usize;
        let snippet_max = (w as i32 - CH_ROW_X - CH_LABEL_W - 30 - SB_ROW_SHRINK) as usize;
        let snippet_lh = text_render::line_height(body_px) as i32;
        let snip_y = (y + (CH_ROW_H - snippet_lh) / 2).max(0) as usize;
        text_render::blit_rgb565_color(
            buf_bytes,
            w,
            &snippet,
            body_px,
            snippet_x,
            snip_y,
            fg,
            snippet_x + snippet_max,
            h,
        );
    }
    if total_hits > MAX_SEARCH_RESULTS {
        let more_y = CH_LIST_TOP + (display_count as i32) * CH_ROW_PITCH - scroll;
        if more_y >= CH_LIST_TOP && more_y < list_bottom {
            let msg = format!("and {} more...", total_hits - MAX_SEARCH_RESULTS);
            let msg_y =
                (more_y + (CH_ROW_H - text_render::line_height(18.0) as i32) / 2).max(0) as usize;
            text_render::blit_rgb565(
                buf_bytes,
                w,
                &msg,
                18.0,
                CH_ROW_X as usize + 20,
                msg_y,
                w,
                h,
            );
        }
    }
    paint_scrollbar(buf_bytes, w, h, display_count, scroll, false);
}

fn build_snippet(chapters: &[kobo_core::Chapter], hit: &WordHit) -> String {
    let body = chapters
        .get(hit.chapter as usize)
        .map(|c| c.body.as_str())
        .unwrap_or("");
    if body.is_empty() {
        return String::new();
    }
    let start = hit.byte_offset as usize;
    let start = if let Some(s) = body.is_char_boundary(start).then_some(start) {
        s
    } else {
        body.floor_char_boundary(start)
    };
    if start >= body.len() {
        return String::new();
    }
    let tail = &body[start..];
    let end = tail
        .char_indices()
        .nth(SNIPPET_CHARS * 2)
        .map_or(body.len(), |(i, _)| start + i);
    let window = &body[start..end];
    let trimmed = window.trim();
    let safe_end = trimmed
        .char_indices()
        .nth(SNIPPET_CHARS)
        .map_or(trimmed.len(), |(i, _)| i);
    if safe_end < trimmed.len() {
        format!("...{}...", &trimmed[..safe_end])
    } else {
        trimmed.to_string()
    }
}

pub fn results_hit_test(tap_y: i32, scroll: i32, result_count: usize) -> Option<usize> {
    let h = crate::h() as i32;
    let list_bottom = h - CH_LIST_BOTTOM_PAD;
    if tap_y < CH_LIST_TOP || tap_y >= list_bottom {
        return None;
    }
    let i = (tap_y - CH_LIST_TOP + scroll) / CH_ROW_PITCH;
    if i >= 0 && (i as usize) < result_count {
        Some(i as usize)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_hit_test_first_row() {
        assert_eq!(results_hit_test(CH_LIST_TOP, 0, 5), Some(0));
    }

    #[test]
    fn results_hit_test_above_returns_none() {
        assert_eq!(results_hit_test(CH_LIST_TOP - 1, 0, 5), None);
    }

    #[test]
    fn results_hit_test_respects_scroll() {
        assert_eq!(results_hit_test(CH_LIST_TOP, CH_ROW_PITCH, 5), Some(1));
    }
}
