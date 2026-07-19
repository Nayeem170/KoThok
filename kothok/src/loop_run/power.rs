// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;
use crate::rendering::common::rgb565_as_bytes_ref;

pub(super) fn check_font_repaginate(st: &mut LoopState, ctx: &mut LoopContext) {
    let reader = ctx.reader;
    let cmd_tx = ctx.cmd_tx;
    let window = ctx.window;
    let fb = ctx.fb;
    let w = ctx.w;
    let h = ctx.h;
    if text_render::font_install_count() != st.last_font_count {
        st.last_font_count = text_render::font_install_count();
        if !st.picker_active && st.chapters.len() > st.current_chapter {
            log::debug!("font: newly installed - rebuilding st.state");
            st.state = build_state(
                &mut st.chapters[st.current_chapter],
                st.body_px,
                st.head_px,
                st.line_h,
            );
            let total = st.state.all_rows.len();
            st.current_page = st.current_page.min(total.saturating_sub(1));
            crate::reader::apply_page(
                reader,
                &st.state,
                st.current_page,
                &st.chapter_offsets,
                st.current_chapter,
            );
            let utts = crate::audio::glue::page_utterances(st.current_page, &st.state);
            crate::audio::glue::best_effort_send(cmd_tx, Cmd::Reload(utts));
            crate::audio::glue::best_effort_send(cmd_tx, Cmd::Seek(0));
            window.request_redraw();
            window.draw_if_needed(|r| {
                r.render(&mut st.buffer, w);
            });
            fb.present(
                rgb565_as_bytes_ref(&st.buffer),
                w,
                h,
                false,
                0,
                h,
                WAVE_GC16,
            );
            st.prev_buffer.copy_from_slice(&st.buffer);
        }
    }
}

/// Dim the frontlight for the locked state, remembering the level to restore.
///
/// Lock keeps the display on and audio running, so the frontlight going dark is
/// the only signal that touch is now rejected -- without it there is nothing on
/// screen to distinguish locked from awake.
pub(super) fn lock_frontlight_off(st: &mut LoopState, ctx: &LoopContext) {
    let Some(path) = ctx.fl_path.as_deref() else {
        return;
    };
    if let Some(level) = frontlight_get(path) {
        st.saved_brightness = level;
    }
    crate::device::power::frontlight_set(path, 0);
    info!("lock: frontlight off (saved {})", st.saved_brightness);
}

/// Restore the frontlight level saved when the lock was entered.
fn unlock_frontlight_restore(st: &LoopState, ctx: &LoopContext) {
    if let Some(path) = ctx.fl_path.as_deref() {
        crate::device::power::restore_frontlight(path, st.saved_brightness);
        info!("unlock: frontlight restored to {}", st.saved_brightness);
    }
}

/// On entering lock: keep BT+WiFi on if audio is playing (so playback continues
/// under lock), but drop them the way sleep does if playback is paused/stopped.
/// Records whether it disconnected so `unlock_radios` can bring them back.
pub(super) fn lock_radios(st: &mut LoopState, ctx: &LoopContext) {
    let reader = ctx.reader;
    if reader.get_playing() {
        st.lock_radios_off = false;
        st.lock_wifi_off = false;
        st.lock_bt_off = false;
        return;
    }
    // Record which radios were on *now*, before we drop them. Unlock restores
    // exactly these, independent of the live state (which goes false as soon as
    // the radio drops).
    st.lock_wifi_off = reader.get_wifi_on();
    st.lock_bt_off = reader.get_bt_on();
    if st.lock_wifi_off {
        crate::device::wifi_toggle(false);
    }
    if st.lock_bt_off {
        crate::device::bt_toggle(false);
    }
    st.lock_radios_off = st.lock_wifi_off || st.lock_bt_off;
    if st.lock_radios_off {
        info!(
            "lock: paused -> disconnected (wifi={} bt={})",
            st.lock_wifi_off, st.lock_bt_off
        );
    }
}

/// On unlock: reconnect exactly the radios that lock disconnected, then clear the
/// markers. No-op if lock left them on. Reconnect is unconditional per-radio --
/// `reader.get_*_on()` is already false by now (the radio dropped), so gating on
/// it would skip the reconnect entirely.
pub(super) fn unlock_radios(st: &mut LoopState, ctx: &LoopContext) {
    if !st.lock_radios_off {
        return;
    }
    let reader = ctx.reader;
    if st.lock_wifi_off {
        crate::device::wifi_toggle(true);
        reader.set_wifi_on(true);
    }
    if st.lock_bt_off {
        crate::device::bt_toggle(true);
        reader.set_bt_on(true);
    }
    info!(
        "unlock: reconnect (wifi={} bt={})",
        st.lock_wifi_off, st.lock_bt_off
    );
    st.lock_radios_off = false;
    st.lock_wifi_off = false;
    st.lock_bt_off = false;
}

pub(super) fn handle_power_button(st: &mut LoopState, ctx: &mut LoopContext) -> LoopFlow {
    let reader = ctx.reader;
    let window = ctx.window;
    let fb = ctx.fb;
    let cb = ctx.cb;
    let all_books = &mut *ctx.all_books;
    let caps = ctx.caps;
    let power_pressed = ctx.power_pressed;
    let fl_path = ctx.fl_path;
    if power_pressed.swap(false, std::sync::atomic::Ordering::SeqCst) {
        match st.system_state {
            SystemState::Awake => {
                if st.view_mode == crate::ViewMode::Audio {
                    st.system_state = SystemState::Locked;
                    st.lock_time = Some(std::time::Instant::now());
                    if st.panel_open {
                        st.panel_open = false;
                        cb.panel_open_cell.set(false);
                        reader.set_panel_open(false);
                    }
                    if reader.get_chapter_overlay_open() {
                        reader.set_chapter_overlay_open(false);
                        reader.set_chapter_preview_idx(-1);
                        reader.set_chapter_pending(-1);
                    }
                    reader.set_audio_locked(true);
                    lock_frontlight_off(st, ctx);
                    lock_radios(st, ctx);
                    info!("LOCK (power button, audio mode)");
                } else {
                    if st.panel_open {
                        st.panel_open = false;
                        cb.panel_open_cell.set(false);
                        reader.set_panel_open(false);
                    }
                    if reader.get_chapter_overlay_open() {
                        reader.set_chapter_overlay_open(false);
                        reader.set_chapter_preview_idx(-1);
                        reader.set_chapter_pending(-1);
                    }
                    st.saved_brightness =
                        enter_sleep(st, ctx, st.picker_active, reader.get_bt_on());
                    st.system_state = SystemState::Asleep {
                        from_picker: st.picker_active,
                    };
                    info!("SLEEP (swipe-up to wake)");
                }
                st.last_activity = std::time::Instant::now();
                return LoopFlow::Continue;
            }
            SystemState::Asleep { from_picker } => {
                if st.panel_open {
                    st.panel_open = false;
                    cb.panel_open_cell.set(false);
                }
                reader.set_panel_open(false);
                reader.set_chapter_overlay_open(false);
                reader.set_chapter_preview_idx(-1);
                reader.set_chapter_pending(-1);
                reader.set_loading_visible(false);
                st.cover_page_visible = false;
                window.request_redraw();
                if from_picker {
                    show_book_picker(
                        reader,
                        fb,
                        window,
                        &mut st.buffer,
                        &mut st.text_cache,
                        &mut st.picker_cover_cache,
                        all_books,
                        st.picker_scroll,
                        st.library_filter,
                        &caps.current_clock(),
                        caps.battery_pct(),
                        "",
                        // Waking from sleep: the panel is showing the splash, so
                        // this one needs the clearing pass. `show_book_picker`
                        // presents it -- a second full GC16 here only added a
                        // second flash.
                        PickerRefresh::Full,
                    );
                    std::thread::sleep(std::time::Duration::from_millis(SLEEP_PANEL_SETTLE_MS));
                    st.prev_buffer.copy_from_slice(&st.buffer);
                    st.picker_active = true;
                    reader.set_picker_active(true);
                    if let Some(ref path) = fl_path {
                        crate::device::power::restore_frontlight(path, st.saved_brightness);
                    }
                    if reader.get_wifi_on() {
                        crate::device::wifi_toggle(true);
                    }
                } else {
                    wake_from_sleep(st, ctx);
                }
                st.system_state = SystemState::Awake;
                st.last_activity = std::time::Instant::now();
                st.prev_down = false;
                if reader.get_bt_on() {
                    crate::device::bt_toggle(true);
                }
                return LoopFlow::Continue;
            }
            SystemState::Locked => {
                st.system_state = SystemState::Awake;
                st.lock_time = None;
                reader.set_audio_locked(false);
                unlock_frontlight_restore(st, ctx);
                unlock_radios(st, ctx);
                st.last_activity = std::time::Instant::now();
                info!("UNLOCK (power button)");
                return LoopFlow::Continue;
            }
        }
    }
    LoopFlow::Normal
}

pub(super) fn poll_asleep_wake(st: &mut LoopState, ctx: &mut LoopContext) -> LoopFlow {
    let touch_dev = &mut *ctx.touch_dev;
    let touch_fd = ctx.touch_fd;
    let touch_cfg = ctx.touch_cfg;
    let power_pressed = ctx.power_pressed;
    if matches!(st.system_state, SystemState::Asleep { .. }) {
        poll_touch_for_wake(touch_dev, touch_fd, power_pressed.clone(), touch_cfg);
        if power_pressed.load(std::sync::atomic::Ordering::SeqCst) {
            st.last_activity = std::time::Instant::now();
            return LoopFlow::Continue;
        }
    }
    LoopFlow::Normal
}

pub(super) fn auto_sleep(st: &mut LoopState, ctx: &mut LoopContext) -> LoopFlow {
    let reader = ctx.reader;
    let cb = ctx.cb;

    // While TTS is playing, keep the inactivity clock fresh so neither the
    // reading-mode auto-sleep nor the audio-mode auto-lock fires mid-listen.
    // Without this, `last_activity` goes stale during a long hands-off listen
    // and the device sleeps/locks the instant playback stops -- which reads as
    // "it slept while I was still hearing it". The power button still works.
    if reader.get_playing() {
        st.last_activity = std::time::Instant::now();
    }

    if st.system_state == SystemState::Locked {
        if let Some(lock_time) = st.lock_time {
            if lock_time.elapsed().as_secs() > LOCK_SLEEP_SECS {
                // The lock already dimmed the frontlight and stashed the real
                // level in `saved_brightness`; `enter_sleep` would read the
                // current (zero) level back and lose it, so keep ours.
                let locked_brightness = st.saved_brightness;
                enter_sleep(st, ctx, st.picker_active, reader.get_bt_on());
                st.saved_brightness = locked_brightness;
                st.system_state = SystemState::Asleep {
                    from_picker: st.picker_active,
                };
                st.lock_time = None;
                // enter_sleep + wake now own the radios; drop the lock markers so
                // a later unlock does not double-reconnect.
                st.lock_radios_off = false;
                st.lock_wifi_off = false;
                st.lock_bt_off = false;
                reader.set_audio_locked(false);
                st.view_mode = crate::ViewMode::Reading;
                reader.set_audio_mode(false);
                info!("LOCK-SLEEP after {}s locked", LOCK_SLEEP_SECS);
                st.last_activity = std::time::Instant::now();
                return LoopFlow::Continue;
            }
        }
        return LoopFlow::Normal;
    }

    if !reader.get_playing() && st.last_activity.elapsed().as_secs() > AUTO_SLEEP_SECS {
        if st.view_mode == crate::ViewMode::Audio {
            st.system_state = SystemState::Locked;
            st.lock_time = Some(std::time::Instant::now());
            reader.set_audio_locked(true);
            if st.panel_open {
                st.panel_open = false;
                cb.panel_open_cell.set(false);
                reader.set_panel_open(false);
            }
            if reader.get_chapter_overlay_open() {
                reader.set_chapter_overlay_open(false);
                reader.set_chapter_preview_idx(-1);
                reader.set_chapter_pending(-1);
            }
            lock_frontlight_off(st, ctx);
            lock_radios(st, ctx);
            info!(
                "AUTO-LOCK after {}s inactivity (audio mode)",
                AUTO_SLEEP_SECS
            );
            st.last_activity = std::time::Instant::now();
            return LoopFlow::Continue;
        }
        info!("AUTO-SLEEP after {}s inactivity", AUTO_SLEEP_SECS);
        if st.panel_open {
            st.panel_open = false;
            cb.panel_open_cell.set(false);
            reader.set_panel_open(false);
        }
        if reader.get_chapter_overlay_open() {
            reader.set_chapter_overlay_open(false);
            reader.set_chapter_preview_idx(-1);
            reader.set_chapter_pending(-1);
        }
        st.saved_brightness = enter_sleep(st, ctx, st.picker_active, reader.get_bt_on());
        st.system_state = SystemState::Asleep {
            from_picker: st.picker_active,
        };
        info!("AUTO-SLEEP (swipe-up to wake)");
        st.last_activity = std::time::Instant::now();
        return LoopFlow::Continue;
    }
    LoopFlow::Normal
}
