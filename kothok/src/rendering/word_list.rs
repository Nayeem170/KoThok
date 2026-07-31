// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use crate::rendering::chapter_list::{
    paint_scrollbar, CH_LIST_BOTTOM_PAD, CH_LIST_TOP, CH_ROW_H, CH_ROW_PITCH, CH_ROW_X,
    SB_ROW_SHRINK,
};
use crate::rendering::common::rgb565_as_bytes;
use crate::rendering::draw::{fill_rounded_rect, truncate_to_width};
use crate::rendering::text_render;

const WHITE: u16 = 0xFFFF;
const BORDER: u16 = 0x94B2;
/// Tabs speak the library picker's filter-pill language (`picker/paint.rs`):
/// same border ink, fully rounded ends, and the active one inverted to a solid
/// black fill with a white label. The old white-on-white pill with a 0x94B2
/// hairline was theme-correct for a *row* (see `chapter_list.rs`) but reads as
/// absent at this size on Kaleido, and a tab bar has to announce which tab is
/// selected the way the picker's pills already do.
pub const TAB_BORDER: u16 = 0x2104;
pub const INK: u16 = 0x0000;

pub fn paint_word_list(
    buf: &mut [Rgb565Pixel],
    words: &[String],
    scroll: i32,
    body_px: f32,
    selected_word: usize,
    dragging: bool,
) {
    let w = crate::w();
    let h = crate::h();
    let list_bottom = h as i32 - CH_LIST_BOTTOM_PAD;
    for y in CH_LIST_TOP..list_bottom {
        let off = (y as usize) * w;
        buf[off..off + w].fill(Rgb565Pixel(0xFFFF));
    }
    if words.is_empty() {
        paint_empty_message(buf, w, h, "No searchable text");
        return;
    }
    let buf_bytes = rgb565_as_bytes(buf);
    let lh = text_render::line_height(body_px) as i32;
    let row_w = (w as i32 - 2 * CH_ROW_X - SB_ROW_SHRINK) as usize;
    let title_max_w = row_w;
    for (i, word) in words.iter().enumerate() {
        let y = CH_LIST_TOP + (i as i32) * CH_ROW_PITCH - scroll;
        if y < CH_LIST_TOP || y + CH_ROW_H > list_bottom {
            continue;
        }
        let selected = selected_word != usize::MAX && i == selected_word;
        let (fill, border) = if selected {
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
            row_w,
            CH_ROW_H as usize,
            fill,
            border,
            8,
        );
        let fg = if selected { WHITE } else { INK };
        let truncated = truncate_to_width(word, body_px, title_max_w);
        let text_y = (y + (CH_ROW_H - lh) / 2).max(0) as usize;
        text_render::blit_rgb565_color(
            buf_bytes,
            w,
            &truncated,
            body_px,
            (CH_ROW_X + 20) as usize,
            text_y,
            fg,
            w,
            h,
        );
    }
    paint_scrollbar(buf_bytes, w, h, words.len(), scroll, dragging);
}

pub fn word_list_hit_test(tap_y: i32, scroll: i32, word_count: usize) -> Option<usize> {
    let h = crate::h() as i32;
    let list_bottom = h - CH_LIST_BOTTOM_PAD;
    if tap_y < CH_LIST_TOP || tap_y >= list_bottom {
        return None;
    }
    let i = (tap_y - CH_LIST_TOP + scroll) / CH_ROW_PITCH;
    if i >= 0 && (i as usize) < word_count {
        Some(i as usize)
    } else {
        None
    }
}

fn paint_empty_message(buf: &mut [Rgb565Pixel], w: usize, h: usize, msg: &str) {
    let buf_bytes = rgb565_as_bytes(buf);
    let px = 28.0;
    let lh = text_render::line_height(px) as i32;
    let tw = crate::rendering::draw::measure_text(msg, px);
    let x = (w - tw) / 2;
    let y = (CH_LIST_TOP + (h as i32 - CH_LIST_TOP - CH_LIST_BOTTOM_PAD - lh) / 2).max(0) as usize;
    text_render::blit_rgb565(buf_bytes, w, msg, px, x, y, x + tw, h);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_list_hit_test_first_row() {
        assert_eq!(word_list_hit_test(CH_LIST_TOP, 0, 5), Some(0));
    }

    #[test]
    fn word_list_hit_test_above_returns_none() {
        assert_eq!(word_list_hit_test(CH_LIST_TOP - 1, 0, 5), None);
    }

    #[test]
    fn word_list_hit_test_below_returns_none() {
        let bottom = crate::h() as i32 - CH_LIST_BOTTOM_PAD;
        assert_eq!(word_list_hit_test(bottom, 0, 5), None);
    }

    #[test]
    fn word_list_hit_test_respects_scroll() {
        assert_eq!(word_list_hit_test(CH_LIST_TOP, CH_ROW_PITCH, 5), Some(1));
    }
}
