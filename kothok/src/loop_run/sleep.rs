// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
//! Run-loop sleep entry triggered by the TTS sleep timer. Reuses `enter_sleep`
//! (the reading-button / auto-off transition) when Awake; when the device is
//! Locked it stops only the audio, because `enter_sleep` during a lock captures
//! the lock's zero frontlight and would wake dark.
use super::*;

/// Drain a `sleep_requested` flag raised by the TTS sleep timer (timed
/// deadline). Auto-off's activity clock is refreshed every tick while
/// audio plays (`power.rs:291-293`), so it can never elapse during playback -
/// the bedtime sleep is the timer's job.
///
/// State-aware:
/// - Awake: full device sleep via `enter_sleep` (same transition as the
///   reading-mode sleep button: cover + frontlight off + Stop + radios off).
///   Audio mode switches to reading first so the tested reading sleep/wake path
///   restores the view on wake.
/// - Locked: the screen is already dark and the user is listening (the lock
///   keeps audio playing by design). Stopping the audio on schedule honours the
///   timer's contract; a full `enter_sleep` here would capture the lock's zero
///   frontlight and wake dark, so the device is left locked and `auto_sleep`'s
///   lock-timeout (`LOCK_SLEEP_SECS`) sleeps it. The timer's fire site has
///   already disarmed and cleared the label before this runs.
/// - Asleep: a stale re-fire; no-op (no double `enter_sleep`).
pub(super) fn sleep_from_timer(st: &mut LoopState, ctx: &mut LoopContext) {
    match st.system_state {
        SystemState::Asleep { .. } => return,
        SystemState::Locked => {
            best_effort_send(ctx.cmd_tx, Cmd::Pause);
            info!("tts-sleep: fired while Locked, pausing audio (device stays locked)");
            return;
        }
        SystemState::Awake => {}
    }
    let reader = ctx.reader;
    let cb = ctx.cb;
    if st.view_mode == ViewMode::Audio {
        st.view_mode = ViewMode::Reading;
        reader.set_audio_mode(false);
    }
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
    st.saved_brightness = enter_sleep(st, ctx, st.picker_active);
    st.system_state = SystemState::Asleep {
        from_picker: st.picker_active,
    };
    info!("SLEEP (tts sleep timer; swipe-up to wake)");
    st.last_activity = std::time::Instant::now();
}
