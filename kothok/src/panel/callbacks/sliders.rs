// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::sync::mpsc::Sender;

use crate::audio::Cmd;
use crate::data::config::{save_config, AppConfig};
use crate::device::power::frontlight_set;
use crate::Reader;

pub(super) fn handle_brightness(
    reader: &Reader,
    cfg: &mut AppConfig,
    fl_path: &Option<std::path::PathBuf>,
    frac_opt: &Option<(i32, f32)>,
) {
    if let Some((0, frac)) = frac_opt {
        let new_val = (frac * 100.0).round() as i32;
        reader.set_brightness_val(new_val);
        cfg.brightness = new_val;
        if let Some(ref path) = fl_path {
            frontlight_set(path, new_val as u32);
        }
        save_config(cfg);
    }
}

pub(super) fn handle_volume(
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cfg: &mut AppConfig,
    frac_opt: &Option<(i32, f32)>,
) {
    if let Some((3, frac)) = frac_opt {
        let new_val = (frac * 100.0).round() as i32;
        cfg.volume = new_val;
        reader.set_volume_val(new_val);
        // best-effort: channel may be full
        let _ = cmd_tx.send(Cmd::Volume(new_val as u32));
        save_config(cfg);
    }
}

pub(super) fn handle_tts_rate(
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cfg: &mut AppConfig,
    frac_opt: &Option<(i32, f32)>,
) {
    if let Some((1, frac)) = frac_opt {
        let new_val = (frac * 100.0).round() as i32;
        cfg.tts_rate = new_val;
        reader.set_tts_speed(new_val);
        // best-effort: channel may be full
        let _ = cmd_tx.send(Cmd::Rate(crate::data::config::rate_string(new_val)));
        save_config(cfg);
    }
}
