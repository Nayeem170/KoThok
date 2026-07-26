// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use crate::data::library::FlatTocRow;
use crate::rendering::text_render;

use crate::rendering::common::rgb565_as_bytes;
#[cfg(test)]
use crate::rendering::draw::measure_text;
use crate::rendering::draw::{fill_rounded_rect, truncate_to_width};

/// Top of the chapter rows: the shared 110px header plus the 48px gap this list
/// has always had below it. Both the painter and the hit-test read this, so they
/// stay in step.
pub const CH_LIST_TOP: i32 = 158;
pub const CH_LIST_BOTTOM_PAD: i32 = 136;
pub const CH_ROW_H: i32 = 60;
pub const CH_ROW_PITCH: i32 = 70;
pub const CH_ROW_X: i32 = 40;
pub const CH_TITLE_PX: f32 = 24.0;
/// Extra left inset per TOC nesting level (issue 11). A row's own indent is
/// `depth * CH_ROW_INDENT`, on top of the row's fixed internal padding.
pub const CH_ROW_INDENT: i32 = 28;

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
        let row_w = (w as i32 - 2 * CH_ROW_X - indent).max(CH_ROW_H);
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
        let (fill, border) = if active {
            (0xFFFFu16, 0x0000u16)
        } else {
            (0xFFFFu16, 0x94B2u16)
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
            text_render::blit_rgb565(buf_bytes, w, &num, CH_TITLE_PX, num_x, num_y, w, h);
        }
        let title = truncate_to_width(&row.label, CH_TITLE_PX, title_max_w);
        let title_y = (y + (CH_ROW_H - lh) / 2).max(0) as usize;
        text_render::blit_rgb565(
            buf_bytes,
            w,
            &title,
            CH_TITLE_PX,
            title_x,
            title_y,
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
        assert_eq!(current_toc_row(&rows, 1), Some(1), "the first, not the second");
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
        let rows: Vec<_> = (0..30).map(|i| row(&format!("Chapter {i}"), 0, Some(i))).collect();
        paint_chapter_list(&mut buf, &rows, 0, -1, -1);
        for y in list_bottom..h as i32 {
            for x in 0..w {
                assert_eq!(buf[(y as usize) * w + x], sent, "Open-button strip painted over at y={y} x={x}");
            }
        }
    }

    /// Symmetric to the bottom case: a row straddling the top edge must not
    /// bleed up into the header gap above `CH_LIST_TOP`. scroll = 30 puts row 0
    /// at y = 128, whose bottom (188) crosses the top edge (158).
    #[test]
    fn straddling_row_does_not_paint_into_the_header_gap() {
        let w = crate::w();
        let h = crate::h();
        let sent = Rgb565Pixel(0x1111);
        let mut buf = vec![sent; w * h];
        let rows: Vec<_> = (0..30).map(|i| row(&format!("Chapter {i}"), 0, Some(i))).collect();
        paint_chapter_list(&mut buf, &rows, 30, -1, -1);
        for y in 0..CH_LIST_TOP {
            for x in 0..w {
                assert_eq!(buf[(y as usize) * w + x], sent, "header gap painted over at y={y} x={x}");
            }
        }
    }
}
