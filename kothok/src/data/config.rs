// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::collections::HashMap;
use std::fs;

use log::warn;

pub use kobo_core::device::paths::{
    CONFIG_FILE, CRASH_LOG, POWER_DEV, PPM_DEBUG, PPM_DEPLOY, TOUCH_DEV,
};

pub const BOOK_DIR: &str = "/mnt/onboard";
pub const DEVICE_BOOK: &str = "/mnt/onboard/.adds/book.epub";
pub const BOOK_CACHE_DIR: &str = "/mnt/onboard/.adds/bookcache";
pub const POSITIONS_FILE: &str = "/mnt/onboard/.adds/positions";
pub const CACHE_DIR: &str = "/mnt/onboard/.adds/cache";
pub const VOICE_CACHE_FILE: &str = "/mnt/onboard/.adds/kothok/voices.json";
/// Screenshot output. On the onboard partition so captures come off over USB.
#[cfg(feature = "screenshot")]
pub const SHOTS_DIR: &str = "/mnt/onboard/.adds/shots";

const KEY_FONT_SIZE: &str = "font_size";
const KEY_TTS_LANG: &str = "tts_lang";
const KEY_TTS_VOICE: &str = "tts_voice";
const KEY_TTS_RATE: &str = "tts_rate";
const KEY_VOLUME: &str = "volume";
const KEY_BRIGHTNESS: &str = "brightness";
const KEY_NATURAL_SCROLL: &str = "natural_scroll";
const KEY_READING_AUTO_SLEEP: &str = "reading_auto_sleep";
const KEY_VOICE_PREFIX: &str = "voice.";

const TTS_RATE_DEFAULT: i32 = 60;
const VOLUME_DEFAULT: i32 = 100;
const BRIGHTNESS_DEFAULT: i32 = 50;

#[derive(Debug, PartialEq, Eq)]
pub struct AppConfig {
    pub font_size: i32,
    pub tts_lang: String,
    pub tts_voice: String,
    pub tts_rate: i32,
    pub volume: i32,
    pub brightness: i32,
    pub voices: HashMap<String, String>,
    /// Natural scroll (content follows finger). false = inverted/standard-list.
    pub natural_scroll: bool,
    /// Auto-sleep delay in reading mode, in seconds. 0 = never (screen stays on
    /// until the power button). Matches e-reader convention: e-ink draws nothing
    /// when static, so the only drain is the frontlight the reader needs anyway.
    pub reading_auto_sleep_secs: u32,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            font_size: 36,
            tts_lang: crate::meta::LANG_AUTO.into(),
            tts_voice: crate::panel::DEFAULT_VOICE_EN.into(),
            tts_rate: 50,
            volume: 100,
            brightness: 50,
            voices: HashMap::new(),
            natural_scroll: true,
            reading_auto_sleep_secs: 0,
        }
    }
}

pub use kothok_edge_tts::rate_string;

/// Load config, using `base_font` as the `font_size` default when the file is
/// absent or has no `font_size` line (first launch). Lets the caller pass a
/// screen-scaled default so a fresh install reads at an appropriate size on any
/// Kobo panel instead of one fixed pixel value. A saved `font_size` always wins.
pub fn load_config_from_base(path: &str, base_font: i32) -> AppConfig {
    let base_font = base_font.clamp(20, 60);
    let mut cfg = AppConfig {
        font_size: base_font,
        ..AppConfig::default()
    };
    let Ok(data) = fs::read_to_string(path) else {
        return cfg;
    };
    for line in data.lines() {
        if let Some((key, val)) = line.split_once('=') {
            let key = key.trim();
            let val = val.trim();
            match key {
                KEY_FONT_SIZE => {
                    cfg.font_size = val.parse::<i32>().unwrap_or(base_font).clamp(20, 60)
                }
                KEY_TTS_LANG => cfg.tts_lang = val.into(),
                KEY_TTS_VOICE => cfg.tts_voice = val.into(),
                KEY_TTS_RATE => {
                    cfg.tts_rate = val.parse::<i32>().unwrap_or(TTS_RATE_DEFAULT).clamp(0, 100)
                }
                KEY_VOLUME => {
                    cfg.volume = val.parse::<i32>().unwrap_or(VOLUME_DEFAULT).clamp(0, 100)
                }
                KEY_BRIGHTNESS => {
                    cfg.brightness = val
                        .parse::<i32>()
                        .unwrap_or(BRIGHTNESS_DEFAULT)
                        .clamp(0, 100)
                }
                KEY_NATURAL_SCROLL => cfg.natural_scroll = val == "1" || val == "true",
                KEY_READING_AUTO_SLEEP => {
                    cfg.reading_auto_sleep_secs =
                        val.parse::<u32>().unwrap_or(0).min(3600)
                }
                _ if key.starts_with(KEY_VOICE_PREFIX) => {
                    let lang = key.trim_start_matches(KEY_VOICE_PREFIX).to_string();
                    if !lang.is_empty() {
                        cfg.voices.insert(lang, val.to_string());
                    }
                }
                _ => {}
            }
        }
    }
    cfg
}

pub fn save_config_to(cfg: &AppConfig, path: &str) {
    let mut data = format!(
        "{KEY_FONT_SIZE}={}\n{KEY_TTS_LANG}={}\n{KEY_TTS_VOICE}={}\n{KEY_TTS_RATE}={}\n{KEY_VOLUME}={}\n{KEY_BRIGHTNESS}={}\n{KEY_NATURAL_SCROLL}={}\n{KEY_READING_AUTO_SLEEP}={}\n",
        cfg.font_size, cfg.tts_lang, cfg.tts_voice, cfg.tts_rate, cfg.volume, cfg.brightness,
        if cfg.natural_scroll { 1 } else { 0 },
        cfg.reading_auto_sleep_secs
    );
    for (lang, voice) in &cfg.voices {
        data.push_str(&format!("{KEY_VOICE_PREFIX}{lang}={voice}\n"));
    }
    if let Err(e) = fs::write(path, data) {
        warn!("config write failed ({path}): {e}");
    }
}

pub fn save_config(cfg: &AppConfig) {
    save_config_to(cfg, CONFIG_FILE);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn tmp_path(name: &str) -> String {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        format!(
            "{}/kothok_cfg_test_{}_{}_{n}",
            std::env::temp_dir().to_string_lossy(),
            name,
            std::process::id()
        )
    }

    #[test]
    fn config_roundtrip() {
        let p = tmp_path("roundtrip");
        let mut cfg = AppConfig::default();
        cfg.font_size = 44;
        cfg.tts_rate = 70;
        cfg.brightness = 30;
        save_config_to(&cfg, &p);
        let loaded = load_config_from_base(&p, 36);
        assert_eq!(loaded, cfg);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn config_load_missing_file_returns_defaults() {
        let cfg = load_config_from_base("/nonexistent/kobo-cfg-missing-12345", 36);
        assert_eq!(cfg, AppConfig::default());
    }

    #[test]
    fn base_font_used_when_no_font_size_line() {
        // First-launch on a device: no font_size in the file -> the screen-scaled
        // base applies.
        let p = tmp_path("basefont-missing");
        std::fs::write(&p, "tts_rate=70\n").unwrap();
        let cfg = load_config_from_base(&p, 42);
        assert_eq!(cfg.font_size, 42, "base font used when font_size absent");
        assert_eq!(cfg.tts_rate, 70, "other keys still parsed");
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn base_font_clamped_and_overridden_by_saved() {
        // Out-of-range base clamps; a saved font_size always wins over the base.
        let p = tmp_path("basefont-saved");
        std::fs::write(&p, "font_size=28\n").unwrap();
        assert_eq!(
            load_config_from_base(&p, 999).font_size,
            28,
            "saved font_size wins"
        );
        assert_eq!(
            load_config_from_base("/nonexistent/basefont-none", 999).font_size,
            60,
            "base clamps to max when no file"
        );
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn config_load_malformed_returns_defaults() {
        let p = tmp_path("malformed");
        std::fs::write(&p, "this is not config\ngarbage=line=extra\n===\n").unwrap();
        let cfg = load_config_from_base(&p, 36);
        assert_eq!(cfg, AppConfig::default());
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn config_voices_map_roundtrip() {
        let p = tmp_path("voices");
        let mut cfg = AppConfig::default();
        cfg.voices
            .insert("bn".into(), "bn-BD-NabanitaNeural".into());
        cfg.voices.insert("ar".into(), "ar-SA-ZariyahNeural".into());
        save_config_to(&cfg, &p);
        let loaded = load_config_from_base(&p, 36);
        assert_eq!(loaded.voices, cfg.voices);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn config_clamps_out_of_range_values() {
        let p = tmp_path("clamp");
        std::fs::write(
            &p,
            "font_size=999\ntts_rate=200\nvolume=-5\nbrightness=9000\n",
        )
        .unwrap();
        let cfg = load_config_from_base(&p, 36);
        assert_eq!(cfg.font_size, 60, "font_size clamped to max");
        assert_eq!(cfg.tts_rate, 100);
        assert_eq!(cfg.volume, 0, "volume clamped to min");
        assert_eq!(cfg.brightness, 100);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn config_reading_auto_sleep_roundtrip() {
        let p = tmp_path("sleep");
        let mut cfg = AppConfig::default();
        cfg.reading_auto_sleep_secs = 300;
        save_config_to(&cfg, &p);
        let loaded = load_config_from_base(&p, 36);
        assert_eq!(loaded.reading_auto_sleep_secs, 300);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn config_reading_auto_sleep_defaults_off() {
        assert_eq!(
            AppConfig::default().reading_auto_sleep_secs,
            0,
            "reading auto-sleep defaults to off"
        );
    }

    #[test]
    fn config_default_matches_save_load_of_default() {
        let p = tmp_path("default");
        let cfg = AppConfig::default();
        save_config_to(&cfg, &p);
        let loaded = load_config_from_base(&p, 36);
        assert_eq!(loaded, AppConfig::default());
        let _ = std::fs::remove_file(&p);
    }
}
