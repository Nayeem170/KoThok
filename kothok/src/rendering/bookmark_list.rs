// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use kobo_core::Chapter;

use crate::rendering::chapter_list::{
    paint_scrollbar, CH_LIST_BOTTOM_PAD, CH_LIST_TOP, CH_ROW_H, CH_ROW_PITCH, CH_ROW_X,
    CH_TITLE_PX, SB_ROW_SHRINK,
};
use crate::rendering::common::rgb565_as_bytes;
use crate::rendering::draw::{fill_rounded_rect, truncate_to_width};
use crate::rendering::text_render;

const WHITE: u16 = 0xFFFF;
const BORDER: u16 = 0x94B2;
const INK: u16 = 0x0000;

/// One overlay row: the bookmark's index into `st.bookmarks` plus the ready
/// label. Rows are pre-sorted by reading position so painting and hit tests
/// share one order.
pub struct BookmarkRow {
    pub orig: usize,
    pub label: String,
}

/// Bookmark indices in reading order (chapter, then offset), stable for
/// equal keys. Shared by row painting and touch hit-testing so a press and
/// its release always agree on which bookmark a row is.
pub fn sorted_orig(bms: &[crate::Bookmark]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..bms.len()).collect();
    order.sort_by(|&a, &b| {
        bms[a]
            .chapter
            .cmp(&bms[b].chapter)
            .then(bms[a].offset.cmp(&bms[b].offset))
    });
    order
}

/// Bookmark rows in reading order (chapter, then offset). The list shows
/// where bookmarks sit in the book; `orig` keeps the Vec identity that
/// selection and deletion act on.
pub fn bookmark_rows(
    bms: &[crate::Bookmark],
    chapters: &[Chapter],
    chapter_offsets: &[usize],
) -> Vec<BookmarkRow> {
    sorted_orig(bms)
        .into_iter()
        .map(|i| {
            let bm = &bms[i];
            let global = chapter_offsets.get(bm.chapter).copied().unwrap_or(0) + bm.page + 1;
            let title = chapters
                .get(bm.chapter)
                .map(|ch| crate::data::library::chapter_display_title(ch, bm.chapter))
                .unwrap_or_default();
            BookmarkRow {
                orig: i,
                label: format!("p. {global}. {title}"),
            }
        })
        .collect()
}

pub fn paint_bookmark_list(
    buf: &mut [Rgb565Pixel],
    rows: &[BookmarkRow],
    scroll: i32,
    selected_orig: Option<usize>,
    dragging: bool,
) {
    let w = crate::w();
    let h = crate::h();
    let list_bottom = h as i32 - CH_LIST_BOTTOM_PAD;
    for y in CH_LIST_TOP..list_bottom {
        let off = (y as usize) * w;
        buf[off..off + w].fill(Rgb565Pixel(0xFFFF));
    }
    if rows.is_empty() {
        paint_empty_message(buf, w, h, "No bookmarks yet");
        return;
    }
    let buf_bytes = rgb565_as_bytes(buf);
    let lh = text_render::line_height(CH_TITLE_PX) as i32;
    let row_w = (w as i32 - 2 * CH_ROW_X - SB_ROW_SHRINK) as usize;
    let title_max_w = (row_w - 80).max(40);
    for (i, row) in rows.iter().enumerate() {
        let y = CH_LIST_TOP + (i as i32) * CH_ROW_PITCH - scroll;
        if y < CH_LIST_TOP || y + CH_ROW_H > list_bottom {
            continue;
        }
        let selected = selected_orig == Some(row.orig);
        // Same state styling as chapter rows: white fill either way, black
        // card + white text when selected. No grey fill (see the note in
        // paint_chapter_list) so the list keeps the quiet e-ink waveform.
        let (fill, border, fg) = if selected {
            (INK, INK, WHITE)
        } else {
            (WHITE, BORDER, INK)
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
        let truncated = truncate_to_width(&row.label, CH_TITLE_PX, title_max_w);
        let text_y = (y + (CH_ROW_H - lh) / 2).max(0) as usize;
        text_render::blit_rgb565_color(
            buf_bytes,
            w,
            &truncated,
            CH_TITLE_PX,
            (CH_ROW_X + 16) as usize,
            text_y,
            fg,
            (CH_ROW_X + row_w as i32 - 16).max(0) as usize,
            h,
        );
    }
    paint_scrollbar(buf_bytes, w, h, rows.len(), scroll, dragging);
}

pub fn bookmark_list_hit_test(tap_y: i32, scroll: i32, row_count: usize) -> Option<usize> {
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

/// What `bm_selected` (an orig Vec index) must become after the bookmark at
/// `removed` is deleted: the deleted row's own selection dies, every later
/// index shifts down to keep naming the same bookmark, earlier ones are
/// untouched. Without the shift, deleting an earlier row silently moves the
/// selection onto a different bookmark.
pub fn selected_after_remove(selected: Option<usize>, removed: usize) -> Option<usize> {
    match selected {
        Some(s) if s == removed => None,
        Some(s) if s > removed => Some(s - 1),
        other => other,
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
    use crate::Bookmark;

    fn bm(chapter: usize, page: usize, offset: usize) -> Bookmark {
        Bookmark {
            chapter,
            page,
            offset,
        }
    }

    #[test]
    fn rows_sort_by_position_not_set_order() {
        let bms = vec![bm(3, 1, 300), bm(0, 5, 50), bm(3, 0, 280)];
        let rows = bookmark_rows(&bms, &[], &[]);
        let origs: Vec<usize> = rows.iter().map(|r| r.orig).collect();
        assert_eq!(origs, vec![1, 2, 0]);
    }

    #[test]
    fn row_label_has_global_page_and_title() {
        let bms = vec![bm(1, 4, 100)];
        let filler = Chapter::from_xhtml(0, None, "<p>filler</p>");
        let ch = Chapter::from_xhtml(1, Some("The Beginning".into()), "<p>body</p>");
        let offsets = [0usize, 30, 55];
        let rows = bookmark_rows(&bms, &[filler, ch], &offsets);
        // 30 pages before chapter 1, plus page 4, 1-based.
        assert_eq!(rows[0].label, "p. 35. The Beginning");
    }

    #[test]
    fn row_for_out_of_range_chapter_skips_title() {
        let bms = vec![bm(9, 0, 0)];
        let rows = bookmark_rows(&bms, &[], &[0, 10]);
        assert!(rows[0].label.starts_with("p. 1."));
    }

    #[test]
    fn hit_test_first_row_and_respects_scroll() {
        assert_eq!(bookmark_list_hit_test(CH_LIST_TOP, 0, 5), Some(0));
        assert_eq!(
            bookmark_list_hit_test(CH_LIST_TOP, CH_ROW_PITCH, 5),
            Some(1)
        );
    }

    #[test]
    fn hit_test_outside_list_returns_none() {
        assert_eq!(bookmark_list_hit_test(CH_LIST_TOP - 1, 0, 5), None);
        let bottom = crate::h() as i32 - CH_LIST_BOTTOM_PAD;
        assert_eq!(bookmark_list_hit_test(bottom, 0, 5), None);
    }

    #[test]
    fn hit_test_past_last_row_returns_none() {
        let last = CH_LIST_TOP + 4 * CH_ROW_PITCH;
        assert_eq!(bookmark_list_hit_test(last, 0, 5), Some(4));
        assert_eq!(bookmark_list_hit_test(last + CH_ROW_PITCH, 0, 5), None);
    }

    #[test]
    fn deleting_the_selected_row_clears_selection() {
        assert_eq!(selected_after_remove(Some(2), 2), None);
    }

    #[test]
    fn deleting_an_earlier_row_shifts_selection_down() {
        assert_eq!(selected_after_remove(Some(2), 0), Some(1));
    }

    #[test]
    fn deleting_a_later_row_keeps_selection() {
        assert_eq!(selected_after_remove(Some(1), 3), Some(1));
    }

    #[test]
    fn no_selection_stays_none() {
        assert_eq!(selected_after_remove(None, 1), None);
    }
}
