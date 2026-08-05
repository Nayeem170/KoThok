// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use crate::data::mark::{Mark, MarkKind};
use crate::rendering::chapter_list::{
    paint_scrollbar, CH_LIST_BOTTOM_PAD, CH_LIST_TOP, CH_ROW_H, CH_ROW_PITCH, CH_ROW_X,
    SB_ROW_SHRINK,
};
use crate::rendering::common::rgb565_as_bytes;
use crate::rendering::draw::{fill_rounded_rect, truncate_to_width};
use crate::rendering::text_render;
use crate::rendering::word_list::{INK, TAB_BORDER};

const WHITE: u16 = 0xFFFF;
const BORDER: u16 = 0x94B2;
const MUTED: u16 = 0x3186;
const KIND_BAR_PX: usize = 3;
const DELETE_LABEL: &str = "Delete";
const DELETE_RIGHT_MARGIN: usize = 10;

pub fn paint_marks_list(
    buf: &mut [Rgb565Pixel],
    marks: &[Mark],
    marks_scroll: i32,
    body_px: f32,
    armed_mark_idx: usize,
    dragging: bool,
) {
    let w = crate::w();
    let h = crate::h();
    let list_bottom = h as i32 - CH_LIST_BOTTOM_PAD;
    for y in CH_LIST_TOP..list_bottom {
        let off = (y as usize) * w;
        buf[off..off + w].fill(Rgb565Pixel(0xFFFF));
    }
    let buf_bytes = rgb565_as_bytes(buf);
    if marks.is_empty() {
        paint_empty_message(buf, w, h, "No bookmarks or highlights yet");
        return;
    }
    let lh = text_render::line_height(body_px) as i32;
    let row_right = marks_row_right(w);
    let row_w = row_right - CH_ROW_X as usize;
    for (i, m) in marks.iter().enumerate() {
        let y = CH_LIST_TOP + (i as i32) * CH_ROW_PITCH - marks_scroll;
        if !row_visible(y, list_bottom) {
            continue;
        }
        let is_sel = armed_mark_idx != usize::MAX && i == armed_mark_idx;
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
            row_w,
            CH_ROW_H as usize,
            fill,
            border,
            8,
        );
        let ch_fg: u16 = if is_sel { WHITE } else { MUTED };
        let is_armed = armed_mark_idx != usize::MAX && i == armed_mark_idx;
        let excerpt_start = CH_ROW_X as usize + KIND_BAR_PX + 4;
        let excerpt_max_w = if is_armed {
            let (del_x, _) = delete_label_rect(w, body_px);
            del_x.saturating_sub(excerpt_start + 4)
        } else {
            row_right.saturating_sub(excerpt_start)
        };
        let truncated = truncate_to_width(&m.excerpt, body_px, excerpt_max_w);
        let text_y = (y + (CH_ROW_H - lh) / 2).max(0) as usize;
        text_render::blit_rgb565_color(
            buf_bytes,
            w,
            &truncated,
            body_px,
            excerpt_start,
            text_y,
            ch_fg,
            w,
            h,
        );
        let ch_label = format!("Ch {} - p {}", m.chapter + 1, m.page_hint + 1);
        let ch_max_w = w - CH_ROW_X as usize - 20 - SB_ROW_SHRINK as usize;
        let ch_trunc = crate::rendering::draw::truncate_to_width(&ch_label, 18.0, ch_max_w);
        text_render::blit_rgb565_color(
            buf_bytes,
            w,
            &ch_trunc,
            18.0,
            excerpt_start,
            text_y + lh as usize,
            ch_fg,
            w,
            h,
        );
        if is_armed {
            let (del_x, _) = delete_label_rect(w, body_px);
            text_render::blit_rgb565_color(
                buf_bytes,
                w,
                DELETE_LABEL,
                body_px,
                del_x,
                text_y,
                WHITE,
                w,
                h,
            );
        }
        paint_kind_marker(
            buf_bytes,
            w,
            h,
            m,
            y as usize,
            CH_ROW_H as usize,
            CH_ROW_X as usize,
            is_sel,
        );
    }
    paint_scrollbar(buf_bytes, w, h, marks.len(), marks_scroll, dragging);
}

fn paint_kind_marker(
    buf_bytes: &mut [u8],
    w: usize,
    h: usize,
    m: &Mark,
    row_y: usize,
    row_h: usize,
    row_x: usize,
    is_sel: bool,
) {
    if m.kind != MarkKind::Highlight {
        return;
    }
    let bar_color: u16 = if is_sel { WHITE } else { INK };
    let bar_bytes = bar_color.to_le_bytes();
    let inset = 4;
    for ry in 0..row_h {
        let y = row_y + ry;
        if y >= h {
            break;
        }
        for dx in 0..KIND_BAR_PX {
            let px = row_x + inset + dx;
            if px < w {
                let idx = (y * w + px) * 2;
                if idx + 1 < buf_bytes.len() {
                    buf_bytes[idx] = bar_bytes[0];
                    buf_bytes[idx + 1] = bar_bytes[1];
                }
            }
        }
    }
}

fn paint_empty_message(buf: &mut [Rgb565Pixel], w: usize, h: usize, msg: &str) {
    let lh = 24.0;
    let text_w = crate::rendering::draw::measure_text(msg, lh);
    let x = (w.saturating_sub(text_w)) / 2;
    let y = ((CH_LIST_TOP as usize + (h as i32 - CH_LIST_BOTTOM_PAD) as usize) / 2)
        .max(CH_LIST_TOP as usize);
    let buf_bytes = rgb565_as_bytes(buf);
    text_render::blit_rgb565_color(buf_bytes, w, msg, lh, x, y, INK, w, h);
}

/// The row fill's right edge (exclusive). Shared by paint (fill width and the
/// unarmed excerpt bound) and the geometry tests. delete_label_rect keeps its
/// own copy of this reference; the containment test relates the two and so
/// catches a label that drifts off the fill (e.g. a screen-edge reference).
fn marks_row_right(width: usize) -> usize {
    width.saturating_sub(CH_ROW_X as usize + SB_ROW_SHRINK as usize)
}

/// The painter's row-visibility predicate: a row is drawn only when it fits
/// entirely within [CH_LIST_TOP, list_bottom]. Shared by paint and both hit
/// tests, so a tap can only resolve to a row that is actually drawn.
fn row_visible(row_y: i32, list_bottom: i32) -> bool {
    row_y >= CH_LIST_TOP && row_y + CH_ROW_H <= list_bottom
}

/// Right-aligned `Delete` label rect within a marks row: `(x, width)`, derived
/// from the row's right edge so the label sits inside the INK fill. The single
/// source for the label -- paint and hit-test both call this, so the tap target
/// cannot drift from what is drawn.
fn delete_label_rect(width: usize, body_px: f32) -> (usize, usize) {
    let del_w = crate::rendering::draw::measure_text(DELETE_LABEL, body_px);
    // Deliberately NOT marks_row_right: this second, independent derivation of
    // the row's right edge is what keeps delete_label_stays_inside_the_row_fill
    // non-vacuous -- the test asserts the label agrees with the paint's fill
    // geometry, which only bites if the two are computed separately. Do not DRY
    // this into marks_row_right without also changing the test's anchor.
    let row_right = width.saturating_sub(CH_ROW_X as usize + SB_ROW_SHRINK as usize);
    let x = row_right.saturating_sub(DELETE_RIGHT_MARGIN + del_w);
    (x, del_w)
}

pub fn marks_list_hit_test(
    tap_y: i32,
    scroll: i32,
    count: usize,
    list_bottom: i32,
) -> Option<usize> {
    if tap_y < CH_LIST_TOP || tap_y >= list_bottom {
        return None;
    }
    let i = (tap_y - CH_LIST_TOP + scroll) / CH_ROW_PITCH;
    if i < 0 || (i as usize) >= count {
        return None;
    }
    let row_y = CH_LIST_TOP + i * CH_ROW_PITCH - scroll;
    if row_visible(row_y, list_bottom) {
        Some(i as usize)
    } else {
        None
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum MarksReleaseAction {
    FinalizeArm,
    ConfirmDelete(usize),
    Dismiss,
    Navigate(usize),
}

pub fn marks_delete_label_hit_test(
    width: usize,
    body_px: f32,
    tap_x: i32,
    tap_y: i32,
    scroll: i32,
    armed_idx: usize,
    count: usize,
    list_bottom: i32,
) -> bool {
    if armed_idx >= count {
        return false;
    }
    let row_y = CH_LIST_TOP + (armed_idx as i32) * CH_ROW_PITCH - scroll;
    if tap_y < row_y || tap_y >= row_y + CH_ROW_H {
        return false;
    }
    if !row_visible(row_y, list_bottom) {
        return false;
    }
    let (del_x, del_w) = delete_label_rect(width, body_px);
    tap_x >= del_x as i32 && tap_x < (del_x + del_w) as i32
}

pub fn decide_marks_release(
    width: usize,
    list_bottom: i32,
    body_px: f32,
    armed_idx: usize,
    armed_this_press: bool,
    tap_x: i32,
    tap_y: i32,
    scroll: i32,
    count: usize,
) -> MarksReleaseAction {
    if armed_idx != usize::MAX {
        if armed_this_press {
            MarksReleaseAction::FinalizeArm
        } else if marks_delete_label_hit_test(
            width,
            body_px,
            tap_x,
            tap_y,
            scroll,
            armed_idx,
            count,
            list_bottom,
        ) {
            MarksReleaseAction::ConfirmDelete(armed_idx)
        } else {
            MarksReleaseAction::Dismiss
        }
    } else if let Some(i) = marks_list_hit_test(tap_y, scroll, count, list_bottom) {
        MarksReleaseAction::Navigate(i)
    } else {
        MarksReleaseAction::Dismiss
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLEET_W: &[usize] = &[600, 758, 1072, 1080, 1264, 1404, 1440];
    const FONTS: &[f32] = &[14.0, 18.0, 22.0, 28.0, 36.0, 44.0];
    const LIST_BOTTOM: i32 = 1500;

    #[test]
    fn delete_label_stays_inside_the_row_fill() {
        // Anchor is the paint's row-fill right edge (marks_row_right), not a
        // transcription; delete_label_rect keeps an independent reference, so a
        // label that drifts off the fill fails here.
        for &w in FLEET_W {
            for &px in FONTS {
                let (x, lw) = delete_label_rect(w, px);
                let right = x + lw;
                assert!(
                    right <= marks_row_right(w),
                    "Delete overhangs row on w={} px={}: right={} > row_right={}",
                    w,
                    px,
                    right,
                    marks_row_right(w)
                );
                assert!(x >= CH_ROW_X as usize, "label left of row on w={}", w);
            }
        }
    }

    #[test]
    fn hit_tests_reject_rows_the_painter_skips() {
        // list_bottom=320: row 2 top = 110 + 2*78 = 266, bottom = 334 > 320, so
        // the painter skips it. A tap on row 2 must not resolve -- there is no
        // drawn row (and no drawn Delete label) to act on.
        let bottom = 320;
        let (del_x, _) = delete_label_rect(1264, 22.0);
        let row2_mid = CH_LIST_TOP + 2 * CH_ROW_PITCH + CH_ROW_H / 2;
        assert_eq!(marks_list_hit_test(row2_mid, 0, 5, bottom), None);
        assert!(!marks_delete_label_hit_test(
            1264,
            22.0,
            del_x as i32,
            row2_mid,
            0,
            2,
            5,
            bottom,
        ));
        // A fully-visible row still resolves.
        assert_eq!(marks_list_hit_test(CH_LIST_TOP + 10, 0, 5, bottom), Some(0));
    }

    #[test]
    fn gap_between_rows_resolves_to_preceding_row() {
        // PITCH (78) - ROW_H (68) = 10px inter-row gap. Integer division folds a
        // gap tap into the preceding row. This governs long-press arming too
        // (touch_dispatch), so a press on the gap arms the row above it.
        let gap_y = CH_LIST_TOP + CH_ROW_H + 5;
        assert_eq!(marks_list_hit_test(gap_y, 0, 5, LIST_BOTTOM), Some(0));
        let next_gap_y = CH_LIST_TOP + CH_ROW_PITCH + CH_ROW_H + 5;
        assert_eq!(marks_list_hit_test(next_gap_y, 0, 5, LIST_BOTTOM), Some(1));
    }

    #[test]
    fn hit_band_matches_drawn_label_span() {
        let w = 1264;
        let px = 22.0;
        let (del_x, del_w) = delete_label_rect(w, px);
        let y = CH_LIST_TOP + 10;
        assert!(marks_delete_label_hit_test(
            w,
            px,
            del_x as i32,
            y,
            0,
            0,
            3,
            LIST_BOTTOM
        ));
        assert!(marks_delete_label_hit_test(
            w,
            px,
            (del_x + del_w - 1) as i32,
            y,
            0,
            0,
            3,
            LIST_BOTTOM,
        ));
        assert!(!marks_delete_label_hit_test(
            w,
            px,
            del_x as i32 - 1,
            y,
            0,
            0,
            3,
            LIST_BOTTOM
        ));
        assert!(!marks_delete_label_hit_test(
            w,
            px,
            (del_x + del_w) as i32,
            y,
            0,
            0,
            3,
            LIST_BOTTOM,
        ));
    }

    #[test]
    fn delete_label_wrong_row_is_false() {
        let (del_x, _) = delete_label_rect(1264, 22.0);
        let y = CH_LIST_TOP + CH_ROW_PITCH + 10;
        assert!(!marks_delete_label_hit_test(
            1264,
            22.0,
            del_x as i32,
            y,
            0,
            0,
            3,
            LIST_BOTTOM
        ));
    }

    #[test]
    fn delete_label_armed_out_of_range_is_false() {
        let (del_x, _) = delete_label_rect(1264, 22.0);
        let y = CH_LIST_TOP + 10;
        assert!(!marks_delete_label_hit_test(
            1264,
            22.0,
            del_x as i32,
            y,
            0,
            5,
            3,
            LIST_BOTTOM
        ));
    }

    /// A tap on the drawn label centre. Used by the decision tests so their tap
    /// point tracks the real label position, not a hand-picked coordinate.
    fn label_tap() -> (i32, i32) {
        let (del_x, del_w) = delete_label_rect(1264, 22.0);
        ((del_x + del_w / 2) as i32, CH_LIST_TOP + 10)
    }

    #[test]
    fn arming_gesture_release_never_deletes() {
        let (x, y) = label_tap();
        assert_eq!(
            decide_marks_release(1264, LIST_BOTTOM, 22.0, 0, true, x, y, 0, 3),
            MarksReleaseAction::FinalizeArm
        );
    }

    #[test]
    fn guard_flag_flips_delete_to_finalize() {
        let (x, y) = label_tap();
        assert_eq!(
            decide_marks_release(1264, LIST_BOTTOM, 22.0, 0, false, x, y, 0, 3),
            MarksReleaseAction::ConfirmDelete(0)
        );
        assert_eq!(
            decide_marks_release(1264, LIST_BOTTOM, 22.0, 0, true, x, y, 0, 3),
            MarksReleaseAction::FinalizeArm
        );
    }

    #[test]
    fn separate_tap_off_label_dismisses() {
        let (_, y) = label_tap();
        assert_eq!(
            decide_marks_release(1264, LIST_BOTTOM, 22.0, 0, false, 200, y, 0, 3),
            MarksReleaseAction::Dismiss
        );
    }

    #[test]
    fn unarmed_tap_navigates() {
        let y = CH_LIST_TOP + 10;
        assert_eq!(
            decide_marks_release(1264, LIST_BOTTOM, 22.0, usize::MAX, false, 200, y, 0, 3),
            MarksReleaseAction::Navigate(0)
        );
    }
}
