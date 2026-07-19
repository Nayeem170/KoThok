// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;

fn tmp(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::create_dir_all(&dir);
    dir.join("positions")
}

#[test]
fn progress_survives_a_roundtrip() {
    let file = tmp("kothok_test_progress_roundtrip");
    save_position(
        &file,
        "/mnt/onboard/Book.epub",
        &ReadingPosition {
            chapter: 4,
            page: 11,
            cur_start: 10,
            cur_end: 20,
            view_mode: ViewMode::Reading,
            bookmark: None,
            progress: 0.4137,
        },
    );
    let pos = load_position(&file, "/mnt/onboard/Book.epub").unwrap();
    assert!(
        (pos.progress - 0.4137).abs() < 0.0005,
        "got {}",
        pos.progress
    );
}

#[test]
fn position_without_a_progress_field_still_loads() {
    let file = tmp("kothok_test_progress_legacy");
    std::fs::write(&file, "/mnt/onboard/Old.epub|2|5|10|20|0:0:0|r").unwrap();
    let pos = load_position(&file, "/mnt/onboard/Old.epub").expect("legacy line loads");
    assert_eq!(pos.chapter, 2);
    assert_eq!(pos.page, 5);
    assert_eq!(pos.progress, 0.0);
}

#[test]
fn progress_is_clamped_both_ways() {
    let file = tmp("kothok_test_progress_clamp");
    for (raw, want) in [("1.9", 1.0), ("-0.5", 0.0), ("garbage", 0.0)] {
        std::fs::write(&file, format!("/mnt/onboard/B.epub|0|0|0|0|0:0:0|r|{raw}")).unwrap();
        let pos = load_position(&file, "/mnt/onboard/B.epub").unwrap();
        assert_eq!(pos.progress, want, "raw {raw:?}");
    }
}

#[test]
fn progress_endpoints_are_exact() {
    let file = tmp("kothok_test_progress_ends");
    for p in [0.0f32, 1.0] {
        save_position(
            &file,
            "/mnt/onboard/B.epub",
            &ReadingPosition {
                chapter: 0,
                page: 0,
                cur_start: 0,
                cur_end: 0,
                view_mode: ViewMode::Reading,
                bookmark: None,
                progress: p,
            },
        );
        assert_eq!(
            load_position(&file, "/mnt/onboard/B.epub")
                .unwrap()
                .progress,
            p
        );
    }
}
