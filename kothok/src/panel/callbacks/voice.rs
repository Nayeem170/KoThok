// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::sync::mpsc::Sender;

use log::debug;

use slint::SharedString;

use crate::audio::glue::best_effort_send;
use crate::audio::Cmd;
use crate::data::config::{save_config, AppConfig};
use crate::Reader;

use super::super::{voice_label, voices_for_lang};

pub(super) fn handle_voice_cycle(
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cfg: &mut AppConfig,
    panel_voice_cell: &std::cell::Cell<i32>,
) {
    let dir = panel_voice_cell.replace(0);
    if dir == 0 {
        return;
    }
    let voices = voices_for_lang(&cfg.tts_lang);
    let current = cfg.tts_voice.as_str();
    let idx = voices.iter().position(|v| v.id() == current).unwrap_or(0);
    let new_idx = if dir == 2 {
        if idx == 0 {
            voices.len() - 1
        } else {
            idx - 1
        }
    } else {
        (idx + 1) % voices.len()
    };
    let new_voice = voices[new_idx].id();
    cfg.tts_voice = new_voice.to_string();
    cfg.voices
        .insert(cfg.tts_lang.clone(), new_voice.to_string());
    debug!(
        "voice-cycle: lang={} dir={} new={} saved_map_size={}",
        cfg.tts_lang,
        if dir == 2 { "prev" } else { "next" },
        new_voice,
        cfg.voices.len()
    );
    reader.set_tts_voice(SharedString::from(new_voice));
    reader.set_tts_voice_label(SharedString::from(voice_label(new_voice)));
    let cmd = if cfg.tts_lang == crate::meta::LANG_BN_BD {
        Cmd::BnVoice(new_voice.to_string())
    } else {
        Cmd::Voice(new_voice.to_string())
    };
    best_effort_send(cmd_tx, cmd);
    save_config(cfg);
}
