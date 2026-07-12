use super::events::{resolve_progress_target, NavOutcome};

use super::*;

#[test]
fn sleep_plan_nevers_powers_bt_when_off() {
    // BT off (or no adapter) MUST keep bt_off=false - the dbus call hangs
    // on BT-less devices, which previously stalled enter_sleep entirely.
    let plan = sleep_plan(false, &None, false, false);
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
fn sleep_plan_from_book_shows_cover_and_powers_down() {
    let plan = sleep_plan(
        false,
        &Some(std::path::PathBuf::from("/sys/bl")),
        true,
        true,
    );
    assert!(plan.show_cover, "locking from a book shows its cover");
    assert!(
        plan.frontlight_off,
        "frontlight powers off when a path exists"
    );
    assert!(plan.wifi_off, "wifi powers off when it was on");
    assert!(plan.bt_off, "bt powers off when it was on");
}

#[test]
fn sleep_plan_from_picker_shows_splash() {
    let plan = sleep_plan(true, &Some(std::path::PathBuf::from("/sys/bl")), true, true);
    assert!(
        !plan.show_cover,
        "locking from the library shows the KoThok splash, not a cover"
    );
}

#[test]
fn sleep_plan_keeps_frontlight_when_no_path() {
    let plan = sleep_plan(false, &None, false, false);
    assert!(
        !plan.frontlight_off,
        "no frontlight path -> leave the frontlight alone"
    );
}

#[test]
fn sleep_plan_leaves_wifi_when_already_off() {
    let plan = sleep_plan(false, &None, false, false);
    assert!(!plan.wifi_off, "wifi already off -> no redundant toggle");
}
