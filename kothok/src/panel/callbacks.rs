// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
mod chapters;
mod connectivity;
mod font;
mod sliders;
mod voice;

use std::sync::mpsc::Sender;

use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::data::config::AppConfig;
use crate::loop_state::LoopState;
use crate::Reader;

pub fn process_panel_callbacks(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cfg: &mut AppConfig,
    fl_path: &Option<std::path::PathBuf>,
    cb: &Callbacks,
) -> bool {
    let frac_opt = cb.panel_frac.take();

    sliders::handle_brightness(reader, cfg, fl_path, &frac_opt);
    sliders::handle_volume(reader, cmd_tx, cfg, &frac_opt);
    sliders::handle_tts_rate(reader, cmd_tx, cfg, &frac_opt);
    let mut text_dirty = font::handle_font_slider(st, reader, cmd_tx, cfg, cb);
    voice::handle_voice_cycle(reader, cmd_tx, cfg, &cb.panel_voice_cell);
    connectivity::ensure_wifi_bt_lists(st, reader);
    connectivity::handle_wifi(reader, st, &cb.wifi_toggle_cell, &cb.wifi_cycle_cell);
    connectivity::handle_bt(reader, st, &cb.bt_toggle_cell, &cb.bt_cycle_cell);
    text_dirty |= chapters::handle_chapter_overlay(st, reader, cmd_tx, cb);

    text_dirty
}
