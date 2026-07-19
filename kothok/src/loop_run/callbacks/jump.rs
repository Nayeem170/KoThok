// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use log::debug;

use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::loop_state::LoopState;
use crate::Reader;

use super::super::{
    apply_page, best_effort_send, build_state, load_page_audio, set_chapter_name, switch_chapter,
    ChapterSwitchOpts, LoopContext, ViewMode,
};
use super::bookmark::{page_for_bookmark, restore_cursor_line};

pub(super) fn handle_jump_to_reading(
    st: &mut LoopState,
    reader: &Reader,
    cb: &Callbacks,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    ctx: &mut LoopContext,
) {
    if !cb.jump_to_reading_cell.replace(false) || st.picker_active {
        return;
    }
    st.panel_open = false;
    cb.panel_open_cell.set(false);
    reader.set_panel_open(false);

    if matches!(st.view_mode, ViewMode::Audio) {
        jump_audio_bookmark(st, reader, cmd_tx, ctx);
    } else if st.reading_ch < st.chapters.len() {
        jump_reading_position(st, reader, cmd_tx, ctx);
    }
}

fn jump_audio_bookmark(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    ctx: &mut LoopContext,
) {
    let Some(bm) = st.bookmark.filter(|b| b.chapter < st.chapters.len()) else {
        return;
    };
    if bm.chapter != st.current_chapter {
        switch_chapter(
            st,
            reader,
            cmd_tx,
            bm.chapter,
            ChapterSwitchOpts {
                to_last_page: false,
                update_cursor: false,
                load_audio: true,
            },
        );
    }
    st.current_page = page_for_bookmark(st, &bm);
    apply_page(
        reader,
        &st.state,
        st.current_page,
        &st.chapter_offsets,
        st.current_chapter,
    );
    let restored = restore_cursor_line(st, reader, bm.offset);
    let base = st
        .chapter_offsets
        .get(st.current_chapter)
        .copied()
        .unwrap_or(0);
    reader.set_saved_page((base + st.current_page) as i32);
    let utts = crate::audio::glue::page_utterances(st.current_page, &st.state);
    let target = crate::audio::glue::utterance_index_for_offset(&utts, bm.offset);
    best_effort_send(cmd_tx, Cmd::Reload(utts));
    best_effort_send(cmd_tx, Cmd::Seek(target));
    st.text_dirty = true;
    ctx.window.request_redraw();
    debug!(
        "panel-jump-bookmark: ch={} pg={} off={} line_restored={}",
        bm.chapter + 1,
        st.current_page + 1,
        bm.offset,
        restored
    );
}

fn jump_reading_position(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    ctx: &mut LoopContext,
) {
    if st.reading_ch != st.current_chapter {
        st.current_chapter = st.reading_ch;
        st.state = build_state(
            &mut st.chapters[st.reading_ch],
            st.body_px,
            st.head_px,
            st.line_h,
        );
        let cn =
            crate::data::library::chapter_display_title(&st.chapters[st.reading_ch], st.reading_ch);
        set_chapter_name(reader, &cn);
        best_effort_send(cmd_tx, Cmd::Reload(st.state.utterances.clone()));
    }
    if st.reading_off > 0 {
        st.current_page = st
            .state
            .pages
            .iter()
            .enumerate()
            .find(|(_, (rs, re))| {
                st.state.all_rows[*rs..*re]
                    .iter()
                    .any(|r| r.start as usize <= st.reading_off && r.end as usize > st.reading_off)
            })
            .map(|(i, _)| i)
            .unwrap_or(st.reading_pg);
    } else {
        st.current_page = st.reading_pg;
    }
    st.current_page = st.current_page.min(st.state.pages.len().saturating_sub(1));
    apply_page(
        reader,
        &st.state,
        st.current_page,
        &st.chapter_offsets,
        st.current_chapter,
    );
    if st.reading_off > 0 {
        reader.set_cur_start(st.reading_off as i32);
        reader.set_cur_end(st.reading_end as i32);
    }
    let base = st
        .chapter_offsets
        .get(st.current_chapter)
        .copied()
        .unwrap_or(0);
    reader.set_saved_page((base + st.current_page) as i32);
    load_page_audio(st.current_page, &st.state, cmd_tx);
    st.text_dirty = true;
    ctx.window.request_redraw();
    debug!(
        "jump-to-reading: ch={} pg={} off={}",
        st.reading_ch + 1,
        st.current_page + 1,
        st.reading_off
    );
}
