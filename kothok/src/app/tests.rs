// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::events::{resolve_progress_target, NavOutcome};

use super::*;
use crate::audio::Utterance;

fn utt(start: usize, end: usize) -> Utterance {
    Utterance {
        text: String::new(),
        start,
        end,
        para_end: false,
        page_break: None,
    }
}

/// An utterance whose text really is the body bytes `[start, end)`, so the
/// cursor-offset arithmetic in `queue_from_cursor` can be checked against it.
fn text_utt(start: usize, text: &str) -> Utterance {
    Utterance {
        text: text.to_string(),
        start,
        end: start + text.len(),
        para_end: false,
        page_break: None,
    }
}

#[test]
fn queue_from_cursor_drops_utterances_before_the_cursor() {
    let utts = vec![
        text_utt(0, "one. "),
        text_utt(5, "two. "),
        text_utt(10, "three."),
    ];
    let q = queue_from_cursor(10, utts);
    assert_eq!(q.len(), 1, "utterances above the cursor are dropped");
    assert_eq!(q[0].text, "three.");
}

#[test]
fn queue_from_cursor_trims_head_to_the_cursor() {
    // Cursor sits 4 bytes into the second utterance.
    let utts = vec![text_utt(0, "one. "), text_utt(5, "hello world")];
    let q = queue_from_cursor(9, utts);
    assert_eq!(q.len(), 1);
    assert_eq!(q[0].text, "o world", "head starts at the cursor");
    assert_eq!(q[0].start, 9, "start moves with the trimmed text");
    assert_eq!(q[0].end, 16, "end is unchanged");
}

#[test]
fn queue_from_cursor_at_utterance_start_does_not_trim() {
    let utts = vec![text_utt(0, "one. "), text_utt(5, "two words")];
    let q = queue_from_cursor(5, utts);
    assert_eq!(
        q[0].text, "two words",
        "cursor on the boundary reads it whole"
    );
    assert_eq!(q[0].start, 5);
}

#[test]
fn queue_from_cursor_shifts_page_break_by_the_trim() {
    let mut u = text_utt(5, "hello world");
    // The page ends 8 bytes into this utterance.
    u.page_break = Some(8);
    let q = queue_from_cursor(9, vec![u]);
    assert_eq!(
        q[0].page_break,
        Some(4),
        "page break is relative to the trimmed start"
    );
}

#[test]
fn queue_from_cursor_ignores_a_cursor_from_another_page() {
    // A cursor left behind on a later page is past everything here; trimming by
    // that difference would cut the first sentence at a meaningless byte.
    let utts = vec![text_utt(0, "one. "), text_utt(5, "two words")];
    let q = queue_from_cursor(9_999, utts);
    assert_eq!(q.len(), 2, "no utterance dropped");
    assert_eq!(q[0].text, "one. ", "head untouched");
    assert_eq!(q[0].start, 0);
}

#[test]
fn queue_from_cursor_never_splits_a_multibyte_char() {
    // "Bangla" text: every char is 3 bytes, so a cursor 1 byte in is not a
    // char boundary and the head must be left whole rather than panicking.
    let utts = vec![text_utt(0, "\u{0986}\u{09AE}\u{09BF}")];
    let q = queue_from_cursor(1, utts);
    assert_eq!(q[0].text, "\u{0986}\u{09AE}\u{09BF}");
    assert_eq!(q[0].start, 0);
}

#[test]
fn queue_from_cursor_empty_page_stays_empty() {
    assert!(queue_from_cursor(42, Vec::new()).is_empty());
}

#[test]
fn resolve_start_target_finds_containing_utterance() {
    let utts = [utt(0, 10), utt(10, 20), utt(20, 30)];
    assert_eq!(resolve_start_target(5, &utts), 0);
    assert_eq!(resolve_start_target(15, &utts), 1);
    assert_eq!(resolve_start_target(25, &utts), 2);
}

#[test]
fn resolve_start_target_at_boundary_starts_next() {
    let utts = [utt(0, 10), utt(10, 20)];
    assert_eq!(resolve_start_target(10, &utts), 1, "start of utt 1");
}

#[test]
fn resolve_start_target_outside_returns_zero() {
    let utts = [utt(10, 20), utt(30, 40)];
    assert_eq!(
        resolve_start_target(0, &utts),
        0,
        "before first -> fallback"
    );
    assert_eq!(resolve_start_target(25, &utts), 0, "in gap -> fallback");
    assert_eq!(resolve_start_target(50, &utts), 0, "past last -> fallback");
}

#[test]
fn resolve_start_target_empty_returns_zero() {
    assert_eq!(resolve_start_target(5, &[]), 0);
}

#[test]
fn sleep_plan_nevers_powers_bt_when_off() {
    // BT off (or no adapter) MUST keep bt_off=false - the dbus call hangs
    // on BT-less devices, which previously stalled enter_sleep entirely.
    let plan = sleep_plan(&None, false, false);
    assert!(!plan.bt_off, "bt off -> must not call bt_toggle");
}

#[test]
fn progress_target_zero_maps_to_first_chapter_first_page() {
    let offsets = [0, 5, 10, 15];
    let (c, lp) = resolve_progress_target(0, &offsets, 3);
    assert_eq!(c, 0);
    assert_eq!(lp, 0);
}

#[test]
fn progress_target_max_maps_to_last_chapter() {
    let offsets = [0, 5, 10, 15];
    let (c, _lp) = resolve_progress_target(1000, &offsets, 3);
    assert_eq!(c, 2, "1000 per-mille should land in the last chapter");
}

#[test]
fn progress_target_midpoint_splits_chapters_correctly() {
    // 3 chapters of 5 pages each: offsets = [0, 5, 10, 15]
    // 500 per-mille -> global = 500 * 15 / 1000 = 7
    // chapter 0 covers [0,5), chapter 1 covers [5,10) -> c=1, local=2
    let offsets = [0, 5, 10, 15];
    let (c, lp) = resolve_progress_target(500, &offsets, 3);
    assert_eq!(c, 1);
    assert_eq!(lp, 2);
}

#[test]
fn progress_target_boundary_lands_in_correct_chapter() {
    // global = 5 should land at start of chapter 1 (offsets[1] = 5)
    // pt such that pt * 15 / 1000 = 5 -> pt = 334 (ceil(5000/15))
    let offsets = [0, 5, 10, 15];
    let (c, lp) = resolve_progress_target(334, &offsets, 3);
    assert_eq!(c, 1);
    assert_eq!(
        lp, 0,
        "landing exactly on a chapter boundary starts that chapter"
    );
}

#[test]
fn progress_target_single_chapter() {
    let offsets = [0, 10];
    let (c, lp) = resolve_progress_target(500, &offsets, 1);
    assert_eq!(c, 0);
    assert_eq!(lp, 5);
}

#[test]
fn progress_target_clamps_beyond_chapter_count() {
    // chapter_count=2 but offsets has entries for 3 chapters
    let offsets = [0, 5, 10, 15];
    let (c, _lp) = resolve_progress_target(1000, &offsets, 2);
    assert_eq!(c, 1, "should clamp to chapter_count-1");
}

#[test]
fn nav_outcome_defaults_all_false() {
    let o = NavOutcome {
        navigated: false,
        text_dirty: false,
        ui_changed: false,
    };
    assert!(!o.navigated);
    assert!(!o.text_dirty);
    assert!(!o.ui_changed);
}

#[test]
fn friendly_error_a2dp_maps_to_speaker_message() {
    assert_eq!(
        friendly_error("A2DP connect failed"),
        "Speaker not connected - check Bluetooth"
    );
    assert_eq!(
        friendly_error("no speaker endpoint"),
        "Speaker not connected - check Bluetooth"
    );
}

#[test]
fn friendly_error_is_case_insensitive() {
    assert_eq!(
        friendly_error("A2dp Stream Error"),
        "Speaker not connected - check Bluetooth"
    );
    assert_eq!(
        friendly_error("TTS synthesis aborted"),
        "WiFi unavailable - can't reach the voice service"
    );
}

#[test]
fn friendly_error_network_or_tts_maps_to_wifi_message() {
    for msg in [
        "ws connect timeout",
        "lookup address failed",
        "try again later",
        "synth buffer empty",
        "tts endpoint unreachable",
    ] {
        assert_eq!(
            friendly_error(msg),
            "WiFi unavailable - can't reach the voice service",
            "message {msg:?} should map to the WiFi warning"
        );
    }
}

#[test]
fn friendly_error_unknown_falls_back_to_generic() {
    assert_eq!(
        friendly_error("disk write protected"),
        "Playback error - see log"
    );
    assert_eq!(friendly_error(""), "Playback error - see log");
}

#[test]
fn sleep_plan_from_book_powers_down() {
    let plan = sleep_plan(&Some(std::path::PathBuf::from("/sys/bl")), true, true);
    assert!(
        plan.frontlight_off,
        "frontlight powers off when a path exists"
    );
    assert!(plan.wifi_off, "wifi powers off when it was on");
    assert!(plan.bt_off, "bt powers off when it was on");
}

#[test]
fn sleep_plan_keeps_frontlight_when_no_path() {
    let plan = sleep_plan(&None, false, false);
    assert!(
        !plan.frontlight_off,
        "no frontlight path -> leave the frontlight alone"
    );
}

#[test]
fn sleep_plan_leaves_wifi_when_already_off() {
    let plan = sleep_plan(&None, false, false);
    assert!(!plan.wifi_off, "wifi already off -> no redundant toggle");
}
