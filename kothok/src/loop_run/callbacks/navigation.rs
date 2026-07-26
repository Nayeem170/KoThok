// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use log::info;

use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::loop_state::LoopState;
use crate::Reader;

use super::super::{best_effort_send, switch_chapter, ChapterSwitchOpts};

/// Move the reading cursor to the first text row on the current page so
/// `toggle_playback` resumes here, not from a stale cursor on the old page.
fn set_cursor_to_page_start(st: &LoopState, reader: &Reader) {
    if let Some((s, e)) = st.state.pages.get(st.current_page) {
        if let Some(rows) = st.state.all_rows.get(*s..*e) {
            for row in rows {
                if row.start < row.end {
                    reader.set_cur_start(row.start);
                    reader.set_cur_end(row.end);
                    return;
                }
            }
        }
    }
}

/// Point the audio queue at the page we just turned to, within the current
/// chapter.
///
/// Reading mode keeps only the current page's utterances queued, so a
/// chapter-wide utterance index means nothing there -- seeking with one lands
/// on some other page's sentence, and a later Play (from paused) then reads the
/// page we turned away from. The queue has to be replaced with the new page.
/// Audio mode has the whole chapter queued, so the chapter index is correct.
fn point_audio_at_current_page(
    st: &LoopState,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    chapter_idx: usize,
) {
    if matches!(st.view_mode, crate::ViewMode::Audio) {
        best_effort_send(cmd_tx, Cmd::Seek(chapter_idx));
    } else {
        let utts = crate::audio::glue::page_utterances(st.current_page, &st.state);
        best_effort_send(cmd_tx, Cmd::Reload(utts));
        best_effort_send(cmd_tx, Cmd::Seek(0));
    }
}

/// Same, right after a chapter switch: the chapter's audio has to be (re)loaded
/// at whichever granularity the current mode plays at.
fn load_audio_for_new_chapter(
    st: &LoopState,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    chapter_idx: usize,
) {
    if matches!(st.view_mode, crate::ViewMode::Audio) {
        crate::audio::glue::load_chapter_audio(&st.state, cmd_tx);
        best_effort_send(cmd_tx, Cmd::Seek(chapter_idx));
    } else {
        crate::audio::glue::load_page_audio(st.current_page, &st.state, cmd_tx);
    }
}

pub(super) fn handle_skip_forward(
    st: &mut LoopState,
    reader: &Reader,
    cb: &Callbacks,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
) {
    if cb.skip_forward_cell.replace(false) && !st.picker_active && !st.current_book_path.is_empty()
    {
        if st.current_page + 1 < st.state.pages.len() {
            let idx = crate::audio::glue::first_utt_on_page(&st.state, st.current_page + 1);
            st.current_page += 1;
            crate::reader::apply_page(
                reader,
                &st.state,
                st.current_page,
                &st.chapter_offsets,
                st.current_chapter,
            );
            let base = st
                .chapter_offsets
                .get(st.current_chapter)
                .copied()
                .unwrap_or(0);
            reader.set_saved_page((base + st.current_page) as i32);
            set_cursor_to_page_start(st, reader);
            point_audio_at_current_page(st, cmd_tx, idx);
            st.text_dirty = true;
            info!("skip-forward: page {}", st.current_page + 1);
        } else if st.current_chapter + 1 < st.chapter_count {
            let nc = st.current_chapter + 1;
            switch_chapter(
                st,
                reader,
                cmd_tx,
                nc,
                ChapterSwitchOpts {
                    to_last_page: false,
                    update_cursor: false,
                    load_audio: true,
                },
            );
            set_cursor_to_page_start(st, reader);
            load_audio_for_new_chapter(st, cmd_tx, 0);
            st.text_dirty = true;
            info!("skip-forward: chapter {}", nc + 1);
        }
    }
}

pub(super) fn handle_skip_rewind(
    st: &mut LoopState,
    reader: &Reader,
    cb: &Callbacks,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
) {
    if cb.skip_rewind_cell.replace(false) && !st.picker_active && !st.current_book_path.is_empty() {
        if st.current_page > 0 {
            let idx = crate::audio::glue::first_utt_on_page(&st.state, st.current_page - 1);
            st.current_page -= 1;
            crate::reader::apply_page(
                reader,
                &st.state,
                st.current_page,
                &st.chapter_offsets,
                st.current_chapter,
            );
            let base = st
                .chapter_offsets
                .get(st.current_chapter)
                .copied()
                .unwrap_or(0);
            reader.set_saved_page((base + st.current_page) as i32);
            set_cursor_to_page_start(st, reader);
            point_audio_at_current_page(st, cmd_tx, idx);
            st.text_dirty = true;
            info!("skip-rewind: page {}", st.current_page + 1);
        } else if st.current_chapter > 0 {
            let nc = st.current_chapter - 1;
            switch_chapter(
                st,
                reader,
                cmd_tx,
                nc,
                ChapterSwitchOpts {
                    to_last_page: true,
                    update_cursor: false,
                    load_audio: true,
                },
            );
            let last = st.state.pages.len().saturating_sub(1);
            let idx = crate::audio::glue::first_utt_on_page(&st.state, last);
            set_cursor_to_page_start(st, reader);
            load_audio_for_new_chapter(st, cmd_tx, idx);
            st.text_dirty = true;
            info!("skip-rewind: chapter {} (last page)", nc + 1);
        } else {
            best_effort_send(cmd_tx, Cmd::Seek(0));
            info!("skip-rewind: at start");
        }
    }
}
