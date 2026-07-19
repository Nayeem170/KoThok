// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use log::info;

use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::loop_state::LoopState;
use crate::Reader;

use super::super::{best_effort_send, switch_chapter, ChapterSwitchOpts};

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
            best_effort_send(cmd_tx, Cmd::Seek(idx));
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
            crate::audio::glue::load_chapter_audio(&st.state, cmd_tx);
            best_effort_send(cmd_tx, Cmd::Seek(0));
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
            best_effort_send(cmd_tx, Cmd::Seek(idx));
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
            crate::audio::glue::load_chapter_audio(&st.state, cmd_tx);
            let last = st.state.pages.len().saturating_sub(1);
            let idx = crate::audio::glue::first_utt_on_page(&st.state, last);
            best_effort_send(cmd_tx, Cmd::Seek(idx));
            st.text_dirty = true;
            info!("skip-rewind: chapter {} (last page)", nc + 1);
        } else {
            best_effort_send(cmd_tx, Cmd::Seek(0));
            info!("skip-rewind: at start");
        }
    }
}
