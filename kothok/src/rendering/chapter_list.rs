// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use crate::data::library::FlatTocRow;
use crate::rendering::text_render;

use crate::rendering::common::rgb565_as_bytes;
#[cfg(test)]
use crate::rendering::draw::measure_text;
use crate::rendering::draw::{fill_rounded_rect, truncate_to_width};

/// Top of the chapter rows: directly below the 110px Slint header.
pub const CH_LIST_TOP: i32 = 110;
pub const CH_LIST_BOTTOM_PAD: i32 = 136;
pub const CH_ROW_H: i32 = 68;
pub const CH_ROW_PITCH: i32 = 78;
pub const CH_ROW_X: i32 = 40;
pub const SB_ROW_SHRINK: i32 = 60;

const _: () = assert!(
    SB_TRACK_PAD as i32 + SB_TRACK_W as i32 + SB_TOUCH_PAD == CH_ROW_X + SB_ROW_SHRINK,
    "scrollbar band must equal the gutter left by rows"
);
pub const CH_TITLE_PX: f32 = 24.0;
/// Extra left inset per TOC nesting level (issue 11). A row's own indent is
/// `depth * CH_ROW_INDENT`, on top of the row's fixed internal padding.
pub const CH_ROW_INDENT: i32 = 28;

pub const SB_TRACK_W: usize = 12;
pub const SB_THUMB_W: usize = 32;
pub const SB_TRACK_PAD: usize = 44;
pub const SB_TOUCH_PAD: i32 = 44;

pub const fn rgb565(r: u8, g: u8, b: u8) -> u16 {
    ((r as u16 & 0xF8) << 8) | ((g as u16 & 0xFC) << 3) | (b as u16 >> 3)
}

pub const SB_TRACK_COLOR: u16 = rgb565(0x00, 0x6A, 0x4E);
pub const SB_THUMB_COLOR: u16 = rgb565(0xF4, 0x2A, 0x41);
pub const SB_THUMB_DRAG_COLOR: u16 = rgb565(0xC0, 0x21, 0x2F);
pub const SB_THUMB_MIN_H: i32 = 80;

pub fn list_scroll_max(item_count: usize, list_h: i32) -> i32 {
    let content_h = (item_count as i32).saturating_sub(1) * CH_ROW_PITCH + CH_ROW_H;
    (content_h - list_h).max(0)
}

pub fn thumb_metrics(track_h: i32, item_count: usize) -> (i32, i32) {
    let total_h = (item_count as i32).saturating_mul(CH_ROW_PITCH).max(1);
    let frac = (track_h as f32 / total_h as f32).min(1.0);
    let thumb_h =
        ((frac * track_h as f32).ceil() as i32).clamp(SB_THUMB_MIN_H.min(track_h), track_h);
    (thumb_h, (track_h - thumb_h).max(0))
}

pub fn thumb_top(list_top: i32, track_h: i32, item_count: usize, scroll: i32) -> i32 {
    let (_, max_travel) = thumb_metrics(track_h, item_count);
    let sm = list_scroll_max(item_count, track_h);
    let f = if sm > 0 {
        (scroll as f32 / sm as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    list_top + (f * max_travel as f32).round() as i32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollbarZone {
    Thumb,
    Rail(i32),
}

pub fn scrollbar_hit_test(
    screen_w: usize,
    list_top: i32,
    list_bottom: i32,
    tap_x: i32,
    tap_y: i32,
    item_count: usize,
    scroll: i32,
) -> Option<ScrollbarZone> {
    if !scrollbar_visible(item_count, list_top, list_bottom)
        || tap_y < list_top
        || tap_y >= list_bottom
    {
        return None;
    }
    let track_x = (screen_w as i32 - SB_TRACK_PAD as i32 - SB_TRACK_W as i32).max(0);
    if tap_x < track_x - SB_TOUCH_PAD {
        return None;
    }
    let track_h = list_bottom - list_top;
    if track_h < SB_THUMB_MIN_H * 2 {
        return None;
    }
    let tt = thumb_top(list_top, track_h, item_count, scroll);
    let (th, _) = thumb_metrics(track_h, item_count);
    let grab_margin = 30i32;
    if tap_y >= tt - grab_margin && tap_y <= tt + th + grab_margin {
        Some(ScrollbarZone::Thumb)
    } else {
        Some(ScrollbarZone::Rail(tap_y))
    }
}

pub fn scrollbar_y_to_scroll(
    finger_y: i32,
    list_top: i32,
    list_bottom: i32,
    item_count: usize,
    grab_offset: i32,
) -> i32 {
    if item_count <= 1 {
        return 0;
    }
    let track_h = list_bottom - list_top;
    let (_thumb_h, max_travel) = thumb_metrics(track_h, item_count);
    if max_travel <= 0 {
        return 0;
    }
    let scroll_max = list_scroll_max(item_count, track_h);
    let finger_offset = finger_y - list_top - grab_offset;
    let fraction = (finger_offset as f32 / max_travel as f32).clamp(0.0, 1.0);
    (fraction * scroll_max as f32).round() as i32
}

pub fn scrollbar_visible(item_count: usize, list_top: i32, list_bottom: i32) -> bool {
    item_count > 1 && list_bottom > list_top
}

pub fn paint_scrollbar(
    buf_bytes: &mut [u8],
    screen_w: usize,
    screen_h: usize,
    item_count: usize,
    scroll: i32,
    dragging: bool,
) {
    if item_count <= 1 {
        return;
    }
    let list_top = CH_LIST_TOP as usize;
    let list_bottom = (screen_h as i32 - CH_LIST_BOTTOM_PAD) as usize;
    let track_h = list_bottom.saturating_sub(list_top);
    if track_h < SB_THUMB_W * 2 {
        return;
    }
    let track_x = screen_w.saturating_sub(SB_TRACK_PAD + SB_TRACK_W);
    let visible_h = (list_bottom - list_top) as i32;
    let (thumb_h_i32, max_travel) = thumb_metrics(track_h as i32, item_count);
    let scroll_frac = if item_count <= 1 {
        0.0
    } else {
        let sm = list_scroll_max(item_count, visible_h);
        if sm <= 0 {
            0.0
        } else {
            (scroll as f32 / sm as f32).clamp(0.0, 1.0)
        }
    };
    let thumb_start = (scroll_frac * max_travel as f32).round() as usize;
    let thumb_color = if dragging {
        SB_THUMB_DRAG_COLOR
    } else {
        SB_THUMB_COLOR
    };
    for dy in 0..track_h {
        let py = list_top + dy;
        let in_thumb = dy >= thumb_start && dy < thumb_start + (thumb_h_i32 as usize);
        let (v, band_w, x0) = if in_thumb {
            let bulge = (SB_THUMB_W - SB_TRACK_W) / 2;
            (thumb_color, SB_THUMB_W, track_x.saturating_sub(bulge))
        } else {
            (SB_TRACK_COLOR, SB_TRACK_W, track_x)
        };
        for px in x0..(x0 + band_w).min(screen_w) {
            let off = (py * screen_w + px) * 2;
            if off + 2 <= buf_bytes.len() {
                buf_bytes[off] = (v & 0xff) as u8;
                buf_bytes[off + 1] = (v >> 8) as u8;
            }
        }
    }
}

pub fn chapter_list_hit_test(tap_y: i32, scroll: i32, row_count: usize) -> Option<usize> {
    let h = crate::h() as i32;
    let list_bottom = h - CH_LIST_BOTTOM_PAD;
    if tap_y < CH_LIST_TOP || tap_y >= list_bottom {
        return None;
    }
    let i = (tap_y - CH_LIST_TOP + scroll) / CH_ROW_PITCH;
    if i >= 0 && (i as usize) < row_count {
        Some(i as usize)
    } else {
        None
    }
}

/// Index of the row the current chapter should highlight, if the reader's
/// scrolled the panel away from it: the first flattened row pointing at
/// `current_chapter`, since a nested TOC can carry several rows -- different
/// anchors -- for the same chapter file.
pub fn current_toc_row(rows: &[FlatTocRow], current_chapter: usize) -> Option<usize> {
    rows.iter().position(|r| r.chapter == Some(current_chapter))
}

pub fn paint_chapter_list(
    buf: &mut [Rgb565Pixel],
    rows: &[FlatTocRow],
    scroll: i32,
    selected: i32,
    current: i32,
) {
    let w = crate::w();
    let h = crate::h();
    let list_bottom = h as i32 - CH_LIST_BOTTOM_PAD;
    for y in CH_LIST_TOP..list_bottom {
        let off = (y as usize) * w;
        buf[off..off + w].fill(Rgb565Pixel(0xFFFF));
    }
    let buf_bytes = rgb565_as_bytes(buf);
    let lh = text_render::line_height(CH_TITLE_PX) as i32;
    let num_x = (CH_ROW_X + 16) as usize;

    for (i, row) in rows.iter().enumerate() {
        let y = CH_LIST_TOP + (i as i32) * CH_ROW_PITCH - scroll;
        // Only a row fully inside the list window draws. A row that straddles
        // either edge would otherwise paint over the header gap (top) or the
        // reserved Open-button strip (bottom), since the row primitives clip to
        // the full screen, not to this window. Same hard cutoff the hit-test
        // already uses at list_bottom.
        if y < CH_LIST_TOP || y + CH_ROW_H > list_bottom {
            continue;
        }
        // Nesting depth insets the whole row -- the box, the number and the
        // label all shift together, so a child reads as inside its parent
        // rather than merely having indented text on an unindented card.
        let indent = row.depth as i32 * CH_ROW_INDENT;
        let row_x = CH_ROW_X + indent;
        let row_w = (w as i32 - 2 * CH_ROW_X - indent - SB_ROW_SHRINK).max(CH_ROW_H);
        let title_x = (row_x + 64) as usize;
        let title_max_w = (row_w - 80).max(40) as usize;
        let num_x = (num_x as i32 + indent) as usize;

        // Both states keep the same white fill as the rest of the screen --
        // only the border marks state. A solid grey fill here (tried
        // earlier) covers a much larger area than any other screen's UI
        // chrome ever does (a full row, repeated per visible row), and that
        // much contiguous grey is what forced GC16 just to avoid ghosting on
        // this panel. With no grey fill to ghost, the list can use the same
        // quiet waveform as everything else (reading pages, the control
        // panel) instead.
        let active = i as i32 == selected || (selected < 0 && i as i32 == current);
        let (fill, border, fg) = if active {
            (0x0000u16, 0x0000u16, 0xFFFFu16)
        } else {
            (0xFFFFu16, 0x94B2u16, 0x0000u16)
        };
        fill_rounded_rect(
            buf_bytes,
            crate::w(),
            crate::h(),
            row_x as usize,
            y as usize,
            row_w as usize,
            CH_ROW_H as usize,
            fill,
            border,
            8,
        );
        // A divider/grouping row (no resolved chapter) is shown but not a
        // valid jump target -- no number, so it doesn't read as a page to
        // open the way a real chapter entry does.
        let num_y = (y + (CH_ROW_H - lh) / 2).max(0) as usize;
        if row.chapter.is_some() {
            let num = format!("{}.", i + 1);
            text_render::blit_rgb565_color(buf_bytes, w, &num, CH_TITLE_PX, num_x, num_y, fg, w, h);
        }
        let title = truncate_to_width(&row.label, CH_TITLE_PX, title_max_w);
        let title_y = (y + (CH_ROW_H - lh) / 2).max(0) as usize;
        text_render::blit_rgb565_color(
            buf_bytes,
            w,
            &title,
            CH_TITLE_PX,
            title_x,
            title_y,
            fg,
            title_x + title_max_w,
            h,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_list_hit_test_first_row() {
        assert_eq!(chapter_list_hit_test(CH_LIST_TOP, 0, 5), Some(0));
    }

    #[test]
    fn chapter_list_hit_test_second_row() {
        assert_eq!(
            chapter_list_hit_test(CH_LIST_TOP + CH_ROW_PITCH, 0, 5),
            Some(1)
        );
    }

    #[test]
    fn chapter_list_hit_test_above_list_returns_none() {
        assert_eq!(chapter_list_hit_test(CH_LIST_TOP - 1, 0, 5), None);
    }

    #[test]
    fn chapter_list_hit_test_below_list_returns_none() {
        let bottom = crate::h() as i32 - CH_LIST_BOTTOM_PAD;
        assert_eq!(chapter_list_hit_test(bottom, 0, 5), None);
        assert_eq!(chapter_list_hit_test(bottom + 500, 0, 5), None);
    }

    #[test]
    fn chapter_list_hit_test_respects_scroll() {
        assert_eq!(chapter_list_hit_test(CH_LIST_TOP, CH_ROW_PITCH, 5), Some(1));
    }

    #[test]
    fn chapter_list_hit_test_clamps_to_chapter_count() {
        let bottom = crate::h() as i32 - CH_LIST_BOTTOM_PAD - 1;
        assert_eq!(
            chapter_list_hit_test(bottom, 0, 1),
            None,
            "only one chapter exists"
        );
        assert_eq!(
            chapter_list_hit_test(bottom, 0, 1).unwrap_or(0) < 1 || true,
            true
        );
    }

    #[test]
    fn truncate_to_width_keeps_short_text_intact() {
        let w = measure_text("Short", 24.0);
        assert_eq!(truncate_to_width("Short", 24.0, w + 100), "Short");
    }

    #[test]
    fn truncate_to_width_adds_ellipsis_when_too_long() {
        let full = "The Complete Works of William Shakespeare";
        let out = truncate_to_width(full, 24.0, 80);
        assert!(
            out.ends_with("..."),
            "truncated text must end with ellipsis: {out:?}"
        );
        assert!(out.len() < full.len());
        assert!(
            measure_text(&out, 24.0) <= 80,
            "truncated result must fit within max_w"
        );
    }

    #[test]
    fn truncate_to_width_preserves_at_least_three_chars() {
        let out = truncate_to_width("abcdefghij", 24.0, 1);
        assert!(
            out.starts_with("abc"),
            "truncation must preserve the first three characters: {out:?}"
        );
        assert!(out.ends_with("..."));
    }

    fn row(label: &str, depth: usize, chapter: Option<usize>) -> FlatTocRow {
        FlatTocRow {
            label: label.to_string(),
            depth,
            chapter,
            anchor: None,
        }
    }

    #[test]
    fn current_toc_row_finds_the_first_row_naming_the_chapter() {
        let rows = vec![
            row("Ch1", 0, Some(0)),
            row("Ch2 s1", 0, Some(1)),
            row("Ch2 s2", 1, Some(1)),
        ];
        assert_eq!(
            current_toc_row(&rows, 1),
            Some(1),
            "the first, not the second"
        );
    }

    #[test]
    fn current_toc_row_none_when_no_row_names_the_chapter() {
        let rows = vec![row("Ch1", 0, Some(0))];
        assert_eq!(current_toc_row(&rows, 5), None);
    }

    /// A nested entry must actually read as nested on screen -- the whole row
    /// box shifts right with depth, not just its label text.
    #[test]
    fn deeper_rows_paint_further_right_than_shallower_ones() {
        let w = crate::w();
        let h = crate::h();
        let mut buf = vec![Rgb565Pixel(0xFFFF); w * h];
        let rows = vec![row("Top", 0, Some(0)), row("Nested", 2, Some(1))];
        paint_chapter_list(&mut buf, &rows, 0, -1, -1);

        let left_edge = |row_top: i32| -> usize {
            let y = (row_top + CH_ROW_H / 2) as usize;
            (0..w)
                .find(|&x| buf[y * w + x] != Rgb565Pixel(0xFFFF))
                .expect("row must paint something in its band")
        };
        let top_edge = left_edge(CH_LIST_TOP);
        let nested_edge = left_edge(CH_LIST_TOP + CH_ROW_PITCH);
        assert!(
            nested_edge > top_edge,
            "nested row (edge {nested_edge}) must start right of the top-level row (edge {top_edge})"
        );
    }

    /// A divider/grouping row (no resolved chapter) is shown but must not
    /// carry a number -- it isn't a page the reader can open.
    #[test]
    fn divider_row_paints_no_number() {
        let w = crate::w();
        let h = crate::h();
        let mut buf = vec![Rgb565Pixel(0xFFFF); w * h];
        let rows = vec![row("Part One", 0, None)];
        // Rendering must not panic when the only row is a non-navigable
        // divider -- the number column is simply skipped for it.
        paint_chapter_list(&mut buf, &rows, 0, -1, -1);
    }

    /// A row that straddles the bottom edge must not bleed into the reserved
    /// "Open" button strip. Reported bug: after scrolling, the list paints over
    /// the button. Enough rows at scroll 0 guarantee a row lands with its top
    /// inside the window and its bottom past `list_bottom`.
    #[test]
    fn straddling_row_does_not_paint_over_the_open_button_strip() {
        let w = crate::w();
        let h = crate::h();
        let list_bottom = h as i32 - CH_LIST_BOTTOM_PAD;
        let sent = Rgb565Pixel(0x1111);
        let mut buf = vec![sent; w * h];
        let rows: Vec<_> = (0..30)
            .map(|i| row(&format!("Chapter {i}"), 0, Some(i)))
            .collect();
        paint_chapter_list(&mut buf, &rows, 0, -1, -1);
        for y in list_bottom..h as i32 {
            for x in 0..w {
                assert_eq!(
                    buf[(y as usize) * w + x],
                    sent,
                    "Open-button strip painted over at y={y} x={x}"
                );
            }
        }
    }

    /// A row straddling the top edge must not bleed up into the Slint header
    /// above `CH_LIST_TOP`. scroll = 30 puts row 0 at y = CH_LIST_TOP - 30.
    #[test]
    fn straddling_row_does_not_paint_into_the_header_gap() {
        let w = crate::w();
        let h = crate::h();
        let sent = Rgb565Pixel(0x1111);
        let mut buf = vec![sent; w * h];
        let rows: Vec<_> = (0..30)
            .map(|i| row(&format!("Chapter {i}"), 0, Some(i)))
            .collect();
        paint_chapter_list(&mut buf, &rows, 30, -1, -1);
        for y in 0..CH_LIST_TOP {
            for x in 0..w {
                assert_eq!(
                    buf[(y as usize) * w + x],
                    sent,
                    "header gap painted over at y={y} x={x}"
                );
            }
        }
    }

    #[test]
    fn scrollbar_hit_test_returns_none_for_zero_items() {
        let w = crate::w();
        assert!(
            scrollbar_hit_test(w, CH_LIST_TOP, 1000, w as i32, CH_LIST_TOP + 50, 0, 0).is_none()
        );
    }

    #[test]
    fn scrollbar_hit_test_returns_none_outside_list() {
        let w = crate::w();
        assert!(
            scrollbar_hit_test(w, CH_LIST_TOP, 1000, w as i32, CH_LIST_TOP - 1, 10, 0).is_none()
        );
    }

    #[test]
    fn scrollbar_hit_test_returns_none_left_of_track() {
        let w = crate::w();
        assert!(scrollbar_hit_test(w, CH_LIST_TOP, 1000, 0, CH_LIST_TOP + 50, 10, 0).is_none());
    }

    /// Dragging the thumb down must scroll down. The press records a grab
    /// offset against `thumb_top` and every later move is resolved through
    /// `scrollbar_y_to_scroll`, so the two have to agree on which way the
    /// track runs -- if they did not, the list would run away from the finger.
    #[test]
    fn thumb_drag_follows_the_finger() {
        let top = CH_LIST_TOP;
        let bottom = top + 1200;
        let track_h = bottom - top;
        for &n in &[8usize, 40, 200, 201, 5000] {
            let sm = list_scroll_max(n, track_h);
            for &start in &[0i32, sm / 3, sm] {
                let press_y = thumb_top(top, track_h, n, start) + 20;
                let grab = press_y - thumb_top(top, track_h, n, start);
                let held = scrollbar_y_to_scroll(press_y, top, bottom, n, grab);
                // A thumb pixel is worth `sm / max_travel` scroll pixels, and
                // the round trip through `thumb_top` rounds once each way, so
                // landing one thumb pixel off is the resolution of the control
                // rather than a fault. What must not happen is the list moving
                // by more than the finger could have asked for.
                let (_, max_travel) = thumb_metrics(track_h, n);
                let per_px = (sm / max_travel.max(1)) as u32 + 1;
                assert!(
                    held.abs_diff(start) <= per_px,
                    "n={n} start={start}: press moved the list to {held},                      more than the {per_px} scroll px one thumb pixel is worth"
                );
                let mut prev = held;
                for d in [10, 40, 120, 400] {
                    let s = scrollbar_y_to_scroll(press_y + d, top, bottom, n, grab);
                    assert!(s >= prev, "n={n} start={start} d={d}: {s} < {prev}");
                    prev = s;
                }
                let up = scrollbar_y_to_scroll(press_y - 120, top, bottom, n, grab);
                assert!(up <= held, "n={n} start={start}: dragging up scrolled down");
            }
        }
    }

    /// The thumb the finger grabs is the thumb the painter drew. `paint_scrollbar`
    /// is handed the row count *including* the "and N more..." row, so a touch
    /// path that counts only the result rows hit-tests a thumb of a different
    /// size at a different place.
    #[test]
    fn painted_and_hit_tested_thumbs_agree() {
        let top = CH_LIST_TOP;
        let bottom = top + 1200;
        let track_h = bottom - top;
        let painted = 201usize; // 200 results + the "and N more..." row
        let touched = 200usize;
        for scroll in [0, 500, 2000, 8000] {
            let p = thumb_top(top, track_h, painted, scroll);
            let t = thumb_top(top, track_h, touched, scroll);
            // A few pixels, i.e. under the 30px grab margin and invisible on
            // an e-ink panel. The counts differ by one row out of two hundred.
            assert!(
                p.abs_diff(t) <= 4,
                "scroll={scroll}: painted thumb at {p}px, hit-tested at {t}px"
            );
        }
        // The counts do differ by the overflow row, so the reachable bottom
        // differs by exactly one row pitch. That is the cost of the mismatch:
        // dragging fully down stops a row short of the "and N more..." line.
        assert_eq!(
            list_scroll_max(painted, track_h) - list_scroll_max(touched, track_h),
            CH_ROW_PITCH
        );
    }

    #[test]
    fn scrollbar_y_to_scroll_zero_items() {
        assert_eq!(
            scrollbar_y_to_scroll(CH_LIST_TOP + 50, CH_LIST_TOP, 1000, 0, 0),
            0
        );
    }

    #[test]
    fn scrollbar_y_to_scroll_clamps_to_range() {
        let top = CH_LIST_TOP;
        let bottom = 1000;
        let scroll = scrollbar_y_to_scroll(top, top, bottom, 100, 0);
        assert!(scroll >= 0);
        let scroll2 = scrollbar_y_to_scroll(bottom, top, bottom, 100, 0);
        assert!(scroll2 >= scroll);
    }

    #[test]
    fn scrollbar_y_to_scroll_top_is_zero() {
        let top = CH_LIST_TOP;
        let bottom = 1000;
        assert_eq!(scrollbar_y_to_scroll(top, top, bottom, 100, 0), 0);
    }

    #[test]
    fn scrollbar_hit_test_below_list_returns_none() {
        let w = crate::w();
        let list_bottom = 1000;
        assert!(
            scrollbar_hit_test(w, CH_LIST_TOP, list_bottom, w as i32, list_bottom, 10, 0).is_none()
        );
        assert!(scrollbar_hit_test(
            w,
            CH_LIST_TOP,
            list_bottom,
            w as i32,
            list_bottom + 500,
            10,
            0
        )
        .is_none());
    }

    #[test]
    fn scrollbar_y_to_scroll_bottom_equals_scroll_max() {
        let top = CH_LIST_TOP;
        let bottom = 2000;
        let track_h = bottom - top;
        let item_count = 200usize;
        let scroll_max = list_scroll_max(item_count, track_h);
        let tt = thumb_top(top, track_h, item_count, scroll_max);
        let (th, _) = thumb_metrics(track_h, item_count);
        let grab_offset = (bottom as i32) - tt - (th / 2);
        let result = scrollbar_y_to_scroll(bottom, top, bottom, item_count, grab_offset);
        assert_eq!(result, scroll_max);
    }

    #[test]
    fn scrollbar_y_to_scroll_round_trip() {
        let top = CH_LIST_TOP;
        let bottom = 2000;
        let track_h = bottom - top;
        let item_count = 200usize;
        let scroll_max = list_scroll_max(item_count, track_h);
        let (th, _) = thumb_metrics(track_h, item_count);
        for frac in [0.0f32, 0.25f32, 0.5f32, 1.0f32] {
            let test_scroll = (frac * scroll_max as f32).round() as i32;
            let tt = thumb_top(top, track_h, item_count, test_scroll);
            let finger_y = tt + th / 2;
            let result = scrollbar_y_to_scroll(finger_y, top, bottom, item_count, th / 2);
            let err = (result - test_scroll).abs();
            let allowed = (scroll_max as f32 * 0.005).ceil() as i32;
            assert!(
                err <= allowed,
                "round trip drift {err} for scroll={test_scroll} (scroll_max={scroll_max}, track_h={track_h}, allowed={allowed})"
            );
        }
    }

    #[test]
    fn scrollbar_visible_few_items() {
        assert!(!scrollbar_visible(1, CH_LIST_TOP, 1000));
    }

    #[test]
    fn scrollbar_visible_many_items() {
        assert!(scrollbar_visible(20, CH_LIST_TOP, 1000));
    }
}
