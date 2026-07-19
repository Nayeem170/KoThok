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
    let state = build_state(&mut chapters[pos.chapter], body_px, head_px, line_h);
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
    let current_page = pos.page.min(state.pages.len().saturating_sub(1));
    let (reading_off, reading_end) = if pos.cur_start > 0 {
        (pos.cur_start, pos.cur_end)
    } else {
        let (rs, re) = state.pages.get(current_page).copied().unwrap_or((0, 0));
        let mut off = 0;
        let mut end = 0;
        if let Some(slice) = state.all_rows.get(rs..re) {
            for row in slice {
                if row.start < row.end {
                    off = row.start as usize;
                    end = row.end as usize;
                    break;
                }
            }
        }
        (off, end)
    };
    BookSession {
        state,
        chapter_offsets,
        current_page,
        offset_rx,
        reading_ch: pos.chapter,
        reading_pg: current_page,
        reading_off,
        reading_end,
        show_cover: pos.page == 0 && pos.cur_start == 0,
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
    reader.set_saved_page((session.chapter_offsets[current_chapter] + session.current_page) as i32);
}
