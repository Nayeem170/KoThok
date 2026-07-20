// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;
use crate::Reader;

mod audio;
mod bookmark;
mod jump;
mod mode_toggle;
mod navigation;

pub(super) fn process_loop_callbacks(st: &mut LoopState, ctx: &mut LoopContext) -> (bool, bool) {
    let reader = ctx.reader;
    let cb = ctx.cb;
    let cmd_tx = ctx.cmd_tx;
    let mut ui_changed = false;
    let mut page_changed = false;
    if let Some(t) = st.pending_tap_at {
        if st.panel_open
            || st.picker_active
            || t.elapsed().as_millis() >= touch::DOUBLE_TAP_WINDOW_MS as u128
        {
            st.pending_tap_at = None;
        }
    }
    // Snapshot the page BEFORE both audio-driven and manual page changes.
    // Reading this after process_audio_events hid TTS auto-advance from the
    // status-clear below: a "Bookmarked page 42" footer stayed pinned while
    // the page number underneath silently advanced. Capturing up front lets
    // either kind of turn retire a stale status line.
    let pre_nav_ch = st.current_chapter;
    let pre_nav_pg = st.current_page;
    let af = process_audio_events(st, ctx.evt_rx, reader, cmd_tx);
    ui_changed |= af.ui_changed;
    page_changed |= af.page_changed;
    st.text_dirty |= af.text_dirty;
    if reader.get_playing() && !st.picker_active {
        st.reading_ch = st.current_chapter;
        st.reading_pg = st.current_page;
        let cs = reader.get_cur_start();
        let ce = reader.get_cur_end();
        if cs > 0 {
            st.reading_off = cs as usize;
            st.reading_end = ce as usize;
        }
    }
    let (nav_text, nav_ui) =
        process_page_navigation(st, reader, cmd_tx, &cb.page_delta, &cb.progress_target);
    st.text_dirty |= nav_text;
    ui_changed |= nav_ui;
    if st.current_chapter != pre_nav_ch || st.current_page != pre_nav_pg {
        st.last_nav = std::time::Instant::now();
        // Latched, never cleared for the session: once a page has been turned,
        // the footer shows where you are instead of how to turn one.
        reader.set_has_navigated(true);
        // Turning a page retires whatever the footer was saying.
        //
        // The footer prefers `status` over the page number whenever it is
        // non-empty, and nothing used to clear it -- so "Bookmarked page 42"
        // stayed there for the rest of the session while the page number it
        // was covering silently advanced underneath. Anything status has to
        // report is about where you just were, so a page turn is exactly the
        // moment it stops being true.
        //
        // Cleared on navigation rather than on a timer so that a message costs
        // one footer refresh instead of two: expiring it would force a second
        // e-ink update a few seconds after every message, for no new
        // information.
        if !reader.get_status().is_empty() {
            reader.set_status(Default::default());
            ui_changed = true;
        }
    }
    // `last_nav` starts at launch, so this alone would also fire for the first
    // three seconds of every session, before any page had been turned.
    reader.set_nav_recent(st.last_nav.elapsed().as_secs() < 3);

    st.text_dirty |= process_panel_callbacks(st, reader, cmd_tx, ctx.cfg, ctx.fl_path, cb);

    if cb.lock_tap_cell.replace(false)
        && !st.picker_active
        && matches!(st.view_mode, ViewMode::Audio)
    {
        st.system_state = SystemState::Locked;
        st.lock_time = Some(std::time::Instant::now());
        reader.set_audio_locked(true);
        if st.panel_open {
            st.panel_open = false;
            cb.panel_open_cell.set(false);
            reader.set_panel_open(false);
        }
        power::lock_frontlight_off(st, ctx);
        power::lock_radios(st, ctx);
        info!("lock-tap: display locked");
    }

    let mt = {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            mode_toggle::process_mode_toggle(st, reader, cb, cmd_tx)
        }));
        match result {
            Ok(v) => v,
            Err(payload) => {
                let msg = payload
                    .downcast_ref::<&str>()
                    .copied()
                    .or_else(|| payload.downcast_ref::<String>().map(|s| s.as_str()))
                    .unwrap_or("<non-string>");
                log::error!("PANIC caught in process_mode_toggle: {msg}");
                false
            }
        }
    };
    ui_changed |= mt;

    if cb.settings_cell.replace(false) && !st.picker_active && !st.panel_open {
        st.panel_open = true;
        cb.panel_open_cell.set(true);
        reader.set_panel_open(true);
        reader.set_battery_pct(ctx.caps.battery_pct());
        reader.set_clock(SharedString::from(ctx.caps.current_clock()));
        reader.set_sleep_label(
            crate::panel::callbacks::sleep::sleep_label(ctx.cfg.reading_auto_sleep_secs).into(),
        );
        if let Some(ref path) = ctx.fl_path {
            if let Some(hw) = frontlight_get(path) {
                reader.set_brightness_val(hw as i32);
            }
        }
        if reader.get_playing() {
            reader.set_playing(false);
            reader.set_paused(true);
            best_effort_send(cmd_tx, Cmd::Pause);
        }
        st.text_dirty = true;
        ui_changed = true;
        info!("audio: panel OPEN (gear tap)");
    }

    ui_changed |= bookmark::handle_bookmark_set(st, reader, cb);

    {
        let total = *st.chapter_offsets.last().unwrap_or(&1).max(&1) as f32;
        let frac = st
            .bookmark
            .and_then(|bm| {
                let global = st.chapter_offsets.get(bm.chapter).copied().unwrap_or(0) + bm.page;
                Some((global as f32 / total).clamp(0.0, 1.0))
            })
            .unwrap_or(-1.0);
        reader.set_bookmark_frac(frac);
        reader.set_has_bookmark(st.bookmark.is_some());
    }

    {
        let npages = st.state.pages.len().max(1) as f32;
        let chapter_frac = (st.current_page as f32 / npages).clamp(0.0, 1.0);
        reader.set_chapter_progress(chapter_frac);

        let total_pages = *st.chapter_offsets.last().unwrap_or(&1).max(&1) as f32;
        let global_page = st
            .chapter_offsets
            .get(st.current_chapter)
            .copied()
            .unwrap_or(0)
            + st.current_page;
        let book_frac = (global_page as f32 / total_pages).clamp(0.0, 1.0);
        reader.set_book_progress(book_frac);

        if matches!(st.view_mode, crate::ViewMode::Audio) {
            ui_changed |= audio::advance_cover_rotation(st, reader);
            audio::refresh_audio_disk(st, reader, chapter_frac, ctx.w);
        }
    }

    ui_changed |= bookmark::handle_bookmark_jump(st, reader, cb, cmd_tx);

    navigation::handle_skip_forward(st, reader, cb, cmd_tx);
    navigation::handle_skip_rewind(st, reader, cb, cmd_tx);

    if cb.play_toggle_cell.replace(false) && !st.picker_active && !st.current_book_path.is_empty() {
        if matches!(st.view_mode, ViewMode::Audio) {
            if reader.get_playing() {
                reader.set_playing(false);
                reader.set_paused(true);
                best_effort_send(cmd_tx, Cmd::Pause);
                debug!("play-pause: audio paused");
            } else if reader.get_play_enabled() {
                reader.set_playing(true);
                reader.set_paused(false);
                best_effort_send(cmd_tx, Cmd::Play);
                debug!("play-pause: audio resumed");
            }
            st.reading_ch = st.current_chapter;
            st.reading_pg = st.current_page;
        } else {
            let pt = toggle_playback(
                reader,
                cmd_tx,
                &st.state,
                st.current_page,
                &st.chapter_offsets,
                st.current_chapter,
            );
            st.reading_ch = pt.ch;
            st.reading_pg = pt.pg;
            st.reading_off = pt.off;
            st.reading_end = pt.end;
            debug!("play-pause: button toggled");
        }
    }

    let overlay_now = reader.get_chapter_overlay_open();
    if overlay_now && !st.prev_chapter_overlay {
        st.chapter_scroll = 0;
    }
    st.prev_chapter_overlay = overlay_now;

    jump::handle_jump_to_reading(st, reader, cb, cmd_tx, ctx);

    if let Some(rx) = st.font_download_rx.take() {
        match rx.try_recv() {
            Ok(result) => {
                if result.ok {
                    info!("font-dl: {:?} installed, re-rendering", result.script);
                    reader.set_status(Default::default());
                    st.text_dirty = true;
                    ui_changed = true;
                } else {
                    reader.set_status("Font download failed".into());
                    ui_changed = true;
                }
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => {
                st.font_download_rx = Some(rx);
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {}
        }
    }

    (ui_changed, page_changed)
}
