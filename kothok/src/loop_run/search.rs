// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;

use crate::data::word_index::MAX_SEARCH_RESULTS;
use crate::gesture;
use crate::loop_state::ChapterTab;
use crate::reader::{apply_page, switch_chapter, ChapterSwitchOpts};
use crate::rendering::chapter_list::{CH_LIST_BOTTOM_PAD, CH_LIST_TOP, CH_ROW_H, CH_ROW_PITCH};
use crate::rendering::search_results::results_hit_test;
use crate::rendering::word_list::word_list_hit_test;
use crate::Reader;

pub(super) fn handle_search_release(
    st: &mut LoopState,
    ctx: &mut LoopContext,
    dx: f32,
    dy: f32,
) -> bool {
    let reader = ctx.reader;
    let cmd_tx = ctx.cmd_tx;
    let (press_dx, press_dy) = touch::to_display(st.press_x, st.press_y, ctx.touch_cfg);
    let swipe_dy = dy - press_dy;
    let swipe_dx = dx - press_dx;

    if st.search_results_active {
        if gesture::search_header_hit_test(dx, dy, ctx.w) == gesture::TabBarAction::Back {
            st.search_results_active = false;
            st.search_results_scroll = 0;
            st.text_dirty = true;
            ctx.window.request_redraw();
            return true;
        }
        if swipe_dy.abs() <= 40.0 || swipe_dy.abs() <= swipe_dx.abs() {
            let hit_count = st
                .word_index
                .occurrences
                .get(st.search_selected_word)
                .map(|h| h.len().min(MAX_SEARCH_RESULTS))
                .unwrap_or(0);
            if let Some(idx) = results_hit_test(dy as i32, st.search_results_scroll, hit_count) {
                return jump_to_occurrence(st, reader, cmd_tx, idx);
            }
        }
        return true;
    }

    match gesture::tab_bar_hit_test(dx, dy, ctx.w) {
        gesture::TabBarAction::ChaptersTab => {
            st.chapter_tab = ChapterTab::Chapters;
            st.chapter_scroll = 0;
            st.search_word_selected = false;
            reader.set_chapter_overlay_active_tab(0);
            st.text_dirty = true;
            ctx.window.request_redraw();
            return true;
        }
        gesture::TabBarAction::WordsTab => {
            st.chapter_tab = ChapterTab::Words;
            st.search_scroll = 0;
            reader.set_chapter_overlay_active_tab(1);
            st.text_dirty = true;
            ctx.window.request_redraw();
            return true;
        }
        gesture::TabBarAction::Close => {
            reader.set_chapter_overlay_open(false);
            st.search_word_selected = false;
            st.text_dirty = true;
            return true;
        }
        _ => {}
    }

    if st.chapter_tab == ChapterTab::Words {
        if swipe_dy.abs() > 40.0 && swipe_dy.abs() > swipe_dx.abs() {
            st.text_dirty = true;
            ctx.window.request_redraw();
            return true;
        }
        if let Some(idx) =
            word_list_hit_test(dy as i32, st.search_scroll, st.word_index.words.len())
        {
            st.search_selected_word = idx;
            st.search_word_selected = true;
            st.text_dirty = true;
            ctx.window.request_redraw();
            return true;
        }
        return true;
    }
    false
}

fn jump_to_occurrence(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    result_idx: usize,
) -> bool {
    let hits = match st.word_index.occurrences.get(st.search_selected_word) {
        Some(h) => h,
        None => return false,
    };
    let hit = match hits.get(result_idx) {
        Some(h) => h.clone(),
        None => return false,
    };
    let target_ch = hit.chapter as usize;
    if target_ch >= st.chapters.len() {
        return false;
    }
    if target_ch != st.current_chapter {
        switch_chapter(
            st,
            reader,
            cmd_tx,
            target_ch,
            ChapterSwitchOpts {
                to_last_page: false,
                update_cursor: false,
                load_audio: false,
            },
        );
    }
    let page = st
        .state
        .page_for_offset(hit.byte_offset as usize)
        .unwrap_or(0);
    st.current_page = page;
    apply_page(
        reader,
        &st.state,
        page,
        &st.chapter_offsets,
        st.current_chapter,
    );
    if let Some(&(s, e)) = st.state.pages.get(page) {
        let hit_row = st.state.all_rows.get(s..e).and_then(|rows| {
            rows.iter()
                .find(|r| r.start <= (hit.byte_offset as i32) && (hit.byte_offset as i32) < r.end)
        });
        let (row_start, row_end) = hit_row
            .map(|r| (r.start, r.end))
            .unwrap_or_else(|| crate::reader::first_text_row(&st.state, s, e).unwrap_or((0, 0)));
        reader.set_cur_start(row_start);
        reader.set_cur_end(row_end);
    }
    let utts = page_utterances(st.current_page, &st.state);
    let utt_idx = crate::audio::glue::utterance_index_for_offset(&utts, hit.byte_offset as usize);
    best_effort_send(cmd_tx, Cmd::Reload(utts));
    best_effort_send(cmd_tx, Cmd::Seek(utt_idx));
    st.search_results_active = false;
    reader.set_chapter_overlay_open(false);
    st.text_dirty = true;
    let cn = crate::data::library::chapter_display_title(&st.chapters[target_ch], target_ch);
    crate::set_chapter_name(reader, &cn);
    true
}

pub(super) fn search_scroll_max(word_count: usize) -> i32 {
    let h = crate::h() as i32;
    let list_h = h - CH_LIST_TOP - CH_LIST_BOTTOM_PAD;
    let content_h = word_count.saturating_sub(1) as i32 * CH_ROW_PITCH + CH_ROW_H;
    (content_h - list_h).max(0)
}

pub(super) fn results_scroll_max(result_count: usize) -> i32 {
    search_scroll_max(result_count)
}
