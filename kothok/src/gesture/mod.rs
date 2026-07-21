// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::rendering::render::{
    chapter_list_hit_test, GridCell, LibraryFilter, PillRect, PICKER_HEADER_H,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FooterZone {
    None,
    ProgressBar,
    PlayPause,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderZone {
    None,
    Library,
    ModeToggle,
    Bookmark,
    JumpToBookmark,
    /// Reading mode's counterpart to audio mode's Lock: same header slot.
    Sleep,
    Menu,
    Chapters,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerTarget {
    None,
    Logo,
    Exit,
    Book(usize),
    Filter(LibraryFilter),
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

/// Hit-test the reading-mode header. Must track `content.slint`'s header, which
/// shares one geometry with AudioPlayer and ControlPanel: 110px tall, 76px
/// buttons, 23px edge padding, 10px gaps, 38px gap before the system pair.
/// Right-to-left: Chapters @973, Settings @887, Sleep @801, Separator @790,
/// Jump @697, Bookmark @611, ModeToggle @525 (for w=1072); Library @23.
pub fn classify_header_zone(dx: f32, dy: f32, w: f32) -> HeaderZone {
    const HEADER_H: f32 = 110.0;
    const BTN: f32 = 76.0;
    const GAP: f32 = 10.0;
    const PAD: f32 = 23.0;
    const SEP: f32 = 38.0;
    if dy >= HEADER_H {
        return HeaderZone::None;
    }
    if dx < PAD + BTN {
        return HeaderZone::Library;
    }
    let mut left = w - PAD - BTN;
    if dx >= left {
        return HeaderZone::Chapters;
    }
    left -= GAP + BTN;
    if dx >= left {
        return HeaderZone::Menu;
    }
    left -= GAP + BTN;
    if dx >= left {
        return HeaderZone::Sleep;
    }
    left -= SEP;
    if dx >= left {
        return HeaderZone::None;
    }
    left -= BTN;
    if dx >= left {
        return HeaderZone::JumpToBookmark;
    }
    left -= GAP + BTN;
    if dx >= left {
        return HeaderZone::Bookmark;
    }
    left -= GAP + BTN;
    if dx >= left {
        return HeaderZone::ModeToggle;
    }
    HeaderZone::None
}

/// How long the screenshot hold must last. Well past any tap or swipe, so the
/// gesture stays unambiguous.
#[cfg(feature = "screenshot")]
pub const SCREENSHOT_HOLD_MS: u128 = 2000;
/// Finger drift tolerated over the hold. A swipe leaves this band immediately.
#[cfg(feature = "screenshot")]
pub const SCREENSHOT_DRIFT_PX: f32 = 30.0;
/// Side of the bottom-left corner square that arms a capture. The header is
/// full of buttons (library, chapters, mode-toggle, ...), so the capture zone
/// moved to a corner no control occupies. A press here is withheld from the
/// tap path entirely, so it never collides with a seek-bar scrub.
#[cfg(feature = "screenshot")]
const SCREENSHOT_CORNER: f32 = 140.0;

/// The capture zone: the bottom-left corner of the screen.
#[cfg(feature = "screenshot")]
pub fn is_in_screenshot_zone(dx: f32, dy: f32, _w: f32, h: f32) -> bool {
    dx < SCREENSHOT_CORNER && dy > h - SCREENSHOT_CORNER
}

/// Screenshot gesture: hold the bottom-left corner still for 2s.
///
/// Tested while the finger is still down, not on release. The library grid acts
/// on `tap_xy` at press time and audio mode forwards the press straight to
/// Slint, so a release-time check would fire only after those screens had
/// already moved on.
#[cfg(feature = "screenshot")]
pub fn is_screenshot_hold(
    press_dx: f32,
    press_dy: f32,
    cur_dx: f32,
    cur_dy: f32,
    dt_ms: u128,
    w: f32,
    h: f32,
) -> bool {
    if dt_ms < SCREENSHOT_HOLD_MS {
        return false;
    }
    if !is_in_screenshot_zone(press_dx, press_dy, w, h)
        || !is_in_screenshot_zone(cur_dx, cur_dy, w, h)
    {
        return false;
    }
    (cur_dx - press_dx).abs() <= SCREENSHOT_DRIFT_PX
        && (cur_dy - press_dy).abs() <= SCREENSHOT_DRIFT_PX
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
    pills: &[PillRect],
    screen_w: f32,
    nav_touch_top: f32,
    bezel_top: f32,
) -> PickerTarget {
    if dy < PICKER_HEADER_H as f32 {
        const EXIT_BTN_PX: f32 = 76.0;
        const EXIT_PAD: f32 = 23.0;
        const EXIT_TOP: f32 = 17.0;
        let exit_left = screen_w - EXIT_BTN_PX - EXIT_PAD;
        let exit_right = screen_w - EXIT_PAD;
        if dx >= exit_left && dx < exit_right && dy >= EXIT_TOP && dy < EXIT_TOP + EXIT_BTN_PX {
            return PickerTarget::Exit;
        }
        const LOGO_PX: f32 = 76.0;
        const LOGO_X: f32 = 23.0;
        const LOGO_Y: f32 = 17.0;
        if dx >= LOGO_X && dx < LOGO_X + LOGO_PX && dy >= LOGO_Y && dy < LOGO_Y + LOGO_PX {
            return PickerTarget::Logo;
        }
        return PickerTarget::None;
    }
    if dy >= nav_touch_top && dy < bezel_top {
        return PickerTarget::None;
    }
    // Pills sit above the grid and never overlap it, but they are tested first
    // so a tall pill can grow into the gap without being shadowed by a card.
    for pill in pills {
        if dx >= pill.x as f32
            && dx < (pill.x + pill.w) as f32
            && dy >= pill.y as f32
            && dy < (pill.y + pill.h) as f32
        {
            return PickerTarget::Filter(pill.filter);
        }
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
mod tests;
