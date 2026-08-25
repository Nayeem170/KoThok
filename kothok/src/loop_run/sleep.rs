// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
//! Run-loop sleep entry triggered by the TTS sleep timer. The timer's contract
//! is identical in both view modes and every system state: put the device to
//! sleep, keep the view, wake where you slept. Awake uses `enter_sleep` (the
//! reading-button / auto-off transition); Locked uses `sleep_locked` (the
//! lock-timeout transition, which preserves the pre-lock brightness).
use super::*;

/// Drain a `sleep_requested` flag raised by the TTS sleep timer (timed
/// deadline). Auto-off's activity clock is refreshed every tick while audio
/// plays (`power.rs`), so it can never elapse during playback - the bedtime
/// sleep is the timer's job.
pub(super) fn sleep_from_timer(st: &mut LoopState, ctx: &mut LoopContext) {
    if matches!(st.system_state, SystemState::Asleep { .. }) {
        return;
    }
    let reader = ctx.reader;
    let cb = ctx.cb;
    // No surface may outlive the sleep it is drawn on.
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
    match st.system_state {
        SystemState::Locked => sleep_locked(st, ctx),
        SystemState::Awake => {
            st.saved_brightness = enter_sleep(st, ctx, st.picker_active);
            st.system_state = SystemState::Asleep {
                from_picker: st.picker_active,
            };
        }
        SystemState::Asleep { .. } => {}
    }
    info!("SLEEP (tts sleep timer; swipe-up to wake)");
    st.last_activity = std::time::Instant::now();
}

/// Transition Locked -> Asleep. `enter_sleep` reads the CURRENT frontlight,
/// which the lock already dimmed to zero, so its return (normally assigned to
/// `saved_brightness`) would wake dark; the lock's saved value is preserved
/// across the call instead. Lock bookkeeping is dropped here because
/// enter_sleep and wake own the radios from this point - a later unlock must
/// not double-reconnect. The view is kept: wake restores whichever mode the
/// user slept in.
pub(super) fn sleep_locked(st: &mut LoopState, ctx: &mut LoopContext) {
    let locked_brightness = st.saved_brightness;
    enter_sleep(st, ctx, st.picker_active);
    st.saved_brightness = locked_brightness;
    st.system_state = SystemState::Asleep {
        from_picker: st.picker_active,
    };
    st.lock_time = None;
    st.lock_radios_off = false;
    st.lock_wifi_off = false;
    st.lock_bt_off = false;
    ctx.reader.set_audio_locked(false);
}
