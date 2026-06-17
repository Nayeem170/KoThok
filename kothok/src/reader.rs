use slint::{ModelRc, VecModel};

use crate::audio::glue::utterance_index_for_offset;
use crate::audio::Cmd;
use crate::rendering::layout::{build_state, ChapterState};

use crate::Reader;
use crate::Row;
use log::debug;

/// Pure: clamp a requested page index into the valid range for a chapter with
/// `page_count` pages. An empty chapter (0 pages) maps any request to 0.
pub fn clamp_page(page: usize, page_count: usize) -> usize {
    if page_count == 0 {
        0
    } else {
        page.min(page_count - 1)
    }
}

/// The first text-bearing row (start < end) within `[s, e)`, as a
/// `(start, end)` byte-offset pair. Shared by `compute_page_view` (cursor
/// placement) and `switch_chapter` (audio-seek target).
pub fn first_text_row(state: &ChapterState, s: usize, e: usize) -> Option<(i32, i32)> {
    state
        .all_rows
        .get(s..e)?
        .iter()
        .find(|r| r.start < r.end)
        .map(|r| (r.start, r.end))
}

/// Pure: the on-page reading cursor `(cur_start, cur_end)` for the page whose
/// row range is `[s, e)`. If the first row is a heading/gap (start == end),
/// falls back to the first real text row on the page; if none exists, (0, 0).
pub fn page_cursor(state: &ChapterState, s: usize, e: usize) -> (i32, i32) {
    match state.all_rows.get(s) {
        Some(first) if first.start < first.end => (first.start, first.end),
        Some(_) => first_text_row(state, s, e).unwrap_or((0, 0)),
        None => (0, 0),
    }
}

/// Pure, testable view of what `apply_page` would set on the Slint `Reader`.
/// Computing this separately means every branch (page clamping, absolute-page
/// math, cursor placement, heading/gap fallback) is unit-testable without a
/// Slint runtime.
pub struct PageView {
    /// Rows visible on this page.
    pub rows: Vec<Row>,
    /// Absolute page number across the whole book.
    pub absolute_page: i32,
    /// Total pages in the book (for the progress bar).
    pub page_count: i32,
    /// Reading cursor to place on the page. `None` when audio is not playing
    /// (the cursor is left untouched so a manual turn drops no marker).
    pub cursor: Option<(i32, i32)>,
}

pub fn compute_page_view(
    state: &ChapterState,
    page: usize,
    chapter_offsets: &[usize],
    current_chapter: usize,
    playing: bool,
) -> PageView {
    let idx = clamp_page(page, state.pages.len());
    let (s, e) = state.pages.get(idx).copied().unwrap_or((0, 0));
    let cursor = if playing {
        Some(page_cursor(state, s, e))
    } else {
        None
    };
    PageView {
        rows: state.all_rows[s..e].to_vec(),
        absolute_page: (chapter_offsets.get(current_chapter).copied().unwrap_or(0) + idx) as i32,
        page_count: *chapter_offsets.last().unwrap_or(&1) as i32,
        cursor,
    }
}

pub fn apply_page(
    reader: &Reader,
    state: &ChapterState,
    page: usize,
    chapter_offsets: &[usize],
    current_chapter: usize,
) {
    let v = compute_page_view(
        state,
        page,
        chapter_offsets,
        current_chapter,
        reader.get_playing(),
    );
    reader.set_rows(ModelRc::new(VecModel::from(v.rows)));
    reader.set_page(v.absolute_page);
    reader.set_page_count(v.page_count);
    reader.set_current_chapter_idx(current_chapter as i32);
    if let Some((cs, ce)) = v.cursor {
        reader.set_cur_start(cs);
        reader.set_cur_end(ce);
    }
}

pub fn switch_chapter(
    st: &mut crate::loop_state::LoopState,
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    nc: usize,
    to_last_page: bool,
    update_cursor: bool,
) {
    let body_px = st.body_px;
    let head_px = st.head_px;
    let line_h = st.line_h;
    st.current_chapter = nc;
    st.state = build_state(&mut st.chapters[nc], body_px, head_px, line_h);
    st.current_page = if to_last_page {
        st.state.pages.len().saturating_sub(1)
    } else {
        0
    };
    let cc = st.current_chapter;
    let cp = st.current_page;
    apply_page(reader, &st.state, cp, &st.chapter_offsets, cc);
    let cn = crate::data::library::chapter_display_title(&st.chapters[nc], nc);
    crate::set_chapter_name(reader, &cn);
    // best-effort: channel may be full
    let _ = cmd_tx.send(Cmd::Reload(st.state.utterances.clone()));
    // Only move the reading cursor for an actual playback progression
    // (auto-advance). When browsing/opening a chapter from the list, leave the
    // cursor on the last reading line — like the saved-page tick, it must NOT
    // jump to the opened chapter's first line.
    if update_cursor {
        if let Some(&(s, e)) = st.state.pages.get(cp) {
            if let Some((row_start, row_end)) = first_text_row(&st.state, s, e) {
                reader.set_cur_start(row_start);
                reader.set_cur_end(row_end);
                let utt_idx = utterance_index_for_offset(&st.state.utterances, row_start as usize);
                // best-effort: channel may be full
                let _ = cmd_tx.send(Cmd::Seek(utt_idx));
            }
        }
    }
    debug!(
        "chapter -> {}/{} (page {}, to_last={}, pages={})",
        nc + 1,
        st.chapter_count,
        cp,
        to_last_page,
        st.state.pages.len()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rendering::layout::ChapterState;
    use std::collections::HashMap;

    fn row(start: i32, end: i32) -> Row {
        Row {
            text: format!("[{start},{end})").into(),
            start,
            end,
            kind: 0,
            tag: 0,
        }
    }

    // A heading/gap row carries no text offset (start == end).
    fn gap() -> Row {
        Row {
            text: "".into(),
            start: 5,
            end: 5,
            kind: 0,
            tag: 0,
        }
    }

    fn state_with(rows: Vec<Row>, pages: Vec<(usize, usize)>) -> ChapterState {
        let n = rows.len();
        ChapterState {
            all_rows: rows,
            row_heights: vec![40; n],
            pages,
            utterances: vec![],
            decoded_images: HashMap::new(),
        }
    }

    #[test]
    fn clamp_page_within_bounds() {
        assert_eq!(clamp_page(0, 10), 0);
        assert_eq!(clamp_page(5, 10), 5);
        assert_eq!(clamp_page(9, 10), 9);
        assert_eq!(clamp_page(20, 10), 9, "over-range clamps to last page");
    }

    #[test]
    fn clamp_page_zero_pages_safe() {
        assert_eq!(clamp_page(0, 0), 0);
        assert_eq!(
            clamp_page(100, 0),
            0,
            "empty chapter never indexes out of range"
        );
    }

    #[test]
    fn compute_page_view_clamps_over_range_page() {
        let st = state_with(vec![row(0, 10), row(10, 20)], vec![(0, 2)]);
        let v = compute_page_view(&st, 99, &[0, 2], 0, false);
        assert_eq!(v.rows.len(), 2, "over-range page clamps to the only page");
        assert_eq!(v.absolute_page, 0);
    }

    #[test]
    fn compute_page_view_absolute_page_sums_offset_and_idx() {
        // chapter_offsets[1] = 4, page idx 2 within chapter -> absolute page 6.
        let st = state_with(vec![row(0, 10)], vec![(0, 1)]);
        let v = compute_page_view(&st, 0, &[0, 4, 9], 1, false);
        assert_eq!(v.absolute_page, 4, "absolute page = chapter base + idx");
        assert_eq!(v.page_count, 9, "page count = chapter_offsets.last()");
    }

    #[test]
    fn compute_page_view_cursor_none_when_not_playing() {
        let st = state_with(vec![row(0, 10)], vec![(0, 1)]);
        let v = compute_page_view(&st, 0, &[0, 1], 0, false);
        assert!(v.cursor.is_none(), "no cursor placed when not playing");
    }

    #[test]
    fn compute_page_view_cursor_on_first_text_row_when_playing() {
        let st = state_with(vec![row(0, 10), row(10, 20)], vec![(0, 2)]);
        let v = compute_page_view(&st, 0, &[0, 2], 0, true);
        assert_eq!(v.cursor, Some((0, 10)), "cursor marks the first row");
    }

    #[test]
    fn compute_page_view_cursor_falls_back_past_heading() {
        // First row is a heading/gap (start==end); cursor must skip to the next
        // real text row on the page.
        let st = state_with(vec![gap(), row(20, 35)], vec![(0, 2)]);
        let v = compute_page_view(&st, 0, &[0, 2], 0, true);
        assert_eq!(v.cursor, Some((20, 35)), "cursor skips the heading gap row");
    }

    #[test]
    fn compute_page_view_cursor_zero_when_page_has_no_text() {
        let st = state_with(vec![gap(), gap()], vec![(0, 2)]);
        let v = compute_page_view(&st, 0, &[0, 2], 0, true);
        assert_eq!(v.cursor, Some((0, 0)), "no text row -> (0,0) cursor");
    }

    #[test]
    fn compute_page_view_empty_chapter_safe() {
        // Zero pages: idx clamps to 0 without indexing into an empty pages vec.
        let st = state_with(vec![], vec![]);
        let v = compute_page_view(&st, 0, &[0, 0], 0, true);
        assert!(v.rows.is_empty());
        assert_eq!(v.cursor, Some((0, 0)));
    }

    #[test]
    fn first_text_row_skips_gaps() {
        let st = state_with(vec![gap(), gap(), row(40, 55), row(55, 70)], vec![(0, 4)]);
        assert_eq!(first_text_row(&st, 0, 4), Some((40, 55)));
    }

    #[test]
    fn first_text_row_none_when_all_gaps() {
        let st = state_with(vec![gap(), gap()], vec![(0, 2)]);
        assert_eq!(first_text_row(&st, 0, 2), None);
    }

    #[test]
    fn first_text_row_none_for_empty_range() {
        let st = state_with(vec![row(0, 10)], vec![(0, 0)]);
        assert_eq!(first_text_row(&st, 0, 0), None, "empty row slice -> None");
    }

    #[test]
    fn page_cursor_uses_first_row_when_text() {
        let st = state_with(vec![row(7, 19), row(19, 30)], vec![(0, 2)]);
        assert_eq!(page_cursor(&st, 0, 2), (7, 19));
    }

    #[test]
    fn page_cursor_falls_back_past_leading_gap() {
        let st = state_with(vec![gap(), row(30, 44)], vec![(0, 2)]);
        assert_eq!(page_cursor(&st, 0, 2), (30, 44));
    }
}
