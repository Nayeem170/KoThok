// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::cell::Cell;

use log::debug;

use slint::SharedString;

use crate::data::config::{save_config, AppConfig};
use crate::Reader;

/// Reading-mode auto-sleep options: (seconds, label). 0 = never.
const SLEEP_OPTIONS: [(u32, &str); 3] = [(0, "Off"), (300, "5 min"), (900, "15 min")];

pub(super) fn handle_sleep_cycle(
    reader: &Reader,
    cfg: &mut AppConfig,
    cycle_cell: &Cell<i32>,
) {
    let dir = cycle_cell.replace(0);
    if dir == 0 {
        return;
    }
    let cur_idx = SLEEP_OPTIONS
        .iter()
        .position(|(secs, _)| *secs == cfg.reading_auto_sleep_secs)
        .unwrap_or(0);
    let n = SLEEP_OPTIONS.len();
    let next_idx = if dir == 2 {
        if cur_idx == 0 {
            n - 1
        } else {
            cur_idx - 1
        }
    } else {
        (cur_idx + 1) % n
    };
    let (secs, label) = SLEEP_OPTIONS[next_idx];
    cfg.reading_auto_sleep_secs = secs;
    reader.set_sleep_label(SharedString::from(label));
    save_config(cfg);
    debug!(
        "sleep-cycle: {} -> {} ({}s)",
        SLEEP_OPTIONS[cur_idx].1,
        label,
        secs
    );
}

/// Label for the current config value, used when the panel opens.
pub fn sleep_label(secs: u32) -> &'static str {
    SLEEP_OPTIONS
        .iter()
        .find(|(s, _)| *s == secs)
        .map(|(_, label)| *label)
        .unwrap_or("Off")
}
