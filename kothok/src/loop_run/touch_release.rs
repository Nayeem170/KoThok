// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;
use crate::rendering::common::rgb565_as_bytes_ref;

pub(super) fn on_release(
    st: &mut LoopState,
    ctx: &mut LoopContext,
    dx: f32,
    dy: f32,
    now: std::time::Instant,
) {
    let reader = ctx.reader;
    let cb = ctx.cb;
    let cmd_tx = ctx.cmd_tx;
    let caps = ctx.caps;

    // The capture itself already happened mid-hold; this only has to make sure
    // the release does not also register as a tap on the captured screen.
    #[cfg(feature = "screenshot")]
    if st.shot_armed {
        st.shot_armed = false;
        st.shot_done = false;
        return;
    }

    if st.scrubbing {
        st.scrubbing = false;
    } else if st.pp_pressed {
        st.pp_pressed = false;
        if reader.get_play_enabled() {
            let is_double = st
                .pp_pending_release
                .map(|t| now.duration_since(t).as_millis() < PLAY_BUTTON_DOUBLE_MS as u128)
                .unwrap_or(false);
            if is_double {
                st.pp_pending_release = None;
                cb.bookmark_set_cell.set(true);
            } else {
                st.pp_pending_release = Some(now);
            }
        }
    } else if st.lib_pressed {
        st.lib_pressed = false;
        cb.quit.set(true);
    } else if st.menu_pressed {
        st.menu_pressed = false;
        st.panel_open = true;
        crate::debug_log::log("panel: swipe-up opened panel");
        cb.panel_open_cell.set(true);
        reader.set_panel_open(true);
        st.text_dirty = true;
    } else if st.mode_toggle_pressed {
        st.mode_toggle_pressed = false;
        cb.mode_toggle_cell.set(true);
    } else if st.bookmark_set_pressed {
        st.bookmark_set_pressed = false;
        cb.bookmark_set_cell.set(true);
    } else if st.bookmark_jump_pressed {
        st.bookmark_jump_pressed = false;
        cb.bookmark_jump_cell.set(true);
    } else if st.sleep_pressed {
        st.sleep_pressed = false;
        ctx.power_pressed
            .store(true, std::sync::atomic::Ordering::SeqCst);
    } else if st.chapter_pressed {
        st.chapter_pressed = false;
        cb.chapter_panel_cell.set(true);
    } else {
        let dt = now.duration_since(st.press_time);
        let (press_dx, press_dy) = touch::to_display(st.press_x, st.press_y, ctx.touch_cfg);
        let swipe_dx = dx - press_dx;
        let swipe_dy = dy - press_dy;
        if st.press_dispatched {
            st.press_dispatched = false;
            ctx.window
                .window()
                .dispatch_event(slint::platform::WindowEvent::PointerReleased {
                    position: slint::LogicalPosition::new(dx, dy),
                    button: slint::platform::PointerEventButton::Left,
                });
            if st.panel_open && swipe_dy < -SWIPE_THRESHOLD_PX && swipe_dy.abs() > swipe_dx.abs() {
                st.panel_open = false;
                cb.panel_open_cell.set(false);
                reader.set_panel_open(false);
                st.text_dirty = true;
            }
            // Audio mode forwards every press to Slint, so this is the only
            // release path it takes -- the double-tap handler further down sits
            // behind `!press_dispatched` and is unreachable here. Re-raise the
            // gesture for the disk area, between the header and the transport.
            if matches!(st.view_mode, ViewMode::Audio)
                && !st.panel_open
                && !reader.get_chapter_overlay_open()
                && swipe_dx.abs() < touch::SWIPE_MIN_DX
                && swipe_dy.abs() < SWIPE_THRESHOLD_PX
                && dy > AUDIO_HEADER_H
                && dy < ctx.h as f32 - AUDIO_FOOTER_H
            {
                let dt_ms = dt.as_millis();
                let since_prev = now.duration_since(st.last_double_tap).as_millis();
                st.last_double_tap = now;
                if touch::is_double_tap(dt_ms, since_prev) {
                    // Reuse the play button's cell so both go through the same
                    // (audio-scope-aware) handler.
                    cb.play_toggle_cell.set(true);
                }
            }

            if matches!(st.view_mode, ViewMode::Audio)
                && !st.panel_open
                && swipe_dy > SWIPE_THRESHOLD_PX
                && swipe_dy.abs() > swipe_dx.abs()
            {
                st.panel_open = true;
                cb.panel_open_cell.set(true);
                reader.set_panel_open(true);
                reader.set_battery_pct(caps.battery_pct());
                reader.set_clock(SharedString::from(caps.current_clock()));
                if reader.get_playing() {
                    reader.set_playing(false);
                    reader.set_paused(true);
                    best_effort_send(cmd_tx, Cmd::Pause);
                }
            }
            // Left/right swipe for page navigation, same as reading mode.
            if matches!(st.view_mode, ViewMode::Audio)
                && !st.panel_open
                && !reader.get_chapter_overlay_open()
                && swipe_dx.abs() > touch::SWIPE_MIN_DX
                && swipe_dx.abs() > swipe_dy.abs()
            {
                let dir = gesture::classify_swipe(
                    swipe_dx,
                    swipe_dy,
                    touch::SWIPE_MIN_DX,
                    dt.as_millis(),
                );
                match dir {
                    gesture::SwipeDirection::Left => {
                        cb.page_delta.set(cb.page_delta.get() + 1);
                    }
                    gesture::SwipeDirection::Right => {
                        cb.page_delta.set(cb.page_delta.get() - 1);
                    }
                    _ => {}
                }
            }
            if reader.get_chapter_overlay_open() {
                // The tab bar, the word list and the search results all live in
                // the same overlay, so their release handler runs first: a tap
                // on [Chapters]/[Words]/[X] must not also fall through to the
                // chapter-row hit test below.
                if search::handle_search_release(st, ctx, dx, dy) {
                    return;
                }
                match gesture::chapter_overlay_target(
                    dy,
                    swipe_dy,
                    swipe_dx,
                    st.chapter_scroll,
                    st.toc_rows.len(),
                ) {
                    gesture::ChapterOverlayAction::Scroll => {
                        st.text_dirty = true;
                        ctx.window.request_redraw();
                    }
                    gesture::ChapterOverlayAction::Select(idx) => {
                        reader.set_chapter_preview_idx(idx as i32);
                        st.text_dirty = true;
                        ctx.window.request_redraw();
                    }
                    gesture::ChapterOverlayAction::None => {}
                }
            }
        } else if !st.picker_active {
            let swipe_down = swipe_dy > SWIPE_THRESHOLD_PX && swipe_dy.abs() > swipe_dx.abs();
            if st.cover_page_visible && !swipe_down {
                st.cover_page_visible = false;
                st.text_dirty = true;
                ctx.window.request_redraw();
                ctx.window.draw_if_needed(|r| {
                    r.render(&mut st.buffer, ctx.w);
                });
                let pv = crate::rendering::text_overlay::PageView {
                    w: ctx.w,
                    h: ctx.h,
                    rows: &st.state.all_rows,
                    page: st.current_page,
                    pages: &st.state.pages,
                    content_top: PAD_TOP,
                    row_heights: &st.state.row_heights,
                    decoded_images: &st.state.decoded_images,
                    body_px: st.body_px,
                    head_px: st.head_px,
                    line_h: st.line_h,
                    style_runs: &st.state.style_runs,
                };
                refresh_text_cache(&mut st.text_cache, &pv);
                composite_text(
                    &mut st.buffer,
                    &st.text_cache,
                    &pv,
                    reader.get_cur_start(),
                    reader.get_cur_end(),
                );
                ctx.fb.present(
                    rgb565_as_bytes_ref(&st.buffer),
                    ctx.w,
                    ctx.h,
                    false,
                    0,
                    ctx.h,
                    WAVE_GC16,
                );
                st.prev_buffer.copy_from_slice(&st.buffer);
                if matches!(st.view_mode, crate::ViewMode::Audio) {
                    crate::audio::glue::load_chapter_audio(&st.state, cmd_tx);
                    let off = reader.get_cur_start().max(0) as usize;
                    if off > 0 {
                        let idx = crate::audio::glue::utterance_index_for_offset(
                            &st.state.utterances,
                            off,
                        );
                        crate::audio::glue::best_effort_send(cmd_tx, Cmd::Seek(idx));
                    }
                } else {
                    load_page_audio(st.current_page, &st.state, cmd_tx);
                }
            } else if !st.picker_active
                && !st.panel_open
                && swipe_dy > SWIPE_THRESHOLD_PX
                && swipe_dy.abs() > swipe_dx.abs()
            {
                if st.cover_page_visible {
                    st.cover_page_visible = false;
                    st.text_dirty = true;
                }
                st.panel_open = true;
                cb.panel_open_cell.set(true);
                reader.set_panel_open(true);
                reader.set_battery_pct(caps.battery_pct());
                reader.set_clock(SharedString::from(caps.current_clock()));
                let wifi = caps.network_available();
                let bt = caps.audio_sink_available();
                reader.set_wifi_on(wifi);
                if !crate::device::bt_reconnect_busy() {
                    reader.set_bt_on(bt);
                    if bt {
                        st.bt_fail_count = 0;
                    }
                    if let Some(n) = caps.bt_name() {
                        reader.set_bt_connected_name(SharedString::from(n));
                    }
                }
                reader.set_play_enabled(wifi && bt);
                if let Some(n) = caps.wifi_name() {
                    reader.set_wifi_connected_name(SharedString::from(n));
                }
                if let Some(ref path) = ctx.fl_path {
                    if let Some(hw) = frontlight_get(path) {
                        reader.set_brightness_val(hw as i32);
                    }
                }
                if reader.get_playing() {
                    reader.set_playing(false);
                    reader.set_paused(true);
                    best_effort_send(cmd_tx, Cmd::Pause);
                }
            } else if st.panel_open
                && (swipe_dy < -SWIPE_THRESHOLD_PX && swipe_dy.abs() > swipe_dx.abs()
                    || press_dy > 500.0)
            {
                st.panel_open = false;
                cb.panel_open_cell.set(false);
                reader.set_panel_open(false);
                st.text_dirty = true;
            } else if st.panel_open {
                st.tap_xy.take();
            } else {
                let dt_ms = dt.as_millis();
                let swipe_dir =
                    gesture::classify_swipe(swipe_dx, swipe_dy, touch::SWIPE_MIN_DX, dt_ms);
                match swipe_dir {
                    gesture::SwipeDirection::Left | gesture::SwipeDirection::Right
                        if st.zoom_active =>
                    {
                        // A swipe while zoomed just exits the magnifier; it does
                        // not also turn the page in the same touch. Panning was
                        // never asked for, so this avoids inventing it.
                        st.zoom_active = false;
                        st.text_dirty = true;
                    }
                    gesture::SwipeDirection::Left => {
                        cb.page_delta.set(cb.page_delta.get() + 1);
                    }
                    gesture::SwipeDirection::Right => {
                        cb.page_delta.set(cb.page_delta.get() - 1);
                    }
                    _ => {}
                }
                if swipe_dir == gesture::SwipeDirection::None
                    && super::link_nav::try_follow_link(st, reader, cmd_tx, ctx, dx, dy)
                {
                    // Consumed: a link tap must not also arm the double-tap
                    // play gesture or toggle the header.
                    st.pending_tap_at = None;
                    st.tap_xy.take();
                } else if swipe_dir == gesture::SwipeDirection::None {
                    let since_prev = now.duration_since(st.last_double_tap).as_millis();
                    let is_double_tap = touch::is_double_tap(dt_ms, since_prev);
                    st.last_double_tap = now;
                    if is_double_tap {
                        // Repurposed from play/pause (now on physical buttons):
                        // a double-tap anywhere toggles a 2x magnifier centred
                        // on the tap. A second double-tap clears it.
                        st.pending_tap_at = None;
                        st.zoom_active = !st.zoom_active;
                        if st.zoom_active {
                            let content_end = (PAD_TOP + ctx.content_h as usize).min(ctx.h);
                            st.zoom_center =
                                zoom_center_for_tap(dx, dy, ctx.w, PAD_TOP, content_end);
                        }
                        st.text_dirty = true;
                    } else {
                        st.pending_tap_at = Some(now);
                    }
                }
            }
        } else if st.picker_active
            && swipe_dx.abs() > SWIPE_THRESHOLD_PX
            && swipe_dx.abs() > swipe_dy.abs()
        {
            // The library pins the hero and pages the card grid sideways: swipe
            // left (dx<0) advances a page. One page per swipe, so the six cells
            // swap as a unit and stay row-major.
            let step = crate::rendering::render::page_pitch();
            let delta = if swipe_dx < 0.0 { step } else { -step };
            cb.picker_scroll_delta
                .set(cb.picker_scroll_delta.get() + delta);
        }
    }
}

/// Clamp a raw tap position into the content region for the zoom crop centre.
/// Split out of `on_release` (which needs a live Slint `Reader`) so the
/// headless-testable clamping math lives here. `content_top..content_end` is a
/// half-open row range; a tap in the header or footer shifts to the nearest
/// content edge rather than reading outside the magnified region.
pub(super) fn zoom_center_for_tap(
    dx: f32,
    dy: f32,
    w: usize,
    content_top: usize,
    content_end: usize,
) -> (usize, usize) {
    let cx = dx.round().clamp(0.0, w as f32 - 1.0) as usize;
    let cy = dy
        .round()
        .clamp(content_top as f32, content_end.saturating_sub(1) as f32) as usize;
    (cx, cy)
}

#[cfg(test)]
mod tests {
    use super::zoom_center_for_tap;

    // Content region is rows 100..200 (half-open), width 400.
    const W: usize = 400;
    const TOP: usize = 100;
    const END: usize = 200;

    #[test]
    fn centre_tap_keeps_its_coordinates() {
        assert_eq!(zoom_center_for_tap(200.0, 150.0, W, TOP, END), (200, 150));
    }

    #[test]
    fn tap_left_of_screen_clamps_to_zero() {
        assert_eq!(zoom_center_for_tap(-50.0, 150.0, W, TOP, END), (0, 150));
    }

    #[test]
    fn tap_past_right_edge_clamps_to_last_column() {
        assert_eq!(
            zoom_center_for_tap(10_000.0, 150.0, W, TOP, END),
            (399, 150)
        );
    }

    #[test]
    fn tap_in_header_clamps_down_to_content_top() {
        assert_eq!(zoom_center_for_tap(200.0, 5.0, W, TOP, END), (200, 100));
    }

    #[test]
    fn tap_in_footer_clamps_up_to_last_content_row() {
        assert_eq!(zoom_center_for_tap(200.0, 999.0, W, TOP, END), (200, 199));
    }
}
