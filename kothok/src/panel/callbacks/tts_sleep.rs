// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::cell::Cell;

use slint::SharedString;

use crate::data::config::{AppConfig, TtsSleepMode};
use crate::loop_state::LoopState;
use crate::Reader;

/// The five sleep-timer modes in cycle order. `Off` is first so a fresh reader
/// starts there, and forward/back cycle wraps around the ends. Timed-only by
/// design: one countdown, identical behaviour in reading and audio mode.
const MODES: [TtsSleepMode; 5] = [
    TtsSleepMode::Off,
    TtsSleepMode::Mins15,
    TtsSleepMode::Mins30,
    TtsSleepMode::Mins45,
    TtsSleepMode::Mins60,
];

/// Cycle the TTS sleep-timer mode. Global setting (not per-book): the timer is
/// a playback preference, so it persists in the main config. The mode is
/// written to both `cfg` (persisted) and `st` (the runtime mirror the
/// audio-event handlers read, since they take `st` not `cfg`). If a timer is
/// already armed, the new mode takes effect immediately via `arm`.
pub(super) fn handle_tts_sleep_cycle(
    st: &mut LoopState,
    reader: &Reader,
    cfg: &mut AppConfig,
    cycle_cell: &Cell<i32>,
) -> bool {
    let dir = cycle_cell.replace(0);
    if dir == 0 {
        return false;
    }
    let cur_idx = MODES
        .iter()
        .position(|m| *m == cfg.tts_sleep_mode)
        .unwrap_or(0);
    let n = MODES.len();
    let next_idx = if dir == 2 {
        if cur_idx == 0 {
            n - 1
        } else {
            cur_idx - 1
        }
    } else {
        (cur_idx + 1) % n
    };
    let mode = MODES[next_idx];
    cfg.tts_sleep_mode = mode;
    st.tts_sleep_mode = mode;
    // A mode change starts a fresh countdown; any frozen remaining from the old
    // mode is meaningless for the new one.
    st.tts_sleep_paused_remaining = None;
    reader.set_tts_sleep_label(SharedString::from(mode.label()));
    // Arm immediately when audio is playing: a mode change off->timed mid-playback
    // would otherwise wait for the next Event::Playing (pause/resume), so the
    // caption would never appear. When not playing, arming is deferred to the
    // next Event::Playing, which reads the mode just set.
    if reader.get_playing() {
        crate::loop_run::tts_sleep::arm(st, reader);
    }
    log::info!("panel: tts-sleep cycle to {}", mode.label());
    true
}
