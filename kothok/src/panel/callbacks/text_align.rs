// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::cell::Cell;
use std::rc::Rc;

use slint::platform::software_renderer::Rgb565Pixel;

use crate::audio::glue::{best_effort_send, page_utterances, utterance_index_for_offset};
use crate::audio::Cmd;
use crate::data::config::{save_settings_for, AppConfig};
use crate::loop_state::LoopState;
use crate::reader::apply_page;
use crate::rendering::layout::{build_state, estimate_chapter_offsets, spawn_offset_computation};
use crate::Reader;

pub(super) fn handle_text_align_toggle(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    cfg: &mut AppConfig,
    cell: &Rc<Cell<bool>>,
) -> bool {
    let toggle = cell.replace(false);
    if !toggle {
        return false;
    }
    cfg.text_justify = !cfg.text_justify;
    st.text_justify = cfg.text_justify;
    reader.set_text_justify(cfg.text_justify);
    save_settings_for(&st.current_book_path, cfg, st.picker_active);
    reflow(st, reader, cmd_tx);
    true
}

fn reflow(st: &mut LoopState, reader: &Reader, cmd_tx: &std::sync::mpsc::Sender<Cmd>) {
    let anchor = reader.get_cur_start().max(1) as usize;
    let cc = st.current_chapter;
    let Some(chapter) = st.chapters.get_mut(cc) else {
        return;
    };
    st.state = build_state(chapter, st.body_px, st.head_px, st.line_h, st.text_justify);
    st.text_cache.fill(Rgb565Pixel(0xFFFF));
    st.current_page = st
        .state
        .pages
        .iter()
        .enumerate()
        .find(|(_, (rs, re))| {
            st.state
                .all_rows
                .get(*rs..*re)
                .unwrap_or(&[])
                .iter()
                .any(|r| r.start as usize <= anchor && r.end as usize > anchor)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);
    let layout = crate::rendering::layout::screen_layout();
    st.chapter_offsets.clone_from(&estimate_chapter_offsets(
        &st.chapters,
        (cc, st.state.pages.len()),
        st.line_h,
        &layout,
    ));
    st.offset_rx = Some(spawn_offset_computation(
        st.chapters.clone(),
        st.body_px,
        st.line_h,
        st.body_px as i32,
        st.current_book_path.clone(),
        layout,
    ));
    apply_page(reader, &st.state, st.current_page, &st.chapter_offsets, cc);
    if let Some(row) = st
        .state
        .all_rows
        .iter()
        .find(|r| r.start as usize <= anchor && r.end as usize > anchor && r.start < r.end)
    {
        reader.set_cur_start(row.start);
        reader.set_cur_end(row.end);
    }
    let Some(&offset) = st.chapter_offsets.get(cc) else {
        return;
    };
    reader.set_saved_page((offset + st.current_page) as i32);
    let utts = page_utterances(st.current_page, &st.state);
    best_effort_send(cmd_tx, Cmd::Reload(utts.clone()));
    if reader.get_playing() {
        let utt_idx = utterance_index_for_offset(&utts, anchor);
        best_effort_send(cmd_tx, Cmd::Seek(utt_idx));
    } else {
        best_effort_send(cmd_tx, Cmd::Seek(0));
    }
}
