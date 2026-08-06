// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;

use crate::data::word_index::MAX_SEARCH_RESULTS;
use crate::loop_state::ChapterTab;
use crate::reader::{apply_page, switch_chapter, ChapterSwitchOpts};
use crate::rendering::chapter_list::{list_scroll_max, CH_LIST_BOTTOM_PAD, CH_LIST_TOP};
use crate::rendering::search_results::results_hit_test;
use crate::rendering::word_list::word_list_hit_test;
use crate::Reader;

pub(super) fn handle_search_release(
    st: &mut LoopState,
    ctx: &mut LoopContext,
    dx: f32,
    dy: f32,
) -> bool {
    let (press_dx, press_dy) = touch::to_display(st.press_x, st.press_y, ctx.touch_cfg);
    let swipe_dy = dy - press_dy;
    let swipe_dx = dx - press_dx;

    if st.search_results_active {
        if swipe_dy.abs() <= 40.0 || swipe_dy.abs() <= swipe_dx.abs() {
            let hit_count = st
                .word_index
                .occurrences
                .get(st.search_selected_word)
                .map(|h| h.len().min(MAX_SEARCH_RESULTS))
                .unwrap_or(0);
            if let Some(idx) = results_hit_test(dy as i32, st.search_results_scroll, hit_count) {
                st.search_selected_result = idx;
                st.search_result_selected = true;
                st.text_dirty = true;
                ctx.window.request_redraw();
            }
        }
        return true;
    }

    match st.chapter_tab {
        ChapterTab::Words => {
            if swipe_dy.abs() > 40.0 && swipe_dy.abs() > swipe_dx.abs() {
                st.text_dirty = true;
                ctx.window.request_redraw();
                return true;
            }
            if let Some(idx) =
                word_list_hit_test(dy as i32, st.search_scroll, st.word_index.words.len())
            {
                select_word(st, idx);
                st.text_dirty = true;
                ctx.window.request_redraw();
                return true;
            }
            return true;
        }
        ChapterTab::Chapters => {}
    }
    false
}

pub(super) fn select_word(st: &mut LoopState, idx: usize) {
    st.search_selected_word = idx;
    st.search_word_selected = true;
    st.search_result_selected = false;
}

pub(super) fn back_from_results(st: &mut LoopState) {
    st.search_results_active = false;
    st.search_results_scroll = 0;
    st.search_result_selected = false;
}

pub(super) fn jump_to_occurrence(
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
    st.search_result_selected = false;
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
    list_scroll_max(word_count, list_h)
}

pub(super) fn results_scroll_max(result_count: usize) -> i32 {
    search_scroll_max(result_count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::word_index::WordIndex;
    use crate::loop_state::ChapterTab;
    use crate::rendering::layout::ChapterState;
    use slint::platform::software_renderer::Rgb565Pixel;

    fn minimal_st() -> LoopState {
        let w = crate::w();
        let h = crate::h();
        let now = std::time::Instant::now();
        let buffer = vec![Rgb565Pixel(0); w * h];
        LoopState {
            current_chapter: 0,
            current_page: 0,
            chapter_count: 0,
            chapters: Vec::new(),
            toc_rows: Vec::new(),
            chapter_offsets: Vec::new(),
            state: ChapterState {
                all_rows: Vec::new(),
                row_heights: Vec::new(),
                pages: Vec::new(),
                utterances: Vec::new(),
                decoded_images: std::collections::HashMap::new(),
                style_runs: Vec::new(),
                links: Vec::new(),
            },
            body_px: 24.0,
            head_px: 18.0,
            line_h: 30,
            line_spacing_pct: 140,
            text_justify: true,
            current_book_path: String::new(),
            reading_ch: 0,
            reading_pg: 0,
            reading_off: 0,
            reading_end: 0,
            picker_active: false,
            picker_scroll: 0,
            picker_cells: Vec::new(),
            library_filter: Default::default(),
            picker_cover_cache: Default::default(),
            picker_entered: None,
            picker_last_tap_idx: None,
            picker_last_tap_time: now,
            panel_open: false,
            prev_panel_open: false,
            prev_chapter_overlay: false,
            cover_page_visible: false,
            chapter_scroll: 0,
            press_chapter_scroll: 0,
            word_index: WordIndex::default(),
            chapter_tab: ChapterTab::Words,
            search_scroll: 0,
            search_results_active: false,
            search_results_scroll: 0,
            search_selected_word: 0,
            search_word_selected: false,
            search_selected_result: 0,
            search_result_selected: false,
            press_search_scroll: 0,
            press_search_results_scroll: 0,
            sb_dragging: false,
            sb_drag_tab: Default::default(),
            sb_grab_offset: 0,
            bt_fail_count: 0,
            text_dirty: false,
            #[cfg(feature = "screenshot")]
            shot_armed: false,
            #[cfg(feature = "screenshot")]
            shot_done: false,
            system_state: crate::SystemState::Awake,
            view_mode: crate::ViewMode::Reading,
            prev_view_mode: crate::ViewMode::Reading,
            bookmark: None,
            lock_time: None,
            saved_brightness: 0,
            lock_radios_off: false,
            lock_wifi_off: false,
            lock_bt_off: false,
            wifi_user_on: false,
            bt_user_on: false,
            disk_key: None,
            disk_cover: None,
            disk_cover_path: String::new(),
            cover_rotation: 0.0,
            prev_cover_rotation: 0.0,
            last_cover_rot: now,
            disk_spin_only: false,
            disk_settle: false,
            prev_playing: false,
            prev_down: false,
            frame_down: false,
            frame_x: 0,
            frame_y: 0,
            tap_xy: None,
            scrubbing: false,
            pp_pressed: false,
            pp_pending_release: None,
            saved_pos: None,
            saved_pos_at: None,
            lib_pressed: false,
            menu_pressed: false,
            mode_toggle_pressed: false,
            bookmark_set_pressed: false,
            bookmark_jump_pressed: false,
            sleep_pressed: false,
            chapter_pressed: false,
            header_visible: true,
            header_revealed_at: None,
            pending_tap_at: None,
            press_dispatched: false,
            press_x: 0,
            press_y: 0,
            press_time: now,
            last_double_tap: now,
            last_tap_time: now,
            last_tap_y: -1,
            exit_armed: false,
            about_open: false,
            device_model: String::new(),
            exit_armed_time: now,
            offset_rx: None,
            last_activity: now,
            last_status_refresh: now,
            last_nav: now,
            last_font_count: 0,
            buffer,
            prev_buffer: vec![Rgb565Pixel(0); w * h],
            text_cache: vec![Rgb565Pixel(0); w * h],
            zoom_active: false,
            zoom_center: (0, 0),
            voice_rx: None,
            voice_fetch_attempted: false,
            wifi_bt_list_rx: None,
            font_download_rx: None,
            update_check_rx: None,
            wifi_list: Vec::new(),
            wifi_list_idx: 0,
            wifi_list_fetched: false,
            wifi_list_ids_valid: true,
            bt_list: Vec::new(),
            bt_list_idx: 0,
            bt_list_fetched: false,
            bt_list_ids_valid: true,
        }
    }

    #[test]
    fn word_tap_selects_does_not_open_results() {
        let mut st = minimal_st();
        st.word_index.words = vec![
            "alpha".to_string(),
            "bravo".to_string(),
            "charlie".to_string(),
        ];
        select_word(&mut st, 1);
        assert!(st.search_word_selected);
        assert_eq!(st.search_selected_word, 1);
        assert!(!st.search_results_active);
    }

    #[test]
    fn back_from_results_preserves_selection() {
        let mut st = minimal_st();
        st.search_results_active = true;
        st.search_results_scroll = 42;
        st.search_word_selected = true;
        st.search_selected_word = 3;
        back_from_results(&mut st);
        assert!(!st.search_results_active);
        assert_eq!(st.search_results_scroll, 0);
        assert!(st.search_word_selected);
        assert_eq!(st.search_selected_word, 3);
    }
}
