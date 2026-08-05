// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;

use crate::device::input::{
    ABS_MT_POSITION_X, ABS_MT_POSITION_Y, BTN_TOUCH_CODE, EV_ABS, EV_KEY, EV_SYN, SYN_REPORT,
};
use crate::rendering::chapter_list::{
    list_scroll_max, scrollbar_hit_test, scrollbar_y_to_scroll, CH_LIST_BOTTOM_PAD, CH_LIST_TOP,
};
#[cfg(feature = "screenshot")]
use crate::rendering::common::rgb565_as_bytes_ref;
use std::io::Read;

pub(super) fn poll_and_dispatch_touch(st: &mut LoopState, ctx: &mut LoopContext) -> bool {
    let reader = ctx.reader;
    let cb = ctx.cb;
    let to_display = |rx: i32, ry: i32| -> (f32, f32) { touch::to_display(rx, ry, ctx.touch_cfg) };
    let pbar_y: f32 = ctx.h as f32 - layout::FOOTER_H_F;
    let pbar_x: f32 = (layout::pad_left() + layout::GUTTER_W + layout::GUTTER_PAD) as f32 + 8.0;
    let pbar_w: f32 = ctx.w as f32 - 200.0;
    let pbar_right: f32 = pbar_x + pbar_w;
    let pp_zone_x: f32 = pbar_right;
    let tap_cooldown = std::time::Duration::from_millis(TAP_COOLDOWN_MS);
    let mut rec = [0u8; 16];
    let mut had_event = false;

    // Locked rejects all touch except a swipe-up to unlock, mirroring the
    // wake-from-sleep gesture. Track the press/release Y directly rather than
    // running the full gesture pipeline, which would dispatch taps to Slint.
    if st.system_state == SystemState::Locked {
        let mut unlock = false;
        while let Ok(16) = ctx.touch_dev.read(&mut rec) {
            let typ = u16::from_le_bytes([rec[8], rec[9]]);
            let code = u16::from_le_bytes([rec[10], rec[11]]);
            let val = i32::from_le_bytes([rec[12], rec[13], rec[14], rec[15]]);
            match (typ, code) {
                (EV_KEY, BTN_TOUCH_CODE) => st.frame_down = val == 1,
                (EV_ABS, ABS_MT_POSITION_X) => st.frame_x = val,
                (EV_ABS, ABS_MT_POSITION_Y) => st.frame_y = val,
                (EV_SYN, SYN_REPORT) => {
                    if st.frame_down && !st.prev_down {
                        st.press_x = st.frame_x;
                        st.press_y = st.frame_y;
                    } else if !st.frame_down && st.prev_down {
                        let (press_dx, press_dy) = to_display(st.press_x, st.press_y);
                        let (rel_dx, rel_dy) = to_display(st.frame_x, st.frame_y);
                        if kobo_core::device::wake::is_wake_swipe(
                            press_dx,
                            press_dy,
                            rel_dx,
                            rel_dy,
                            ctx.h as f32,
                        ) {
                            unlock = true;
                        }
                    }
                    st.prev_down = st.frame_down;
                }
                _ => {}
            }
        }
        if unlock {
            st.system_state = SystemState::Awake;
            st.lock_time = None;
            reader.set_audio_locked(false);
            if let Some(path) = ctx.fl_path.as_deref() {
                crate::device::power::restore_frontlight(path, st.saved_brightness);
            }
            super::power::unlock_radios(st, ctx);
            st.last_activity = std::time::Instant::now();
            st.text_dirty = true;
            info!("UNLOCK (swipe-up)");
            return true;
        }
        return false;
    }

    let audio_mode = matches!(st.view_mode, ViewMode::Audio)
        && !st.picker_active
        && !st.panel_open
        && !reader.get_chapter_overlay_open();
    {
        let mut pfd = libc::pollfd {
            fd: ctx.touch_fd,
            events: libc::POLLIN,
            revents: 0,
        };
        const LOOP_POLL_MS: libc::c_int = 100;
        // SAFETY: poll takes a single initialized `pollfd` on our stack frame pointing
        // at the caller-owned touch_fd. It only writes `revents` (no aliasing) and
        // returns an int count; LOOP_POLL_MS bounds the wait.
        unsafe {
            libc::poll(&mut pfd, 1, LOOP_POLL_MS);
        }
    }
    while let Ok(16) = ctx.touch_dev.read(&mut rec) {
        had_event = true;
        let typ = u16::from_le_bytes([rec[8], rec[9]]);
        let code = u16::from_le_bytes([rec[10], rec[11]]);
        let val = i32::from_le_bytes([rec[12], rec[13], rec[14], rec[15]]);
        match (typ, code) {
            (EV_KEY, BTN_TOUCH_CODE) => st.frame_down = val == 1,
            (EV_ABS, ABS_MT_POSITION_X) => st.frame_x = val,
            (EV_ABS, ABS_MT_POSITION_Y) => st.frame_y = val,
            (EV_SYN, SYN_REPORT) => {
                let now = std::time::Instant::now();
                let (dx, dy) = to_display(st.frame_x, st.frame_y);
                if st.frame_down && !st.prev_down {
                    st.press_x = st.frame_x;
                    st.press_y = st.frame_y;
                    st.press_time = now;
                    st.press_chapter_scroll = st.chapter_scroll;
                    st.press_search_scroll = st.search_scroll;
                    st.press_search_results_scroll = st.search_results_scroll;
                    #[cfg(feature = "screenshot")]
                    {
                        st.shot_armed =
                            gesture::is_in_screenshot_zone(dx, dy, ctx.w as f32, ctx.h as f32);
                        st.shot_done = false;
                        if st.shot_armed {
                            st.prev_down = st.frame_down;
                            continue;
                        }
                    }
                    let footer_zone = if !st.picker_active
                        && !st.panel_open
                        && !reader.get_chapter_overlay_open()
                        && !audio_mode
                    {
                        gesture::classify_footer_zone(
                            dx, dy, pbar_y, PBAR_H, pbar_x, pbar_right, pp_zone_x,
                        )
                    } else {
                        gesture::FooterZone::None
                    };
                    match footer_zone {
                        gesture::FooterZone::PlayPause => {
                            st.pp_pressed = true;
                        }
                        gesture::FooterZone::ProgressBar => {
                            let frac = ((dx - pbar_x) / pbar_w).clamp(0.0, 1.0);
                            cb.progress_target.set((frac * 1000.0) as i32);
                            st.scrubbing = true;
                        }
                        gesture::FooterZone::None => {}
                    }
                    let header_zone = if !st.picker_active
                        && !st.panel_open
                        && !reader.get_chapter_overlay_open()
                        && !st.cover_page_visible
                        && st.header_visible
                        && !audio_mode
                    {
                        gesture::classify_header_zone(dx, dy, ctx.w as f32)
                    } else {
                        gesture::HeaderZone::None
                    };
                    match header_zone {
                        gesture::HeaderZone::Library => st.lib_pressed = true,
                        gesture::HeaderZone::Menu => st.menu_pressed = true,
                        gesture::HeaderZone::ModeToggle => st.mode_toggle_pressed = true,
                        gesture::HeaderZone::Bookmark => st.bookmark_set_pressed = true,
                        gesture::HeaderZone::JumpToBookmark => st.bookmark_jump_pressed = true,
                        gesture::HeaderZone::Sleep => st.sleep_pressed = true,
                        gesture::HeaderZone::Chapters => st.chapter_pressed = true,
                        gesture::HeaderZone::None => {}
                    }
                    let near = st.last_tap_y >= 0
                        && (st.frame_y - st.last_tap_y).abs() < SWIPE_DELTA_TOLERANCE_PX;
                    if (now.duration_since(st.last_tap_time) >= tap_cooldown || !near)
                        && !st.scrubbing
                        && !st.pp_pressed
                        && !st.lib_pressed
                        && !st.menu_pressed
                        && !st.mode_toggle_pressed
                        && !st.bookmark_set_pressed
                        && !st.bookmark_jump_pressed
                        && !st.sleep_pressed
                        && !st.chapter_pressed
                    {
                        st.tap_xy = Some((dx, dy));
                        st.last_tap_time = now;
                        st.last_tap_y = st.frame_y;
                        // The control panel is a Slint overlay in *both* reading
                        // and library mode, so its presses have to reach Slint
                        // even while the (Rust-drawn) picker owns the screen.
                        // Gating the whole dispatch on `!picker_active` left
                        // every row of the device-settings panel dead: WiFi, BT,
                        // sleep, the sliders and the back button all hang off
                        // Slint TouchAreas that never saw a pointer event.
                        if !st.picker_active || st.panel_open {
                            if audio_mode {
                                st.press_dispatched = true;
                                ctx.window.window().dispatch_event(
                                    slint::platform::WindowEvent::PointerPressed {
                                        position: slint::LogicalPosition::new(dx, dy),
                                        button: slint::platform::PointerEventButton::Left,
                                    },
                                );
                            } else if !st.panel_open && !reader.get_chapter_overlay_open() {
                                st.press_dispatched = false;
                            } else if reader.get_chapter_overlay_open() {
                                let list_top = CH_LIST_TOP;
                                let list_bottom = ctx.h as i32 - CH_LIST_BOTTOM_PAD;
                                let (item_count, cur_scroll) = if st.search_results_active {
                                    let hc = st
                                        .word_index
                                        .occurrences
                                        .get(st.search_selected_word)
                                        .map(|h| {
                                            h.len().min(crate::data::word_index::MAX_SEARCH_RESULTS)
                                        })
                                        .unwrap_or(0);
                                    (hc, st.search_results_scroll)
                                } else {
                                    match st.chapter_tab {
                                        crate::loop_state::ChapterTab::Words => {
                                            (st.word_index.words.len(), st.search_scroll)
                                        }
                                        crate::loop_state::ChapterTab::Marks => {
                                            (st.marks.len(), st.marks_scroll)
                                        }
                                        crate::loop_state::ChapterTab::Chapters => {
                                            (st.toc_rows.len(), st.chapter_scroll)
                                        }
                                    }
                                };
                                let sb_dx = dx as i32;
                                let sb_dy = dy as i32;
                                if let Some(zone) = scrollbar_hit_test(
                                    ctx.w,
                                    list_top,
                                    list_bottom,
                                    sb_dx,
                                    sb_dy,
                                    item_count,
                                    cur_scroll,
                                ) {
                                    st.press_dispatched = false;
                                    let track_h = list_bottom - list_top;
                                    match zone {
                                        crate::rendering::chapter_list::ScrollbarZone::Thumb => {
                                            st.sb_dragging = true;
                                            st.sb_drag_tab = st.chapter_tab;
                                            st.sb_grab_offset = sb_dy
                                                - crate::rendering::chapter_list::thumb_top(
                                                    list_top, track_h, item_count, cur_scroll,
                                                );
                                            let new_scroll = scrollbar_y_to_scroll(
                                                sb_dy,
                                                list_top,
                                                list_bottom,
                                                item_count,
                                                st.sb_grab_offset,
                                            );
                                            if st.search_results_active {
                                                st.search_results_scroll = new_scroll;
                                                st.press_search_results_scroll = new_scroll;
                                            } else {
                                                match st.chapter_tab {
                                                    crate::loop_state::ChapterTab::Words => {
                                                        st.search_scroll = new_scroll;
                                                        st.press_search_scroll = new_scroll;
                                                    }
                                                    crate::loop_state::ChapterTab::Marks => {
                                                        st.marks_scroll = new_scroll;
                                                        st.press_marks_scroll = new_scroll;
                                                    }
                                                    crate::loop_state::ChapterTab::Chapters => {
                                                        st.chapter_scroll = new_scroll;
                                                        st.press_chapter_scroll = new_scroll;
                                                    }
                                                }
                                            }
                                            st.text_dirty = true;
                                            ctx.window.request_redraw();
                                        }
                                        crate::rendering::chapter_list::ScrollbarZone::Rail(
                                            tap_y,
                                        ) => {
                                            let visible_rows = track_h
                                                / crate::rendering::chapter_list::CH_ROW_PITCH;
                                            let delta = visible_rows
                                                * crate::rendering::chapter_list::CH_ROW_PITCH;
                                            let sm =
                                                crate::rendering::chapter_list::list_scroll_max(
                                                    item_count, track_h,
                                                );
                                            if tap_y < list_top + track_h / 2 {
                                                let new_scroll = (cur_scroll - delta).max(0);
                                                if st.search_results_active {
                                                    st.search_results_scroll = new_scroll;
                                                    st.press_search_results_scroll = new_scroll;
                                                } else {
                                                    match st.chapter_tab {
                                                        crate::loop_state::ChapterTab::Words => {
                                                            st.search_scroll = new_scroll;
                                                            st.press_search_scroll = new_scroll;
                                                        }
                                                        crate::loop_state::ChapterTab::Marks => {
                                                            st.marks_scroll = new_scroll;
                                                            st.press_marks_scroll = new_scroll;
                                                        }
                                                        crate::loop_state::ChapterTab::Chapters => {
                                                            st.chapter_scroll = new_scroll;
                                                            st.press_chapter_scroll = new_scroll;
                                                        }
                                                    }
                                                }
                                            } else {
                                                let new_scroll = (cur_scroll + delta).min(sm);
                                                if st.search_results_active {
                                                    st.search_results_scroll = new_scroll;
                                                    st.press_search_results_scroll = new_scroll;
                                                } else {
                                                    match st.chapter_tab {
                                                        crate::loop_state::ChapterTab::Words => {
                                                            st.search_scroll = new_scroll;
                                                            st.press_search_scroll = new_scroll;
                                                        }
                                                        crate::loop_state::ChapterTab::Marks => {
                                                            st.marks_scroll = new_scroll;
                                                            st.press_marks_scroll = new_scroll;
                                                        }
                                                        crate::loop_state::ChapterTab::Chapters => {
                                                            st.chapter_scroll = new_scroll;
                                                            st.press_chapter_scroll = new_scroll;
                                                        }
                                                    }
                                                }
                                            }
                                            st.text_dirty = true;
                                            ctx.window.request_redraw();
                                        }
                                    }
                                } else {
                                    let list_bottom = ctx.h as i32 - CH_LIST_BOTTOM_PAD;
                                    if (dy as i32) >= list_bottom {
                                        st.press_dispatched = false;
                                    } else {
                                        st.press_dispatched = true;
                                        ctx.window.window().dispatch_event(
                                            slint::platform::WindowEvent::PointerPressed {
                                                position: slint::LogicalPosition::new(dx, dy),
                                                button: slint::platform::PointerEventButton::Left,
                                            },
                                        );
                                    }
                                }
                            } else {
                                st.press_dispatched = true;
                                ctx.window.window().dispatch_event(
                                    slint::platform::WindowEvent::PointerPressed {
                                        position: slint::LogicalPosition::new(dx, dy),
                                        button: slint::platform::PointerEventButton::Left,
                                    },
                                );
                            }
                        } else {
                            st.press_dispatched = false;
                        }
                    }
                } else if !st.frame_down && st.prev_down {
                    touch_release::on_release(st, ctx, dx, dy, now);
                } else if st.frame_down {
                    #[cfg(feature = "screenshot")]
                    if st.shot_armed {
                        let (press_dx, press_dy) = to_display(st.press_x, st.press_y);
                        if !st.shot_done
                            && gesture::is_screenshot_hold(
                                press_dx,
                                press_dy,
                                dx,
                                dy,
                                now.duration_since(st.press_time).as_millis(),
                                ctx.w as f32,
                                ctx.h as f32,
                            )
                        {
                            match crate::rendering::screenshot::capture(
                                rgb565_as_bytes_ref(&st.buffer),
                                ctx.w,
                                ctx.h,
                            ) {
                                Ok(path) => info!("screenshot: wrote {}", path.display()),
                                Err(e) => warn!("screenshot: {}", e),
                            }
                            st.shot_done = true;
                        }
                        st.prev_down = st.frame_down;
                        continue;
                    }
                    if st.scrubbing {
                        let frac = ((dx - pbar_x) / pbar_w).clamp(0.0, 1.0);
                        cb.progress_target.set((frac * 1000.0) as i32);
                    } else if st.sb_dragging && reader.get_chapter_overlay_open() {
                        let list_top = CH_LIST_TOP;
                        let list_bottom = ctx.h as i32 - CH_LIST_BOTTOM_PAD;
                        if st.search_results_active {
                            let hit_count = st
                                .word_index
                                .occurrences
                                .get(st.search_selected_word)
                                .map(|h| h.len().min(crate::data::word_index::MAX_SEARCH_RESULTS))
                                .unwrap_or(0);
                            let new_scroll = scrollbar_y_to_scroll(
                                dy as i32,
                                list_top,
                                list_bottom,
                                hit_count,
                                st.sb_grab_offset,
                            );
                            st.search_results_scroll = new_scroll;
                            st.press_search_results_scroll = new_scroll;
                        } else {
                            match st.chapter_tab {
                                crate::loop_state::ChapterTab::Words => {
                                    let item_count = st.word_index.words.len();
                                    let new_scroll = scrollbar_y_to_scroll(
                                        dy as i32,
                                        list_top,
                                        list_bottom,
                                        item_count,
                                        st.sb_grab_offset,
                                    );
                                    st.search_scroll = new_scroll;
                                    st.press_search_scroll = new_scroll;
                                }
                                crate::loop_state::ChapterTab::Marks => {
                                    let item_count = st.marks.len();
                                    let new_scroll = scrollbar_y_to_scroll(
                                        dy as i32,
                                        list_top,
                                        list_bottom,
                                        item_count,
                                        st.sb_grab_offset,
                                    );
                                    st.marks_scroll = new_scroll;
                                    st.press_marks_scroll = new_scroll;
                                }
                                crate::loop_state::ChapterTab::Chapters => {
                                    let item_count = st.toc_rows.len();
                                    let new_scroll = scrollbar_y_to_scroll(
                                        dy as i32,
                                        list_top,
                                        list_bottom,
                                        item_count,
                                        st.sb_grab_offset,
                                    );
                                    st.chapter_scroll = new_scroll;
                                    st.press_chapter_scroll = new_scroll;
                                }
                            }
                        }
                        st.text_dirty = true;
                        ctx.window.request_redraw();
                    } else if reader.get_chapter_overlay_open() {
                        let (press_dx, press_dy) = to_display(st.press_x, st.press_y);
                        let swipe_dy = dy - press_dy;
                        let swipe_dx = dx - press_dx;
                        if swipe_dy.abs() > 10.0 && swipe_dy.abs() > swipe_dx.abs() {
                            if st.search_results_active {
                                let hit_count = st
                                    .word_index
                                    .occurrences
                                    .get(st.search_selected_word)
                                    .map(|h| {
                                        h.len().min(crate::data::word_index::MAX_SEARCH_RESULTS)
                                    })
                                    .unwrap_or(0);
                                let max_scroll = search::results_scroll_max(hit_count);
                                let raw = (st.press_search_results_scroll - swipe_dy as i32)
                                    .clamp(0, max_scroll);
                                st.search_results_scroll = raw;
                            } else {
                                match st.chapter_tab {
                                    crate::loop_state::ChapterTab::Words => {
                                        let max_scroll =
                                            search::search_scroll_max(st.word_index.words.len());
                                        let raw = (st.press_search_scroll - swipe_dy as i32)
                                            .clamp(0, max_scroll);
                                        st.search_scroll = raw;
                                    }
                                    crate::loop_state::ChapterTab::Marks => {
                                        let list_h =
                                            ctx.h as i32 - CH_LIST_TOP - CH_LIST_BOTTOM_PAD;
                                        let max_scroll = list_scroll_max(st.marks.len(), list_h);
                                        let raw = (st.press_marks_scroll - swipe_dy as i32)
                                            .clamp(0, max_scroll);
                                        st.marks_scroll = raw;
                                    }
                                    crate::loop_state::ChapterTab::Chapters => {
                                        let item_count = st.chapters.len();
                                        let list_h =
                                            ctx.h as i32 - CH_LIST_TOP - CH_LIST_BOTTOM_PAD;
                                        let max_scroll = list_scroll_max(item_count, list_h);
                                        let raw = (st.press_chapter_scroll - swipe_dy as i32)
                                            .clamp(0, max_scroll);
                                        st.chapter_scroll = raw;
                                    }
                                }
                            }
                            st.text_dirty = true;
                            ctx.window.request_redraw();
                        }
                    } else {
                        let (press_dx, press_dy) = to_display(st.press_x, st.press_y);
                        if gesture::classify_body_long_press(
                            press_dx,
                            press_dy,
                            dx,
                            dy,
                            now.duration_since(st.press_time).as_millis(),
                        ) {
                            if reader.get_chapter_overlay_open()
                                && st.chapter_tab == crate::loop_state::ChapterTab::Marks
                            {
                                let list_bottom = ctx.h as i32 - CH_LIST_BOTTOM_PAD;
                                if let Some(idx) = crate::rendering::marks_list::marks_list_hit_test(
                                    dy as i32,
                                    st.marks_scroll,
                                    st.marks.len(),
                                ) {
                                    if (dy as i32) < list_bottom {
                                        st.armed_mark_idx = idx;
                                        st.text_dirty = true;
                                        ctx.window.request_redraw();
                                    }
                                }
                            } else if !reader.get_chapter_overlay_open()
                                && !st.panel_open
                                && !st.picker_active
                            {
                                let cur = reader.get_cur_start().max(0) as usize;
                                st.selection = Some(crate::loop_state::Selection {
                                    anchor: cur,
                                    head: cur,
                                });
                                reader.set_selection_active(true);
                                st.text_dirty = true;
                                ctx.window.request_redraw();
                            }
                        } else {
                            ctx.window.window().dispatch_event(
                                slint::platform::WindowEvent::PointerMoved {
                                    position: slint::LogicalPosition::new(dx, dy),
                                },
                            );
                        }
                    }
                }
                if !st.frame_down {
                    st.scrubbing = false;
                }
                st.prev_down = st.frame_down;
            }
            _ => {}
        }
    }
    had_event
}
