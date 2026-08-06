// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;
use crate::Reader;

mod audio;
pub(super) mod bookmark;
mod jump;
mod mode_toggle;
mod navigation;

/// Retract the reading header once its reveal has expired, or as soon as
/// playback starts.
///
/// Playback wins over the countdown: read-aloud is the case where the screen is
/// being looked at rather than touched, so the header is pure furniture there.
/// The header stays put in the library, behind the panel and behind the chapter
/// overlay -- those either do not draw it or are mid-interaction.
fn retract_header_if_due(st: &mut LoopState, ctx: &LoopContext) -> bool {
    let reader = ctx.reader;
    if !ctx.cfg.auto_hide_header || st.picker_active || st.panel_open {
        return false;
    }
    if !st.header_visible {
        return false;
    }
    let expired = st
        .header_revealed_at
        .is_none_or(|t| t.elapsed().as_secs() >= HEADER_REVEAL_SECS);
    if !(reader.get_playing() || expired) {
        return false;
    }
    st.header_visible = false;
    st.header_revealed_at = None;
    reader.set_header_visible(false);
    st.text_dirty = true;
    true
}

pub(super) fn process_loop_callbacks(st: &mut LoopState, ctx: &mut LoopContext) -> (bool, bool) {
    let reader = ctx.reader;
    let cb = ctx.cb;
    let cmd_tx = ctx.cmd_tx;
    let mut ui_changed = false;
    let mut page_changed = false;
    // Translate one-shot AVRCP media-button signals from the BT monitor into the
    // same cells the on-screen buttons use.
    let sig = ctx.media_signals;
    if sig.play.swap(false, std::sync::atomic::Ordering::SeqCst) {
        cb.play_toggle_cell.set(true);
    }
    if sig.next.swap(false, std::sync::atomic::Ordering::SeqCst) {
        cb.skip_forward_cell.set(true);
    }
    if sig.prev.swap(false, std::sync::atomic::Ordering::SeqCst) {
        cb.skip_rewind_cell.set(true);
    }

    if let Some(t) = st.pending_tap_at {
        if st.panel_open
            || st.picker_active
            || t.elapsed().as_millis() >= touch::DOUBLE_TAP_WINDOW_MS
        {
            st.pending_tap_at = None;
        }
    }
    // Resolve a deferred single footer play-button tap: once the double-click
    // window elapses with no second tap, promote it to a real toggle. Opening
    // the panel cancels it so no play/pause fires behind the overlay.
    if let Some(t) = st.pp_pending_release {
        if st.panel_open || t.elapsed().as_millis() >= PLAY_BUTTON_DOUBLE_MS as u128 {
            st.pp_pending_release = None;
            if !st.panel_open {
                cb.play_toggle_cell.set(true);
            }
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

    let panel = process_panel_callbacks(st, reader, cmd_tx, ctx.cfg, ctx.fl_path, cb);
    st.text_dirty |= panel.text_dirty;
    ui_changed |= panel.ui_changed;

    ui_changed |= retract_header_if_due(st, ctx);

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

    ui_changed |= mode_toggle::process_mode_toggle(st, reader, cb, cmd_tx);

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
            .map(|bm| {
                // A font-size change repaginates the loaded chapter, so the
                // stored page number drifts and the seek-bar marker would
                // land away from the reading cursor. Derive the page from the
                // stable offset when the bookmark is in the loaded chapter;
                // other chapters keep the stored estimate (their pagination is
                // not rebuilt until they are opened).
                let page_in_chapter = if bm.chapter == st.current_chapter {
                    bookmark::page_for_bookmark(st, &bm)
                } else {
                    bm.page
                };
                let global =
                    st.chapter_offsets.get(bm.chapter).copied().unwrap_or(0) + page_in_chapter;
                (global as f32 / total).clamp(0.0, 1.0)
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

    ui_changed |= bookmark::handle_bookmark_jump(st, reader, cb, cmd_tx, ctx);

    navigation::handle_skip_forward(st, reader, cb, cmd_tx);
    navigation::handle_skip_rewind(st, reader, cb, cmd_tx);

    if cb.play_toggle_cell.replace(false) && !st.picker_active && !st.current_book_path.is_empty() {
        if matches!(st.view_mode, ViewMode::Audio) {
            if reader.get_playing() {
                reader.set_playing(false);
                reader.set_paused(true);
                best_effort_send(cmd_tx, Cmd::Pause);
            } else if reader.get_play_enabled() {
                reader.set_playing(true);
                reader.set_paused(false);
                best_effort_send(cmd_tx, Cmd::Play);
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
            if pt.pg != st.current_page {
                info!(
                    "play-start: correcting page {} -> {} (cursor off={})",
                    st.current_page + 1,
                    pt.pg + 1,
                    pt.off
                );
                st.current_page = pt.pg;
                apply_page(
                    reader,
                    &st.state,
                    st.current_page,
                    &st.chapter_offsets,
                    st.current_chapter,
                );
                st.text_dirty = true;
            }
            st.reading_ch = pt.ch;
            st.reading_pg = pt.pg;
            st.reading_off = pt.off;
            st.reading_end = pt.end;
        }
    }

    let overlay_now = reader.get_chapter_overlay_open();
    if overlay_now && !st.prev_chapter_overlay {
        st.chapter_scroll = 0;
        st.search_scroll = 0;
        st.search_results_active = false;
        st.search_results_scroll = 0;
        st.search_word_selected = false;
        let requested = cb.overlay_requested_tab_cell.replace(-1);
        let tab = match requested {
            1 => crate::loop_state::ChapterTab::Words,
            _ => crate::loop_state::ChapterTab::Chapters,
        };
        st.chapter_tab = tab;
        let tab_int = match tab {
            crate::loop_state::ChapterTab::Chapters => 0,
            crate::loop_state::ChapterTab::Words => 1,
        };
        reader.set_chapter_overlay_active_tab(tab_int);
        let (seg_w, gap, font_px) = crate::rendering::tab_bar_geom::tab_bar_geom(ctx.w);
        reader.set_tab_seg_w(seg_w as f32);
        reader.set_tab_gap(gap as f32);
        reader.set_tab_font_px(font_px);
    }

    if overlay_now {
        reader.set_chapter_overlay_results_active(st.search_results_active);
        if st.search_results_active {
            let title = st
                .word_index
                .words
                .get(st.search_selected_word)
                .map(|w| {
                    let n = st
                        .word_index
                        .occurrences
                        .get(st.search_selected_word)
                        .map(|h| h.len())
                        .unwrap_or(0);
                    format!("{w} - {n} matches")
                })
                .unwrap_or_default();
            reader.set_chapter_overlay_results_title(slint::SharedString::from(title));
        }
    }

    if cb.overlay_tab_switch_cell.get() != -1 {
        let tab = cb.overlay_tab_switch_cell.replace(-1);
        match tab {
            0 => {
                st.chapter_tab = crate::loop_state::ChapterTab::Chapters;
                st.chapter_scroll = 0;
                st.search_word_selected = false;
            }
            1 => {
                st.chapter_tab = crate::loop_state::ChapterTab::Words;
                st.search_scroll = 0;
            }
            _ => {}
        }
        st.text_dirty = true;
        ui_changed = true;
    }

    if cb.overlay_back_from_results_cell.replace(false) {
        search::back_from_results(st);
        st.text_dirty = true;
    }

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
