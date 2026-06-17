use crate::rendering::render::{chapter_list_hit_test, GridCell, NAV_EXIT_W, NAV_EXIT_X};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterZone {
    None,
    ProgressBar,
    PlayPause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerTarget {
    None,
    Exit,
    Book(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwipeDirection {
    None,
    Left,
    Right,
}

pub fn classify_footer_zone(
    dx: f32,
    dy: f32,
    pbar_y: f32,
    pbar_h: f32,
    pbar_x: f32,
    pbar_right: f32,
    pp_zone_x: f32,
) -> FooterZone {
    let in_band = dy >= pbar_y && dy < pbar_y + pbar_h;
    if !in_band {
        return FooterZone::None;
    }
    if dx >= pp_zone_x {
        FooterZone::PlayPause
    } else if dx >= pbar_x && dx <= pbar_right {
        FooterZone::ProgressBar
    } else {
        FooterZone::None
    }
}

pub fn classify_swipe(swipe_dx: f32, swipe_dy: f32, threshold: f32, dt_ms: u128) -> SwipeDirection {
    let horizontal = swipe_dx.abs() > threshold
        && dt_ms < crate::device::touch::SWIPE_MAX_MS
        && swipe_dx.abs() > swipe_dy.abs();
    if horizontal {
        if swipe_dx < 0.0 {
            return SwipeDirection::Left;
        }
        return SwipeDirection::Right;
    }
    SwipeDirection::None
}

pub fn picker_hit_test(
    dx: f32,
    dy: f32,
    cells: &[GridCell],
    nav_touch_top: f32,
    bezel_top: f32,
) -> PickerTarget {
    if dy >= nav_touch_top && dy < bezel_top {
        let dxi = dx as i32;
        if (NAV_EXIT_X..NAV_EXIT_X + NAV_EXIT_W).contains(&dxi) {
            return PickerTarget::Exit;
        }
        return PickerTarget::None;
    }
    for cell in cells {
        if dx >= cell.x as f32
            && dx < (cell.x + cell.w) as f32
            && dy >= cell.y as f32
            && dy < (cell.y + cell.h) as f32
        {
            return PickerTarget::Book(cell.idx);
        }
    }
    PickerTarget::None
}

pub fn picker_book_double_tap(
    idx: usize,
    last_idx: Option<usize>,
    now: std::time::Instant,
    last_tap_time: std::time::Instant,
    window: std::time::Duration,
) -> bool {
    last_idx == Some(idx) && now.duration_since(last_tap_time) < window
}

pub fn chapter_overlay_target(
    dy: f32,
    swipe_dy: f32,
    swipe_dx: f32,
    scroll: i32,
    chapter_count: usize,
) -> ChapterOverlayAction {
    if swipe_dy.abs() > 40.0 && swipe_dy.abs() > swipe_dx.abs() {
        ChapterOverlayAction::Scroll
    } else {
        match chapter_list_hit_test(dy as i32, scroll, chapter_count) {
            Some(idx) => ChapterOverlayAction::Select(idx),
            None => ChapterOverlayAction::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChapterOverlayAction {
    None,
    Scroll,
    Select(usize),
}

#[cfg(test)]
mod tests {
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
        let target = picker_hit_test(50.0, 1400.0, &[], 1350.0, 1446.0);
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
        let target = picker_hit_test(200.0, 300.0, &cells, 1350.0, 1446.0);
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
        let target = picker_hit_test(900.0, 300.0, &cells, 1350.0, 1446.0);
        assert_eq!(target, PickerTarget::None);
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
}
