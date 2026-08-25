// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::fs;
use std::path::Path;

const KEY_FONT_SIZE: &str = "font_size";
const KEY_TTS_RATE: &str = "tts_rate";
const KEY_TTS_VOICE: &str = "tts_voice";
const KEY_LINE_SPACING_PCT: &str = "line_spacing_pct";
const KEY_TEXT_JUSTIFY: &str = "text_justify";
const KEY_MARGIN_PX: &str = "margin_px";

#[derive(Clone, Debug, Default)]
pub struct BookSettings {
    pub font_size: Option<i32>,
    pub tts_rate: Option<i32>,
    pub tts_voice: Option<String>,
    /// Line spacing and justification are edited from the *book* settings
    /// panel, so they belong to the book. They used to be written through
    /// `save_settings_for` into this file and then dropped on the floor here,
    /// which is why both silently reverted on the next open.
    pub line_spacing_pct: Option<i32>,
    pub text_justify: Option<bool>,
    pub margin_px: Option<i32>,
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
                KEY_LINE_SPACING_PCT => {
                    if let Ok(v) = val.parse::<i32>() {
                        settings.line_spacing_pct = Some(v.clamp(100, 200));
                    }
                }
                KEY_TEXT_JUSTIFY => {
                    settings.text_justify = Some(val != "0" && val != "false");
                }
                KEY_MARGIN_PX => {
                    if let Ok(v) = val.parse::<i32>() {
                        settings.margin_px = Some(v.clamp(
                            crate::rendering::layout::MARGIN_MIN_PX,
                            crate::rendering::layout::MARGIN_MAX_PX,
                        ));
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
        "{book_path}|{KEY_FONT_SIZE}={}|{KEY_TTS_RATE}={}|{KEY_TTS_VOICE}={}|{KEY_LINE_SPACING_PCT}={}|{KEY_TEXT_JUSTIFY}={}|{KEY_MARGIN_PX}={}",
        cfg.font_size,
        cfg.tts_rate,
        cfg.tts_voice,
        cfg.line_spacing_pct,
        if cfg.text_justify { 1 } else { 0 },
        cfg.margin_px,
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
    if let Some(v) = settings.line_spacing_pct {
        cfg.line_spacing_pct = v;
    }
    if let Some(v) = settings.text_justify {
        cfg.text_justify = v;
    }
    if let Some(v) = settings.margin_px {
        cfg.margin_px = v;
    }
    // Deliberately does NOT push the margin into `layout`'s global: this
    // function is pure config-merging and is exercised by tests that run in
    // parallel with the layout tests, which measure `text_w()`. Callers apply
    // it (see `book_init` and `open_book`) as part of the same step that
    // repaginates.
}

/// Drop a book's saved overrides so it reads with the global defaults again.
///
/// Removes the book's line outright rather than writing default values into
/// it: a line of defaults is indistinguishable from a deliberate choice, and
/// would pin the book to today's defaults even after the globals moved on.
pub fn clear_book_settings(file: &Path, book_path: &str) {
    let Ok(existing) = fs::read_to_string(file) else {
        return;
    };
    let kept: Vec<&str> = existing
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with(book_path))
        .collect();
    let _ = fs::write(file, kept.join("\n"));
}

pub fn push_book_settings_to_ui(reader: &crate::Reader, cfg: &crate::data::config::AppConfig) {
    reader.set_font_size_val(cfg.font_size);
    reader.set_line_spacing_val(cfg.line_spacing_pct);
    reader.set_text_justify(cfg.text_justify);
    reader.set_margin_val(cfg.margin_px);
    reader.set_auto_hide_header(cfg.auto_hide_header);
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

    /// Both controls live in the book settings panel and were being written
    /// into this file by `save_settings_for` and then dropped, so every change
    /// reverted on the next open.
    #[test]
    fn line_spacing_and_justification_survive_a_roundtrip() {
        let p = tmp_path("spacing");
        let cfg = crate::data::config::AppConfig {
            line_spacing_pct: 180,
            text_justify: false,
            ..test_cfg()
        };
        save_book_settings(Path::new(&p), "/mnt/onboard/A.epub", &cfg);
        let loaded = load_book_settings(Path::new(&p), "/mnt/onboard/A.epub");
        assert_eq!(loaded.line_spacing_pct, Some(180));
        assert_eq!(loaded.text_justify, Some(false));

        let mut applied = crate::data::config::AppConfig::default();
        apply_book_settings(&mut applied, &loaded);
        assert_eq!(applied.line_spacing_pct, 180);
        assert!(!applied.text_justify);
        let _ = std::fs::remove_file(&p);
    }

    /// A book saved before these keys existed must keep the current values
    /// rather than snapping to a parsed-from-nothing default.
    #[test]
    fn book_settings_without_spacing_keys_leave_config_alone() {
        let p = tmp_path("legacy");
        std::fs::write(&p, "/mnt/onboard/A.epub|font_size=30\n").unwrap();
        let loaded = load_book_settings(Path::new(&p), "/mnt/onboard/A.epub");
        assert_eq!(loaded.line_spacing_pct, None);
        assert_eq!(loaded.text_justify, None);

        let mut applied = crate::data::config::AppConfig {
            line_spacing_pct: 160,
            text_justify: false,
            ..Default::default()
        };
        apply_book_settings(&mut applied, &loaded);
        assert_eq!(applied.line_spacing_pct, 160);
        assert!(!applied.text_justify);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn margin_survives_a_roundtrip_and_clamps() {
        let p = tmp_path("margin");
        let cfg = crate::data::config::AppConfig {
            margin_px: 64,
            ..test_cfg()
        };
        save_book_settings(Path::new(&p), "/mnt/onboard/A.epub", &cfg);
        assert_eq!(
            load_book_settings(Path::new(&p), "/mnt/onboard/A.epub").margin_px,
            Some(64)
        );

        std::fs::write(&p, "/mnt/onboard/B.epub|margin_px=9999\n").unwrap();
        assert_eq!(
            load_book_settings(Path::new(&p), "/mnt/onboard/B.epub").margin_px,
            Some(crate::rendering::layout::MARGIN_MAX_PX),
            "an out-of-range margin must clamp, not blank the page"
        );
        let _ = std::fs::remove_file(&p);
    }

    /// Reset drops the book's line entirely. Writing defaults into it instead
    /// would pin the book to today's defaults even after the globals moved on,
    /// and is indistinguishable from a deliberate choice.
    #[test]
    fn clearing_one_book_leaves_the_others_untouched() {
        let p = tmp_path("clear");
        let mut a = test_cfg();
        a.font_size = 30;
        let mut b = test_cfg();
        b.font_size = 48;
        save_book_settings(Path::new(&p), "/mnt/onboard/A.epub", &a);
        save_book_settings(Path::new(&p), "/mnt/onboard/B.epub", &b);

        clear_book_settings(Path::new(&p), "/mnt/onboard/A.epub");

        assert_eq!(
            load_book_settings(Path::new(&p), "/mnt/onboard/A.epub").font_size,
            None,
            "the reset book must fall back to globals"
        );
        assert_eq!(
            load_book_settings(Path::new(&p), "/mnt/onboard/B.epub").font_size,
            Some(48),
            "resetting one book must not touch another"
        );
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn clearing_a_book_with_no_saved_settings_is_a_noop() {
        let p = tmp_path("clear-missing");
        let cfg = test_cfg();
        save_book_settings(Path::new(&p), "/mnt/onboard/B.epub", &cfg);
        clear_book_settings(Path::new(&p), "/mnt/onboard/A.epub");
        assert_eq!(
            load_book_settings(Path::new(&p), "/mnt/onboard/B.epub").font_size,
            Some(42)
        );
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
