// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::cell::Cell;
use std::sync::mpsc::{Receiver, Sender};

use log::{debug, info, warn};

use slint::{ModelRc, SharedString, VecModel};

use crate::audio::glue::{best_effort_send, load_page_audio};
use crate::audio::{Cmd, Event};
use crate::loop_state::LoopState;
use crate::reader::{apply_page, switch_chapter, ChapterSwitchOpts};
use crate::Reader;

pub use kobo_core::rendering::layout::resolve_progress_target;

use super::*;

pub fn process_audio_events(
    st: &mut LoopState,
    evt_rx: &Receiver<Event>,
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
) -> AudioFlags {
    let mut ui_changed = false;
    let page_changed = false;
    let mut text_dirty = false;
    while let Ok(ev) = evt_rx.try_recv() {
        match ev {
            Event::Playing => {
                reader.set_playing(true);
                reader.set_paused(false);
                // Clear any idle hint / notice now that playback is underway; the
                // sentence band takes over the status line.
                reader.set_status("".into());
                ui_changed = true;
            }
            Event::Paused => {
                reader.set_playing(false);
                reader.set_paused(true);
                reader.set_status("".into());
                ui_changed = true;
            }
            Event::Stopped => {
                reader.set_playing(false);
                reader.set_paused(false);
                reader.set_status("".into());
                // NOTE: do NOT clear cur_start/cur_end here. The reading cursor is
                // a persistent position, not playback state - wiping it on Stop
                // raced the book-open restore (Cmd::Stop -> Stopped arrives after
                // the saved cursor was set), leaving the page with no cursor and
                // making Play resume from the page top instead of the saved line.
                ui_changed = true;
            }
            Event::Ended => {
                // Auto page-turn on TTS completion: keep text_dirty + ui_changed
                // but NOT page_changed, so this is a PARTIAL refresh - matching
                // the flicker-free swipe page-turn (no full GC16 black flash).
                let (td, uc) = handle_audio_ended(st, reader, cmd_tx);
                text_dirty |= td;
                ui_changed |= uc;
            }
            Event::Sentence { start, end } => {
                debug!(
                    "cursor: Event::Sentence start={start} end={end} page={} prev_cs={} prev_ce={}",
                    st.current_page,
                    reader.get_cur_start(),
                    reader.get_cur_end()
                );
                reader.set_cur_start(start as i32);
                reader.set_cur_end(end as i32);
                if let Some((pg_start, pg_end)) = st.state.pages.get(st.current_page) {
                    let mut found_row = -1i32;
                    for (ri, row) in st.state.all_rows[*pg_start..*pg_end].iter().enumerate() {
                        if row.start < row.end
                            && start >= row.start as usize
                            && start < row.end as usize
                        {
                            found_row = (*pg_start + ri) as i32;
                            let txt = st
                                .state
                                .utterances
                                .iter()
                                .find(|u| u.start <= start && start < u.end)
                                .map(|u| u.text.as_str());
                            if let Some(txt) = txt {
                                reader.set_current_sentence(SharedString::from(txt));
                            }
                            break;
                        }
                    }
                    debug!("cursor: matched row_idx={found_row} for start={start} on page {} (rows {}-{})",
                        st.current_page, pg_start, pg_end);
                }
                ui_changed = true;
            }
            Event::PageBreak => {
                if st.current_page + 1 < st.state.pages.len() {
                    st.current_page += 1;
                    apply_page(
                        reader,
                        &st.state,
                        st.current_page,
                        &st.chapter_offsets,
                        st.current_chapter,
                    );
                    reader.set_saved_page(
                        (st.chapter_offsets[st.current_chapter] + st.current_page) as i32,
                    );
                    text_dirty = true;
                    if !matches!(st.view_mode, crate::ViewMode::Audio) {
                        let next_utts =
                            crate::audio::glue::page_utterances(st.current_page, &st.state);
                        best_effort_send(cmd_tx, Cmd::Append(next_utts));
                    }
                    info!(
                        "page-break: visual page advanced to {}",
                        st.current_page + 1
                    );
                }
            }
            Event::Error(m) => {
                // The driver has stopped (want_play=false). Reflect that in the
                // transport, otherwise the play/pause button keeps showing
                // "playing" over dead audio and the reader looks frozen.
                reader.set_playing(false);
                reader.set_paused(true);
                reader.set_status(friendly_error(&m).into());
                warn!("audio error: {m}");
                ui_changed = true;
            }
        }
    }
    AudioFlags {
        ui_changed,
        page_changed,
        text_dirty,
    }
}

fn handle_audio_ended(st: &mut LoopState, reader: &Reader, cmd_tx: &Sender<Cmd>) -> (bool, bool) {
    let mut text_dirty = false;

    if matches!(st.view_mode, crate::ViewMode::Audio) {
        if st.current_page + 1 < st.state.pages.len() {
            st.current_page += 1;
            apply_page(
                reader,
                &st.state,
                st.current_page,
                &st.chapter_offsets,
                st.current_chapter,
            );
            reader
                .set_saved_page((st.chapter_offsets[st.current_chapter] + st.current_page) as i32);
            text_dirty = true;
            best_effort_send(cmd_tx, Cmd::Play);
            info!("audio-page: advanced to page {}", st.current_page + 1);
        } else if st.current_chapter + 1 < st.chapter_count {
            let nc = st.current_chapter + 1;
            switch_chapter(
                st,
                reader,
                cmd_tx,
                nc,
                ChapterSwitchOpts {
                    to_last_page: false,
                    update_cursor: true,
                    load_audio: false,
                },
            );
            text_dirty = true;
            crate::audio::glue::load_chapter_audio(&st.state, cmd_tx);
            best_effort_send(cmd_tx, Cmd::Play);
            reader.set_playing(true);
            reader.set_paused(false);
            reader.set_status("".into());
            reader
                .set_saved_page((st.chapter_offsets[st.current_chapter] + st.current_page) as i32);
            info!("audio-chapter: advanced to chapter {}", nc + 1);
        } else {
            reader.set_playing(false);
            reader.set_paused(false);
            reader.set_status("Book complete".into());
            reader.set_cur_start(0);
            reader.set_cur_end(0);
        }
        return (text_dirty, true);
    }

    if st.current_page + 1 < st.state.pages.len() {
        st.current_page += 1;
        apply_page(
            reader,
            &st.state,
            st.current_page,
            &st.chapter_offsets,
            st.current_chapter,
        );
        reader.set_saved_page((st.chapter_offsets[st.current_chapter] + st.current_page) as i32);
        text_dirty = true;
        let next_utts = crate::audio::glue::page_utterances(st.current_page, &st.state);
        best_effort_send(cmd_tx, Cmd::Append(next_utts));
        best_effort_send(cmd_tx, Cmd::Play);
        info!(
            "auto-page: {}/{} -> {}/{} (page audio ended)",
            st.current_page,
            st.state.pages.len(),
            st.current_page + 1,
            st.state.pages.len()
        );
    } else if st.current_chapter + 1 < st.chapter_count {
        let nc = st.current_chapter + 1;
        switch_chapter(
            st,
            reader,
            cmd_tx,
            nc,
            ChapterSwitchOpts {
                to_last_page: false,
                update_cursor: true,
                load_audio: false,
            },
        );
        text_dirty = true;
        let page_utts = crate::audio::glue::page_utterances(st.current_page, &st.state);
        best_effort_send(cmd_tx, Cmd::Reload(page_utts));
        best_effort_send(cmd_tx, Cmd::Seek(0));
        best_effort_send(cmd_tx, Cmd::Play);
        reader.set_playing(true);
        reader.set_paused(false);
        reader.set_status("".into());
        reader.set_saved_page((st.chapter_offsets[st.current_chapter] + st.current_page) as i32);
    } else {
        reader.set_playing(false);
        reader.set_paused(false);
        reader.set_status("Book complete".into());
        reader.set_cur_start(0);
        reader.set_cur_end(0);
    }
    (text_dirty, true)
}

pub fn process_page_navigation(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    page_delta: &Cell<i32>,
    progress_target: &Cell<i32>,
) -> (bool, bool) {
    // Manual navigation (swipe or progress-bar drag) is BROWSING: it changes
    // the displayed page (knob) only. It must NOT move the cursor or the
    // reading-page tick, and if audio is playing it pauses. switch_chapter
    // sets the cursor internally, so capture + restore it around navigation.
    let was_playing = reader.get_playing();
    let prev_cur_start = reader.get_cur_start();
    let prev_cur_end = reader.get_cur_end();

    let (mut text_dirty, mut ui_changed, mut navigated) = {
        let o = apply_page_delta(st, reader, cmd_tx, page_delta);
        (o.text_dirty, o.ui_changed, o.navigated)
    };
    let o = apply_progress_target(st, reader, cmd_tx, progress_target);
    text_dirty |= o.text_dirty;
    ui_changed |= o.ui_changed;
    navigated |= o.navigated;

    // Browsing never moves the cursor or tick: undo switch_chapter's cursor
    // set, and pause if we were playing.
    if navigated {
        reader.set_cur_start(prev_cur_start);
        reader.set_cur_end(prev_cur_end);
        if was_playing {
            reader.set_playing(false);
            reader.set_paused(true);
            // best-effort: channel may be full
            let _ = cmd_tx.send(Cmd::Pause);
            ui_changed = true;
        }
    }

    (text_dirty, ui_changed)
}

pub(super) struct NavOutcome {
    pub(super) navigated: bool,
    pub(super) text_dirty: bool,
    pub(super) ui_changed: bool,
}

fn switch_to(st: &mut LoopState, reader: &Reader, cmd_tx: &Sender<Cmd>, nc: usize, to_last: bool) {
    switch_chapter(
        st,
        reader,
        cmd_tx,
        nc,
        ChapterSwitchOpts {
            to_last_page: to_last,
            update_cursor: false,
            load_audio: true,
        },
    );
}

fn apply_page_display(st: &LoopState, reader: &Reader, cmd_tx: &Sender<Cmd>) {
    let (s, e) = st
        .state
        .pages
        .get(st.current_page)
        .copied()
        .unwrap_or((0, 0));
    let rows = st.state.all_rows.get(s..e).unwrap_or(&[]).to_vec();
    reader.set_rows(ModelRc::new(VecModel::from(rows)));
    reader.set_page((st.chapter_offsets[st.current_chapter] + st.current_page) as i32);
    reader.set_page_count(*st.chapter_offsets.last().unwrap_or(&1) as i32);
    load_page_audio(st.current_page, &st.state, cmd_tx);
}

fn apply_page_delta(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    page_delta: &Cell<i32>,
) -> NavOutcome {
    let mut o = NavOutcome {
        navigated: false,
        text_dirty: false,
        ui_changed: false,
    };
    let d = page_delta.replace(0);
    if d == 0 {
        return o;
    }
    let last = st.state.pages.len() as i32 - 1;
    let target = st.current_page as i32 + d;
    if target < 0 {
        if st.current_chapter > 0 {
            switch_to(st, reader, cmd_tx, st.current_chapter - 1, true);
            o.navigated = true;
            o.text_dirty = true;
            o.ui_changed = true;
        }
    } else if target > last {
        if st.current_chapter + 1 < st.chapter_count {
            switch_to(st, reader, cmd_tx, st.current_chapter + 1, false);
            o.navigated = true;
            o.text_dirty = true;
            o.ui_changed = true;
        }
    } else if target as usize != st.current_page {
        st.current_page = target as usize;
        o.navigated = true;
        o.text_dirty = true;
        apply_page_display(st, reader, cmd_tx);
        o.ui_changed = true;
    }
    o
}

fn apply_progress_target(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    progress_target: &Cell<i32>,
) -> NavOutcome {
    let mut o = NavOutcome {
        navigated: false,
        text_dirty: false,
        ui_changed: false,
    };
    let pt_val = progress_target.replace(-1);
    if pt_val < 0 || st.picker_active {
        return o;
    }
    let (c, local) = resolve_progress_target(pt_val, &st.chapter_offsets, st.chapter_count);
    if c != st.current_chapter {
        switch_to(st, reader, cmd_tx, c, false);
        o.navigated = true;
    }
    let lp = local.min(st.state.pages.len().saturating_sub(1));
    if lp != st.current_page {
        st.current_page = lp;
        o.navigated = true;
        apply_page_display(st, reader, cmd_tx);
    }
    o.text_dirty = true;
    o.ui_changed = true;
    o
}
