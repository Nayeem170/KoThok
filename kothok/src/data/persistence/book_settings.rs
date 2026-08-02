// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::fs;
use std::path::Path;

const KEY_FONT_SIZE: &str = "font_size";
const KEY_TTS_RATE: &str = "tts_rate";
const KEY_TTS_VOICE: &str = "tts_voice";

#[derive(Clone, Debug, Default)]
pub struct BookSettings {
    pub font_size: Option<i32>,
    pub tts_rate: Option<i32>,
    pub tts_voice: Option<String>,
}

pub fn load_book_settings(file: &Path, book_path: &str) -> BookSettings {
    let data = match fs::read_to_string(file) {
        Ok(d) => d,
        Err(_) => return BookSettings::default(),
    };
    for line in data.lines() {
        if line.is_empty() {
            continue;
        }
        let Some((path, rest)) = line.split_once('|') else {
            continue;
        };
        if path != book_path {
            continue;
        }
        let mut settings = BookSettings::default();
        for pair in rest.split('|') {
            let Some((key, val)) = pair.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let val = val.trim();
            match key {
                KEY_FONT_SIZE => {
                    if let Ok(v) = val.parse::<i32>() {
                        settings.font_size = Some(v.clamp(20, 60));
                    }
                }
                KEY_TTS_RATE => {
                    if let Ok(v) = val.parse::<i32>() {
                        settings.tts_rate = Some(v.clamp(0, 100));
                    }
                }
                KEY_TTS_VOICE => {
                    if !val.is_empty() {
                        settings.tts_voice = Some(val.to_string());
                    }
                }
                _ => {}
            }
        }
        return settings;
    }
    BookSettings::default()
}

pub fn save_book_settings(file: &Path, book_path: &str, cfg: &crate::data::config::AppConfig) {
    let mut lines: Vec<String> = fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with(book_path))
        .map(String::from)
        .collect();
    let entry = format!(
        "{book_path}|{KEY_FONT_SIZE}={}|{KEY_TTS_RATE}={}|{KEY_TTS_VOICE}={}",
        cfg.font_size, cfg.tts_rate, cfg.tts_voice
    );
    lines.push(entry);
    let _ = fs::write(file, lines.join("\n"));
}

pub fn apply_book_settings(cfg: &mut crate::data::config::AppConfig, settings: &BookSettings) {
    if let Some(v) = settings.font_size {
        cfg.font_size = v;
    }
    if let Some(v) = settings.tts_rate {
        cfg.tts_rate = v;
    }
    if let Some(ref v) = settings.tts_voice {
        cfg.tts_voice = v.clone();
        cfg.voices.insert(cfg.tts_lang.clone(), v.clone());
    }
}

pub fn push_book_settings_to_ui(reader: &crate::Reader, cfg: &crate::data::config::AppConfig) {
    reader.set_font_size_val(cfg.font_size);
    reader.set_tts_speed(cfg.tts_rate);
    reader.set_tts_voice(slint::SharedString::from(&cfg.tts_voice));
    reader.set_tts_voice_label(slint::SharedString::from(crate::panel::voice_label(
        &cfg.tts_voice,
    )));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static SEQ: AtomicU64 = AtomicU64::new(0);

    fn tmp_path(name: &str) -> String {
        let n = SEQ.fetch_add(1, Ordering::SeqCst);
        format!(
            "{}/kothok_bs_test_{}_{}_{n}",
            std::env::temp_dir().to_string_lossy(),
            name,
            std::process::id()
        )
    }

    fn test_cfg() -> crate::data::config::AppConfig {
        crate::data::config::AppConfig {
            font_size: 42,
            brightness: 60,
            volume: 80,
            tts_rate: 55,
            tts_lang: "en-US".into(),
            tts_voice: "Guy (English)".into(),
            ..Default::default()
        }
    }

    #[test]
    fn book_settings_roundtrip() {
        let p = tmp_path("roundtrip");
        let cfg = test_cfg();
        save_book_settings(Path::new(&p), "/mnt/onboard/A.epub", &cfg);
        let loaded = load_book_settings(Path::new(&p), "/mnt/onboard/A.epub");
        assert_eq!(loaded.font_size, Some(42));
        assert_eq!(loaded.tts_rate, Some(55));
        assert_eq!(loaded.tts_voice, Some("Guy (English)".into()));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn book_settings_missing_file_returns_all_none() {
        let loaded = load_book_settings(
            Path::new("/nonexistent/kothok_bs_missing_12345"),
            "/mnt/onboard/A.epub",
        );
        assert_eq!(loaded.font_size, None);
        assert_eq!(loaded.tts_rate, None);
        assert_eq!(loaded.tts_voice, None);
    }

    #[test]
    fn book_settings_multiple_books() {
        let p = tmp_path("multi");
        let mut cfg_a = test_cfg();
        cfg_a.font_size = 30;
        let mut cfg_b = test_cfg();
        cfg_b.font_size = 48;
        save_book_settings(Path::new(&p), "/mnt/onboard/A.epub", &cfg_a);
        save_book_settings(Path::new(&p), "/mnt/onboard/B.epub", &cfg_b);
        let a = load_book_settings(Path::new(&p), "/mnt/onboard/A.epub");
        assert_eq!(a.font_size, Some(30));
        let b = load_book_settings(Path::new(&p), "/mnt/onboard/B.epub");
        assert_eq!(b.font_size, Some(48));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn book_settings_save_overwrites_previous() {
        let p = tmp_path("overwrite");
        let mut cfg1 = test_cfg();
        cfg1.font_size = 30;
        save_book_settings(Path::new(&p), "/mnt/onboard/A.epub", &cfg1);
        let mut cfg2 = test_cfg();
        cfg2.font_size = 48;
        save_book_settings(Path::new(&p), "/mnt/onboard/A.epub", &cfg2);
        let loaded = load_book_settings(Path::new(&p), "/mnt/onboard/A.epub");
        assert_eq!(loaded.font_size, Some(48));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn book_settings_partial_fields() {
        let p = tmp_path("partial");
        std::fs::write(&p, "/mnt/onboard/A.epub|font_size=48\n").unwrap();
        let loaded = load_book_settings(Path::new(&p), "/mnt/onboard/A.epub");
        assert_eq!(loaded.font_size, Some(48));
        assert_eq!(loaded.tts_rate, None);
        assert_eq!(loaded.tts_voice, None);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn book_settings_unknown_keys_ignored() {
        let p = tmp_path("unknown");
        std::fs::write(
            &p,
            "/mnt/onboard/A.epub|font_size=42|future_setting=foo|another_bar\n",
        )
        .unwrap();
        let loaded = load_book_settings(Path::new(&p), "/mnt/onboard/A.epub");
        assert_eq!(loaded.font_size, Some(42));
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn book_settings_clamps_out_of_range() {
        let p = tmp_path("clamp");
        std::fs::write(&p, "/mnt/onboard/A.epub|font_size=999|tts_rate=abc\n").unwrap();
        let loaded = load_book_settings(Path::new(&p), "/mnt/onboard/A.epub");
        assert_eq!(loaded.font_size, Some(60));
        assert_eq!(loaded.tts_rate, None);
        let _ = std::fs::remove_file(&p);
    }
}
