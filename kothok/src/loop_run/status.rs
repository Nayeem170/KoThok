// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;
use crate::Reader;

pub(super) fn sync_panel_close(st: &mut LoopState, ctx: &LoopContext, _msg: &str) {
    if st.panel_open && !ctx.cb.panel_open_cell.get() {
        st.panel_open = false;
        ctx.reader.set_panel_open(false);
        st.text_dirty = true;
    }
}

/// Shortest gap between two position writes while reading.
///
/// The cursor moves once per spoken sentence, so saving on every change would
/// rewrite the positions file every few seconds for the whole session -- the
/// write is a full read-modify-rewrite, and this is eMMC. Losing at most this
/// much progress to a flat battery is the trade.
const POSITION_AUTOSAVE_SECS: u64 = 15;

/// The position as it stands right now.
fn current_position(st: &LoopState, reader: &Reader) -> persistence::ReadingPosition {
    persistence::ReadingPosition {
        chapter: st.current_chapter,
        page: st.current_page,
        cur_start: reader.get_cur_start().max(0) as usize,
        cur_end: reader.get_cur_end().max(0) as usize,
        view_mode: st.view_mode,
        bookmark: st.bookmark,
        progress: reader.get_book_progress(),
    }
}

/// Write the reading position now, unconditionally. For the moments where
/// losing it is not acceptable: leaving the book, leaving the app, sleeping.
pub(crate) fn save_position_now(st: &mut LoopState, reader: &Reader) {
    if st.current_book_path.is_empty() {
        return;
    }
    let pos = current_position(st, reader);
    if let Err(e) = save_position(
        std::path::Path::new(POSITIONS_FILE),
        &st.current_book_path,
        &pos,
    ) {
        log::error!("save_position_now: {e}");
    }
    st.saved_pos = Some((pos.chapter, pos.page, pos.cur_start));
    st.saved_pos_at = Some(std::time::Instant::now());
}

/// Write the reading position if it has actually moved and the rate limit has
/// elapsed. Called every loop iteration; almost always a no-op.
///
/// Without this, a crash or a flat battery lost the whole session, because the
/// only writes were on the two clean exits.
pub(super) fn autosave_position(st: &mut LoopState, ctx: &LoopContext) {
    if st.picker_active || st.current_book_path.is_empty() {
        return;
    }
    let reader = ctx.reader;
    let now = (
        st.current_chapter,
        st.current_page,
        reader.get_cur_start().max(0) as usize,
    );
    if st.saved_pos == Some(now) {
        return;
    }
    // First move of a session establishes the baseline without a write; the
    // rate limit then governs from there.
    let due = match st.saved_pos_at {
        Some(t) => t.elapsed().as_secs() >= POSITION_AUTOSAVE_SECS,
        None => true,
    };
    if !due {
        return;
    }
    save_position_now(st, reader);
    save_marks_if_dirty(st);
}

fn save_marks_if_dirty(st: &mut LoopState) {
    save_marks_if_dirty_at(
        &mut st.marks_dirty,
        &st.current_book_path,
        &st.marks,
        crate::data::persistence::marks_path(),
    );
}

pub(crate) fn save_marks_if_dirty_at(
    dirty: &mut bool,
    book_path: &str,
    marks: &[crate::data::mark::Mark],
    marks_file: &std::path::Path,
) {
    if !*dirty || book_path.is_empty() {
        return;
    }
    match crate::data::persistence::save_marks(marks_file, book_path, marks) {
        Ok(()) => *dirty = false,
        Err(e) => log::error!("save_marks_if_dirty_at: {e}"),
    }
}

pub(super) fn handle_exit_button(st: &mut LoopState, ctx: &LoopContext) -> LoopFlow {
    if ctx.cb.exit_app.get() {
        if !st.picker_active {
            save_position_now(st, ctx.reader);
        }
        best_effort_send(&ctx.cmd_tx, Cmd::Stop);
        info!("EXIT: leaving app to nickel");
        return LoopFlow::Break;
    }
    LoopFlow::Normal
}

pub(super) fn handle_quit_button(st: &mut LoopState, ctx: &mut LoopContext) -> LoopFlow {
    let reader = ctx.reader;
    let cb = ctx.cb;
    if cb.quit.get() {
        if st.panel_open {
            cb.panel_open_cell.set(false);
            reader.set_panel_open(false);
        }
        if st.picker_active {
            return LoopFlow::Break;
        }
        save_position_now(st, reader);
        // Leaving the book clears the baseline: the next book opened must not
        // inherit this one's saved tuple and skip its own first autosave.
        st.saved_pos = None;
        st.saved_pos_at = None;
        best_effort_send(&ctx.cmd_tx, Cmd::Stop);
        reader.set_playing(false);
        reader.set_paused(false);
        reader.set_cur_start(0);
        reader.set_cur_end(0);
        st.cover_page_visible = false;
        st.tap_xy = None;
        cb.quit.set(false);
        st.text_dirty = true;
        st.picker_scroll = 0;
        if !st.current_book_path.is_empty() {
            if let Some(pos) = ctx
                .all_books
                .iter()
                .position(|b| b.path == st.current_book_path)
            {
                if pos != 0 {
                    ctx.all_books.swap(0, pos);
                }
            }
            if let Some(b) = ctx.all_books.first_mut() {
                if b.progress <= 0.005 {
                    b.progress = 0.01;
                }
            }
        }
        show_book_picker(
            reader,
            ctx.fb,
            ctx.window,
            &mut st.buffer,
            &mut st.text_cache,
            &mut st.picker_cover_cache,
            ctx.all_books,
            st.picker_scroll,
            st.library_filter,
            &ctx.caps.current_clock(),
            ctx.caps.battery_pct(),
            if st.exit_armed {
                "Double-tap to Exit"
            } else {
                ""
            },
            // Returning from a book: the whole screen was the reader a
            // moment ago, so repaint all of it -- but quietly. GC16 partial
            // clears the reader well enough; the inverting flash is reserved
            // for sleep/wake.
            PickerRefresh::FullQuiet,
        );
        st.picker_active = true;
        st.panel_open = false;
        reader.set_panel_open(false);
        {
            let device_font = (ctx.w as i32 / 38).clamp(20, 60);
            *ctx.cfg = load_config_from_base(CONFIG_FILE, device_font);
            push_book_settings_to_ui(reader, ctx.cfg);
        }
        st.picker_entered = Some(std::time::Instant::now());
        st.picker_cells = picker_scroll_cells(ctx.all_books, st.picker_scroll, st.library_filter);
        st.prev_buffer.copy_from_slice(&st.buffer);
        return LoopFlow::Continue;
    }
    LoopFlow::Normal
}

pub(super) fn refresh_status(st: &mut LoopState, ctx: &LoopContext) {
    match crate::device::bt_take_reconnect_result() {
        1 => {
            ctx.reader.set_bt_on(true);
            st.bt_fail_count = 0;
            st.text_dirty = true;
        }
        2 => {
            ctx.reader.set_bt_on(false);
            st.text_dirty = true;
        }
        _ => {}
    }

    if st.last_status_refresh.elapsed().as_millis() as u64 >= STATUS_REFRESH_MS {
        st.last_status_refresh = std::time::Instant::now();
        let wifi = ctx.caps.network_available();
        let prev_wifi = ctx.reader.get_wifi_on();
        crate::crash_report::try_upload_if_wifi();
        crate::debug_log::try_upload_if_wifi();
        let bt = ctx.caps.audio_sink_available();
        let prev_bt = ctx.reader.get_bt_on();
        if wifi != prev_wifi {
            crate::debug_log::log(&format!(
                "wifi: status changed {} -> {} (toggle_age={}ms)",
                prev_wifi,
                wifi,
                crate::device::wifi_toggle_age_ms()
            ));
        }
        if bt != prev_bt {
            crate::debug_log::log(&format!(
                "bt: status changed {} -> {} (reconnect_busy={})",
                prev_bt,
                bt,
                crate::device::bt_reconnect_busy()
            ));
        }
        if crate::device::wifi_toggle_age_ms() >= WIFI_TOGGLE_GRACE_MS {
            ctx.reader.set_wifi_on(wifi);
        }
        if ctx.reader.get_wifi_on() && !st.voice_fetch_attempted {
            st.voice_fetch_attempted = true;
            st.voice_rx = Some(crate::panel::spawn_voice_fetch());
            info!("fetching voice list from Edge");
        }
        // Skip while a reconnect thread is actively retrying: without this,
        // this 3s tick overwrites `bt_on` from live status mid-connect,
        // flickering the pill back to "off" while a real Device1.Connect
        // handshake (which can take far longer than any fixed grace window)
        // is still in flight.
        if !crate::device::bt_reconnect_busy() {
            if bt {
                st.bt_fail_count = 0;
                ctx.reader.set_bt_on(true);
            } else {
                st.bt_fail_count = st.bt_fail_count.saturating_add(1);
                if st.bt_fail_count >= 2 {
                    ctx.reader.set_bt_on(false);
                }
            }
        }
        if wifi && st.wifi_list.is_empty() {
            st.wifi_list_fetched = false;
        }
        // Key off the UI's on-state, not `bt` (== `bt_status()`, which reports
        // *connected*, not *powered*). Gating a re-fetch on being connected
        // deadlocks: the list is what you connect *from*, so an empty list could
        // never refill once it had been cached against a powered-down adapter.
        if ctx.reader.get_bt_on() && st.bt_list.is_empty() {
            st.bt_list_fetched = false;
        }
        if let Some(n) = ctx.caps.wifi_name() {
            ctx.reader.set_wifi_connected_name(SharedString::from(n));
        }
        if let Some(n) = ctx.caps.bt_name() {
            ctx.reader.set_bt_connected_name(SharedString::from(n));
        }
        ctx.reader
            .set_play_enabled(ctx.reader.get_wifi_on() && ctx.reader.get_bt_on());
    }
}

pub(super) fn poll_voice_rx(st: &mut LoopState) {
    if let Some(rx) = &st.voice_rx {
        if let Ok(voices) = rx.try_recv() {
            let count = voices.len();
            crate::panel::save_voice_cache(&voices);
            crate::panel::set_dynamic_voices(voices);
            st.voice_rx = None;
            info!("voice list updated: {count} voices from Edge");
        }
    }
}

pub(super) fn poll_offset_rx(st: &mut LoopState, ctx: &LoopContext) {
    if let Some(ref comp) = st.offset_rx {
        while let Ok(pct) = comp.pct_rx.try_recv() {
            ctx.reader.set_loading_pct(pct);
            // In audio mode the loading bar sits under the spinning disk, and
            // audio renders force a heavy (GC16) refresh -- one per pct tick
            // reads as the disk blinking. The disk is already on screen, so let
            // the progress value update silently and skip the per-tick repaint;
            // the final state is presented once loading completes below. Reading
            // mode has no disk to flash, so it keeps the live progress bar.
            if !matches!(st.view_mode, crate::ViewMode::Audio) {
                ctx.window.request_redraw();
            }
        }
        if let Ok(real_offsets) = comp.result_rx.try_recv() {
            st.chapter_offsets = real_offsets;
            st.offset_rx = None;
            ctx.reader.set_loading_visible(false);
            // Audio mode suppresses incidental presents while loading (so the
            // disk does not flash); force one now so the settled screen -- with
            // the final header/footer state -- is drawn.
            ctx.window.request_redraw();
            ctx.reader.set_page(
                (*st.chapter_offsets.get(st.current_chapter).unwrap_or(&0) + st.current_page)
                    as i32,
            );
            ctx.reader
                .set_page_count(*st.chapter_offsets.last().unwrap_or(&1) as i32);
            ctx.reader.set_saved_page(
                (*st.chapter_offsets.get(st.current_chapter).unwrap_or(&0) + st.current_page)
                    as i32,
            );
            st.text_dirty = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_save_keeps_marks_dirty() {
        let dir = std::env::temp_dir().join(format!(
            "kothok_test_status_marks_dirty_{}",
            std::process::id()
        ));
        let _ = std::fs::create_dir_all(&dir);
        let bad_path = dir.join("no_such_dir").join("marks");
        let mut dirty = true;
        let marks: Vec<crate::data::mark::Mark> = Vec::new();
        save_marks_if_dirty_at(&mut dirty, "/mnt/onboard/Test.epub", &marks, &bad_path);
        assert!(dirty, "dirty must remain true when save fails");
    }
}
