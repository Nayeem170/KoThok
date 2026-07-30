// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use crate::rendering::chapter_list::{
    paint_scrollbar, CH_LIST_BOTTOM_PAD, CH_LIST_TOP, CH_ROW_H, CH_ROW_PITCH, CH_ROW_X, CH_TITLE_PX,
};
use crate::rendering::common::rgb565_as_bytes;
use crate::rendering::draw::{fill_rounded_rect, measure_text, truncate_to_width};
use crate::rendering::text_render;

pub const TAB_BAR_TOP: i32 = 110;
pub const TAB_BTN_H: i32 = 36;
pub const TAB_BTN_Y: i32 = TAB_BAR_TOP + (CH_LIST_TOP - TAB_BAR_TOP - TAB_BTN_H) / 2;
pub const TAB_BTN_PAD: i32 = 40;
pub const TAB_GAP: i32 = 8;
pub const TAB_FONT_PX: f32 = CH_TITLE_PX;
pub const CLOSE_BTN_PX: i32 = 36;

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

pub fn tab_btn_rects(w: usize) -> (TabBtn, TabBtn, CloseBtn) {
    let ch_w = measure_text("Chapters", TAB_FONT_PX) + 2 * TAB_BTN_PAD as usize;
    let ch = TabBtn {
        x: TAB_BTN_PAD as usize,
        y: TAB_BTN_Y as usize,
        w: ch_w,
        h: TAB_BTN_H as usize,
    };
    let w_w = measure_text("Words", TAB_FONT_PX) + 2 * TAB_BTN_PAD as usize;
    let wd = TabBtn {
        x: ch.x + ch.w + TAB_GAP as usize,
        y: TAB_BTN_Y as usize,
        w: w_w,
        h: TAB_BTN_H as usize,
    };
    let cl = CloseBtn {
        x: w - TAB_BTN_PAD as usize - CLOSE_BTN_PX as usize,
        y: TAB_BTN_Y as usize,
        s: CLOSE_BTN_PX as usize,
    };
    (ch, wd, cl)
}

pub struct TabBtn {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
}

pub struct CloseBtn {
    pub x: usize,
    pub y: usize,
    pub s: usize,
}

pub fn paint_tab_bar(
    buf: &mut [Rgb565Pixel],
    w: usize,
    h: usize,
    active_tab: crate::loop_state::ChapterTab,
) {
    for y in TAB_BAR_TOP..CH_LIST_TOP {
        let off = (y as usize) * w;
        buf[off..off + w].fill(Rgb565Pixel(0xFFFF));
    }
    let buf_bytes = rgb565_as_bytes(buf);
    let (ch_btn, wd_btn, cl_btn) = tab_btn_rects(w);
    let chapters_active = active_tab == crate::loop_state::ChapterTab::Chapters;
    paint_tab_button(buf_bytes, w, h, &ch_btn, "Chapters", chapters_active);
    paint_tab_button(buf_bytes, w, h, &wd_btn, "Words", !chapters_active);
    paint_close_button(buf_bytes, w, h, &cl_btn);
}

fn paint_tab_button(
    buf_bytes: &mut [u8],
    w: usize,
    h: usize,
    btn: &TabBtn,
    label: &str,
    active: bool,
) {
    let (fill, fg) = if active { (INK, WHITE) } else { (WHITE, INK) };
    fill_rounded_rect(
        buf_bytes,
        w,
        h,
        btn.x,
        btn.y,
        btn.w,
        btn.h,
        fill,
        TAB_BORDER,
        btn.h / 2,
    );
    let lh = text_render::line_height(TAB_FONT_PX) as i32;
    let tx = btn.x + (btn.w - measure_text(label, TAB_FONT_PX)) / 2;
    let ty = btn.y + (btn.h - lh as usize) / 2;
    text_render::blit_rgb565_color(buf_bytes, w, label, TAB_FONT_PX, tx, ty, fg, w, h);
}

fn paint_close_button(buf_bytes: &mut [u8], w: usize, h: usize, btn: &CloseBtn) {
    fill_rounded_rect(
        buf_bytes,
        w,
        h,
        btn.x,
        btn.y,
        btn.s,
        btn.s,
        WHITE,
        TAB_BORDER,
        btn.s / 2,
    );
    let cx = btn.x + btn.s / 2;
    let cy = btn.y + btn.s / 2;
    let arm = btn.s as i32 / 4;
    // Two-pixel arms: a single-pixel cross at this size is the same hairline
    // that made the whole bar disappear on the panel.
    for d in -arm..=arm {
        for t in 0..2i32 {
            let px = (cx as i32 + d + t).clamp(0, w as i32 - 1) as usize;
            let py_up = (cy as i32 - d).clamp(0, h as i32 - 1) as usize;
            let py_dn = (cy as i32 + d).clamp(0, h as i32 - 1) as usize;
            for off in [py_up * w + px, py_dn * w + px] {
                if off * 2 + 1 < buf_bytes.len() {
                    buf_bytes[off * 2] = (INK & 0xFF) as u8;
                    buf_bytes[off * 2 + 1] = (INK >> 8) as u8;
                }
            }
        }
    }
}

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
    let row_w = (w as i32 - 2 * CH_ROW_X) as usize;
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
    let tw = measure_text(msg, px);
    let x = (w - tw) / 2;
    let y = (CH_LIST_TOP + (h as i32 - CH_LIST_TOP - CH_LIST_BOTTOM_PAD - lh) / 2).max(0) as usize;
    text_render::blit_rgb565(buf_bytes, w, msg, px, x, y, x + tw, h);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dark_frac(buf: &[Rgb565Pixel], w: usize, btn: &TabBtn) -> f32 {
        let mut dark = 0usize;
        for y in btn.y..btn.y + btn.h {
            for x in btn.x..btn.x + btn.w {
                if ((buf[y * w + x].0 >> 11) & 0x1F) < 8 {
                    dark += 1;
                }
            }
        }
        dark as f32 / (btn.w * btn.h) as f32
    }

    /// The active tab is the picker's inverted pill: mostly ink, so the bar
    /// cannot read as an empty band the way a hairline outline did.
    #[test]
    fn active_tab_is_inked_and_inactive_is_not() {
        let (w, h) = (crate::w(), crate::h());
        let (ch, wd, _) = tab_btn_rects(w);

        let mut buf = vec![Rgb565Pixel(0xFFFF); w * h];
        paint_tab_bar(&mut buf, w, h, crate::loop_state::ChapterTab::Chapters);
        assert!(dark_frac(&buf, w, &ch) > 0.5, "chapters tab not inked");
        assert!(dark_frac(&buf, w, &wd) < 0.3, "words tab should stay light");

        let mut buf = vec![Rgb565Pixel(0xFFFF); w * h];
        paint_tab_bar(&mut buf, w, h, crate::loop_state::ChapterTab::Words);
        assert!(dark_frac(&buf, w, &wd) > 0.5, "words tab not inked");
        assert!(
            dark_frac(&buf, w, &ch) < 0.3,
            "chapters tab should stay light"
        );
    }

    #[test]
    fn tab_bar_fits_left_of_close_button() {
        let (ch, wd, cl) = tab_btn_rects(crate::w());
        assert_eq!(wd.x, ch.x + ch.w + TAB_GAP as usize);
        assert!(wd.x + wd.w < cl.x, "tabs overlap the close button");
    }

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
