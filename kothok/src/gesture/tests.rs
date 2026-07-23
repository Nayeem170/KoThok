use super::*;

#[test]
fn footer_zone_none_above_band() {
    assert_eq!(
        classify_footer_zone(100.0, 10.0, 1300.0, 70.0, 60.0, 900.0, 900.0),
        FooterZone::None
    );
}

#[test]
fn footer_zone_progress_bar() {
    assert_eq!(
        classify_footer_zone(500.0, 1350.0, 1300.0, 70.0, 60.0, 900.0, 900.0),
        FooterZone::ProgressBar
    );
}

#[test]
fn footer_zone_play_pause() {
    assert_eq!(
        classify_footer_zone(950.0, 1350.0, 1300.0, 70.0, 60.0, 900.0, 900.0),
        FooterZone::PlayPause
    );
}

#[test]
fn footer_zone_left_of_bar_is_none() {
    assert_eq!(
        classify_footer_zone(10.0, 1350.0, 1300.0, 70.0, 60.0, 900.0, 900.0),
        FooterZone::None
    );
}

#[test]
fn header_zone_library_top_left() {
    assert_eq!(
        classify_header_zone(20.0, 20.0, 1072.0),
        HeaderZone::Library
    );
}

#[test]
fn header_zone_menu_top_right() {
    assert_eq!(classify_header_zone(925.0, 20.0, 1072.0), HeaderZone::Menu);
}

#[test]
fn header_zone_chapters_top_right() {
    assert_eq!(
        classify_header_zone(1011.0, 20.0, 1072.0),
        HeaderZone::Chapters
    );
}

// Zones for w=1072 (76px buttons, 10px gaps within groups,
// 38px separator gap between book buttons and system buttons):
// ModeToggle 515..591, Bookmark 601..677, JumpToBookmark 687..763,
// [separator 763..801], Sleep 801..877, Menu 887..963, Chapters 973..1049.
#[test]
fn header_zone_mode_toggle() {
    assert_eq!(
        classify_header_zone(553.0, 20.0, 1072.0),
        HeaderZone::ModeToggle
    );
}

#[test]
fn header_zone_bookmark() {
    assert_eq!(
        classify_header_zone(639.0, 20.0, 1072.0),
        HeaderZone::Bookmark
    );
}

#[test]
fn header_zone_jump_to_bookmark() {
    assert_eq!(
        classify_header_zone(725.0, 20.0, 1072.0),
        HeaderZone::JumpToBookmark
    );
}

#[test]
fn header_zone_sleep() {
    assert_eq!(classify_header_zone(839.0, 20.0, 1072.0), HeaderZone::Sleep);
}

#[test]
fn header_zone_center_is_none() {
    assert_eq!(classify_header_zone(500.0, 20.0, 1072.0), HeaderZone::None);
}

#[test]
fn header_zone_below_band_is_none() {
    assert_eq!(classify_header_zone(20.0, 120.0, 1072.0), HeaderZone::None);
}

#[test]
#[cfg(feature = "screenshot")]
fn screenshot_zone_is_the_bottom_left_corner() {
    // Screen is 1072x1448. The zone gates whether a press is withheld from
    // the tap path, so a false positive would eat real taps.
    // Inside the corner.
    assert!(is_in_screenshot_zone(50.0, 1400.0, 1072.0, 1448.0));
    // Outside: top-left (header library button area).
    assert!(!is_in_screenshot_zone(30.0, 55.0, 1072.0, 1448.0));
    // Outside: header centre, where the audio mode-toggle lives.
    assert!(!is_in_screenshot_zone(536.0, 55.0, 1072.0, 1448.0));
    // Outside: bottom-right corner.
    assert!(!is_in_screenshot_zone(1011.0, 1400.0, 1072.0, 1448.0));
    // Outside: bottom edge but right of the corner.
    assert!(!is_in_screenshot_zone(536.0, 1400.0, 1072.0, 1448.0));
}

#[test]
#[cfg(feature = "screenshot")]
fn screenshot_tap_in_bottom_left_corner_captures() {
    // A quick tap (or a longer hold) in the corner captures - the press is
    // withheld from the tap path, so duration no longer matters.
    assert!(is_screenshot_hold(70.0, 1410.0, 73.0, 1412.0, 2100, 1072.0, 1448.0));
    assert!(is_screenshot_hold(70.0, 1410.0, 70.0, 1410.0, 120, 1072.0, 1448.0));
}

#[test]
#[cfg(feature = "screenshot")]
fn screenshot_hold_rejected_outside_the_corner() {
    // The old header-centre zone must NOT trigger now (the audio mode-toggle
    // sits there at x~515).
    assert!(!is_screenshot_hold(536.0, 55.0, 536.0, 55.0, 2100, 1072.0, 1448.0));
    // Bottom-right corner.
    assert!(!is_screenshot_hold(1011.0, 1400.0, 1011.0, 1400.0, 2100, 1072.0, 1448.0));
    // Mid-screen.
    assert!(!is_screenshot_hold(50.0, 700.0, 50.0, 700.0, 2100, 1072.0, 1448.0));
}

#[test]
#[cfg(feature = "screenshot")]
fn screenshot_hold_rejected_when_finger_drifts() {
    // A slow swipe starting in the corner is still a swipe, not a capture.
    assert!(!is_screenshot_hold(70.0, 1410.0, 400.0, 1410.0, 2100, 1072.0, 1448.0));
}

#[test]
fn classify_swipe_left() {
    assert_eq!(classify_swipe(-200.0, 5.0, 60.0, 120), SwipeDirection::Left);
}

#[test]
fn classify_swipe_right() {
    assert_eq!(classify_swipe(200.0, 5.0, 60.0, 120), SwipeDirection::Right);
}

#[test]
fn classify_swipe_none_small() {
    assert_eq!(classify_swipe(10.0, 10.0, 60.0, 120), SwipeDirection::None);
}

#[test]
fn classify_swipe_none_too_slow() {
    assert_eq!(classify_swipe(200.0, 5.0, 60.0, 600), SwipeDirection::None);
}

#[test]
fn picker_hit_test_exit_zone() {
    let sw = 1072.0f32;
    let exit_x = sw - 48.0 - 18.0;
    let target = picker_hit_test(exit_x + 24.0, 30.0, &[], &[], sw, 1350.0, 1446.0);
    assert_eq!(target, PickerTarget::Exit);
}

#[test]
fn picker_hit_test_book_cell() {
    let cells = vec![GridCell {
        x: 100,
        y: 200,
        w: 300,
        h: 400,
        idx: 3,
    }];
    let target = picker_hit_test(200.0, 300.0, &cells, &[], 1072.0, 1350.0, 1446.0);
    assert_eq!(target, PickerTarget::Book(3));
}

#[test]
fn picker_hit_test_miss_returns_none() {
    let cells = vec![GridCell {
        x: 100,
        y: 200,
        w: 300,
        h: 400,
        idx: 0,
    }];
    let target = picker_hit_test(500.0, 300.0, &cells, &[], 1072.0, 1350.0, 1446.0);
    assert_eq!(target, PickerTarget::None);
}

fn pill(filter: LibraryFilter, x: i32) -> PillRect {
    PillRect {
        filter,
        x,
        y: 120,
        w: 140,
        h: 50,
    }
}

#[test]
fn picker_hit_test_filter_pill() {
    let pills = vec![
        pill(LibraryFilter::All, 10),
        pill(LibraryFilter::Reading, 160),
    ];
    let target = picker_hit_test(200.0, 140.0, &[], &pills, 1072.0, 1350.0, 1446.0);
    assert_eq!(target, PickerTarget::Filter(LibraryFilter::Reading));
}

#[test]
fn picker_hit_test_between_pills_is_none() {
    let pills = vec![
        pill(LibraryFilter::All, 10),
        pill(LibraryFilter::Reading, 160),
    ];
    // The gap between two pills must not select either one.
    let target = picker_hit_test(155.0, 140.0, &[], &pills, 1072.0, 1350.0, 1446.0);
    assert_eq!(target, PickerTarget::None);
}

#[test]
fn picker_pill_does_not_shadow_a_card() {
    // A pill and a card can never overlap, but the ordering is load-bearing:
    // a tap below the pill band must still reach the card under it.
    let pills = vec![pill(LibraryFilter::All, 100)];
    let cells = vec![GridCell {
        x: 100,
        y: 200,
        w: 300,
        h: 400,
        idx: 7,
    }];
    let target = picker_hit_test(150.0, 300.0, &cells, &pills, 1072.0, 1350.0, 1446.0);
    assert_eq!(target, PickerTarget::Book(7));
}

#[test]
fn double_tap_detected_within_window() {
    let now = std::time::Instant::now();
    let earlier = now - std::time::Duration::from_millis(200);
    assert!(picker_book_double_tap(
        5,
        Some(5),
        now,
        earlier,
        std::time::Duration::from_millis(450)
    ));
}

#[test]
fn double_tap_rejected_different_book() {
    let now = std::time::Instant::now();
    assert!(!picker_book_double_tap(
        5,
        Some(3),
        now,
        now,
        std::time::Duration::from_millis(450)
    ));
}
