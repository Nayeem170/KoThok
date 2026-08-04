// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use kobo_core::Chapter;

use crate::data::config::AppConfig;
use crate::data::persistence::{cache_path, load_offset_cache, ReadingPosition};
use crate::reader::apply_page;
use crate::rendering::layout::{
    build_state, estimate_chapter_offsets, spawn_offset_computation, ChapterState,
    OffsetComputation,
};
use crate::Reader;

pub struct BookSession {
    pub state: ChapterState,
    pub chapter_offsets: Vec<usize>,
    pub current_page: usize,
    pub offset_rx: Option<OffsetComputation>,
    pub reading_ch: usize,
    pub reading_pg: usize,
    pub reading_off: usize,
    pub reading_end: usize,
    pub show_cover: bool,
}

pub fn open_book_session(
    chapters: &mut [Chapter],
    pos: &ReadingPosition,
    cfg: &AppConfig,
    body_px: f32,
    head_px: f32,
    line_h: i32,
    book_path: &str,
) -> BookSession {
    // Justification is a saved per-book setting; hardcoding `true` here meant a
    // book left on left-alignment reopened justified until the panel was
    // touched again.
    let state = build_state(
        &mut chapters[pos.chapter],
        body_px,
        head_px,
        line_h,
        cfg.text_justify,
    );
    let (chapter_offsets, offset_rx) = resolve_offsets(
        chapters,
        pos.chapter,
        &state,
        cfg,
        body_px,
        head_px,
        line_h,
        book_path,
    );
    let npages = state.pages.len();
    // `current_page` is what the reader was *looking at* -- `pos.page`, taken
    // as-is. `reading_pg`/`reading_off`/`reading_end` are the separate "jump to
    // reading position" target the TTS cursor last committed to (see
    // `jump_reading_position`), which browsing (a swipe, a progress-bar drag)
    // deliberately leaves behind without moving (`process_page_navigation`).
    // Deriving the displayed page from `cur_start` conflated the two: after
    // browsing forward from wherever playback last was and closing the book
    // there, every reopen snapped back to the stale cursor's page instead of
    // the page actually on screen.
    let current_page = pos.page.min(npages.saturating_sub(1));
    let (reading_pg, reading_off, reading_end) = if pos.cur_start > 0 {
        let page = state.page_for_offset(pos.cur_start).unwrap_or(current_page);
        let (mut off, mut end) = (pos.cur_start, pos.cur_end.max(pos.cur_start));
        if let Some((rs, re)) = state.pages.get(page) {
            if let Some(rows) = state.all_rows.get(*rs..*re) {
                for row in rows {
                    if row.start < row.end
                        && pos.cur_start >= row.start as usize
                        && pos.cur_start < row.end as usize
                    {
                        off = row.start as usize;
                        end = row.end as usize;
                        break;
                    }
                }
            }
        }
        (page, off, end)
    } else {
        let mut off = 0;
        let mut end = 0;
        if let Some((rs, re)) = state.pages.get(current_page).copied() {
            if let Some(slice) = state.all_rows.get(rs..re) {
                for row in slice {
                    if row.start < row.end {
                        off = row.start as usize;
                        end = row.end as usize;
                        break;
                    }
                }
            }
        }
        (current_page, off, end)
    };
    BookSession {
        state,
        chapter_offsets,
        current_page,
        offset_rx,
        reading_ch: pos.chapter,
        reading_pg,
        reading_off,
        reading_end,
        show_cover: pos.bookmark.is_none() && pos.cur_start == 0,
    }
}

fn resolve_offsets(
    chapters: &[Chapter],
    current_chapter: usize,
    state: &ChapterState,
    cfg: &AppConfig,
    body_px: f32,
    _head_px: f32,
    line_h: i32,
    book_path: &str,
) -> (Vec<usize>, Option<OffsetComputation>) {
    let cached = load_offset_cache(&cache_path(book_path, cfg.font_size));
    if let Some(offsets) = cached {
        if offsets.len() == chapters.len() + 1 {
            return (offsets, None);
        }
    }
    let layout = crate::rendering::layout::screen_layout();
    let estimated = estimate_chapter_offsets(
        chapters,
        (current_chapter, state.pages.len()),
        line_h,
        &layout,
    );
    let rx = Some(spawn_offset_computation(
        chapters.to_vec(),
        body_px,
        line_h,
        cfg.font_size,
        book_path.to_string(),
        layout,
    ));
    (estimated, rx)
}

pub fn apply_session(reader: &Reader, session: &BookSession, current_chapter: usize) {
    apply_page(
        reader,
        &session.state,
        session.current_page,
        &session.chapter_offsets,
        current_chapter,
    );
    if session.reading_off > 0 {
        reader.set_cur_start(session.reading_off as i32);
        reader.set_cur_end(session.reading_end as i32);
    }
    reader.set_saved_page(
        (*session.chapter_offsets.get(current_chapter).unwrap_or(&0) + session.current_page) as i32,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use kobo_core::Chapter;

    fn multi_page_chapter() -> Chapter {
        let xhtml: String = (0..60)
            .map(|i| format!("<p>Paragraph number {i} of the test chapter body.</p>"))
            .collect();
        Chapter::from_xhtml(0, None, &xhtml)
    }

    fn base_pos() -> ReadingPosition {
        ReadingPosition {
            chapter: 0,
            page: 0,
            cur_start: 0,
            cur_end: 0,
            view_mode: crate::ViewMode::Reading,
            bookmark: None,
            progress: 0.0,
        }
    }

    /// The device regression this decoupling exists for. Browsing (a swipe) is
    /// deliberately designed to leave the reading cursor behind -- see
    /// `process_page_navigation` in `app/events.rs` -- so `cur_start` on exit
    /// can legitimately still point at an earlier page than the one on screen.
    /// Reopening used to resolve the displayed page from that stale cursor
    /// instead of from `pos.page`, so a book closed after browsing forward
    /// reopened back at the page browsing started from.
    #[test]
    fn reopens_on_the_displayed_page_not_the_stale_cursors_page() {
        let mut chapters = vec![multi_page_chapter()];
        let cfg = AppConfig::default();

        // Establish where page 0's and a later page's rows actually begin, the
        // same way the app would have while reading page 0 before browsing on.
        let probe = build_state(&mut chapters[0], 36.0, 48.0, 48, true);
        assert!(
            probe.pages.len() >= 3,
            "fixture must paginate to several pages: {}",
            probe.pages.len()
        );
        let browsed_to = probe.pages.len() - 1;
        let (rs, re) = probe.pages[0];
        // Any row inside page 0 with a real byte range and a non-zero start --
        // not necessarily the page's first row, which starts at byte 0 and
        // would be indistinguishable from "no saved cursor at all".
        let stale_cursor = probe.all_rows[rs..re]
            .iter()
            .find(|r| r.start < r.end && r.start > 0)
            .map(|r| r.start as usize)
            .expect("page 0 must contain a row past byte 0");

        let mut pos = base_pos();
        pos.page = browsed_to;
        pos.cur_start = stale_cursor;
        pos.cur_end = stale_cursor + 1;

        let session = open_book_session(&mut chapters, &pos, &cfg, 36.0, 48.0, 48, "");

        assert_eq!(
            session.current_page, browsed_to,
            "must reopen where the reader was looking, not where the cursor was"
        );
        // The stale cursor is still honoured for "jump to reading position".
        assert_eq!(session.reading_pg, 0);
    }

    #[test]
    fn reopens_on_the_saved_page_when_never_read_at_all() {
        let mut chapters = vec![multi_page_chapter()];
        let cfg = AppConfig::default();
        let pos = base_pos();
        let session = open_book_session(&mut chapters, &pos, &cfg, 36.0, 48.0, 48, "");
        assert_eq!(session.current_page, 0);
        assert_eq!(session.reading_pg, 0);
    }
}
