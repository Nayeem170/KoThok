// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use log::info;

use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::loop_state::LoopState;
use crate::reader::apply_page;
use crate::Reader;

use super::super::{LoopContext, ViewMode};

pub(super) fn process_mode_toggle(
    st: &mut LoopState,
    reader: &Reader,
    cb: &Callbacks,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
) -> bool {
    if cb.mode_toggle_cell.replace(false) && !st.picker_active && !st.current_book_path.is_empty() {
        let new_mode = match st.view_mode {
            ViewMode::Reading => ViewMode::Audio,
            ViewMode::Audio => ViewMode::Reading,
        };
        info!(
            "mode-toggle: START {:?} -> {:?} (ch={} pg={} pages={} utts={})",
            st.view_mode,
            new_mode,
            st.current_chapter,
            st.current_page,
            st.state.pages.len(),
            st.state.utterances.len(),
        );
        st.view_mode = new_mode;
        info!(
            "mode-toggle: set_audio_mode({})",
            new_mode == ViewMode::Audio
        );
        reader.set_audio_mode(new_mode == ViewMode::Audio);
        match new_mode {
            ViewMode::Audio => {
                info!("mode-toggle: loading chapter audio");
                crate::audio::glue::load_chapter_audio(&st.state, cmd_tx);
                let off = reader.get_cur_start().max(0) as usize;
                let idx = crate::audio::glue::utterance_index_for_offset(&st.state.utterances, off);
                info!("mode-toggle: seek to utt {} (off={})", idx, off);
                crate::audio::glue::best_effort_send(cmd_tx, Cmd::Seek(idx));
            }
            ViewMode::Reading => {
                let off = reader.get_cur_start().max(0) as usize;
                if let Some(page) = st.state.page_for_offset(off) {
                    if page != st.current_page {
                        info!(
                            "mode-toggle: correcting page {} -> {} (off={})",
                            st.current_page + 1,
                            page + 1,
                            off
                        );
                        st.current_page = page;
                    }
                }
                info!(
                    "mode-toggle: switching to reading, re-applying page {}",
                    st.current_page + 1
                );
                apply_page(
                    reader,
                    &st.state,
                    st.current_page,
                    &st.chapter_offsets,
                    st.current_chapter,
                );
                let page_utts = crate::audio::glue::page_utterances(st.current_page, &st.state);
                let idx = crate::audio::glue::utterance_index_for_offset(&page_utts, off);
                crate::audio::glue::best_effort_send(cmd_tx, Cmd::Reload(page_utts));
                crate::audio::glue::best_effort_send(cmd_tx, Cmd::Seek(idx));
            }
        }
        st.text_dirty = true;
        info!("mode-toggle: DONE {:?}", new_mode);
        true
    } else {
        false
    }
}
