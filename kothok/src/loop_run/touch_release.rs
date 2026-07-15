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
    if st.scrubbing {
        st.scrubbing = false;
        debug!("pbar: scrub end");
    } else if st.pp_pressed {
        st.pp_pressed = false;
        if reader.get_play_enabled() {
            cb.play_toggle_cell.set(true);
            debug!("play-pause: footer tap");
        }
    } else if st.lib_pressed {
        st.lib_pressed = false;
        cb.quit.set(true);
        debug!("header: library tap");
    } else if st.menu_pressed {
        st.menu_pressed = false;
        st.panel_open = true;
        cb.panel_open_cell.set(true);
        reader.set_panel_open(true);
        st.text_dirty = true;
        debug!("header: menu tap (open panel)");
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
                debug!("panel: CLOSED (swipe-up while open)");
            }
            if reader.get_chapter_overlay_open() {
                match gesture::chapter_overlay_target(
                    dy,
                    swipe_dy,
                    swipe_dx,
                    st.chapter_scroll,
                    st.chapters.len(),
                ) {
                    gesture::ChapterOverlayAction::Scroll => {
                        let n = st.chapters.len() as i32;
                        let list_h = ctx.h as i32
                            - crate::rendering::render::CH_LIST_TOP
                            - crate::rendering::render::CH_LIST_BOTTOM_PAD;
                        let content_h = n * crate::rendering::render::CH_ROW_PITCH;
                        let max_scroll = (content_h - list_h).max(0);
                        st.chapter_scroll =
                            (st.chapter_scroll - swipe_dy as i32).clamp(0, max_scroll);
                        st.chapter_scroll = (st.chapter_scroll
                            / crate::rendering::render::CH_ROW_PITCH)
                            * crate::rendering::render::CH_ROW_PITCH;
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
            debug!(
                "gesture: book release dx={:.0} dy={:.0} swipe_down={} cover={} panel={} overlay={}",
                swipe_dx, swipe_dy, swipe_down, st.cover_page_visible,
                st.panel_open, reader.get_chapter_overlay_open()
            );
            if st.cover_page_visible && !swipe_down {
                st.cover_page_visible = false;
                st.text_dirty = true;
                ctx.window.request_redraw();
                let _ = ctx.window.draw_if_needed(|r| {
                    r.render(&mut st.buffer, ctx.w);
                });
                refresh_text_cache(
                    &mut st.text_cache,
                    ctx.w,
                    ctx.h,
                    &st.state.all_rows,
                    st.current_page,
                    &st.state.pages,
                    PAD_TOP,
                    &st.state.row_heights,
                    &st.state.decoded_images,
                    st.body_px,
                    st.head_px,
                    st.line_h,
                );
                composite_text(
                    &mut st.buffer,
                    &st.text_cache,
                    ctx.w,
                    ctx.h,
                    &st.state.all_rows,
                    st.current_page,
                    &st.state.pages,
                    PAD_TOP,
                    &st.state.row_heights,
                    st.line_h,
                    reader.get_cur_start(),
                    reader.get_cur_end(),
                );
                ctx.fb.present(
                    rgb565_as_bytes_ref(&st.buffer),
                    ctx.w,
                    ctx.h,
                    true,
                    0,
                    0,
                    WAVE_GC16,
                );
                st.prev_buffer.copy_from_slice(&st.buffer);
                load_page_audio(st.current_page, &st.state, &cmd_tx);
                debug!("cover: dismissed, showing page {}", st.current_page + 1);
                debug!("gesture: branch=cover_dismissed");
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
                if crate::device::bt_toggle_age_ms() >= BT_TOGGLE_GRACE_MS {
                    reader.set_bt_on(bt);
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
                    let _ = cmd_tx.send(Cmd::Pause);
                }
                debug!("panel: OPEN (swipe-down)");
                debug!("gesture: branch=st.panel_open");
            } else if st.panel_open
                && swipe_dy < -SWIPE_THRESHOLD_PX
                && swipe_dy.abs() > swipe_dx.abs()
            {
                st.panel_open = false;
                cb.panel_open_cell.set(false);
                reader.set_panel_open(false);
                debug!("panel: CLOSED (swipe-up)");
                st.text_dirty = true;
            } else if st.panel_open && press_dy > 500.0 {
                st.panel_open = false;
                cb.panel_open_cell.set(false);
                reader.set_panel_open(false);
                debug!("panel: CLOSED (content tap)");
                st.text_dirty = true;
            } else if st.panel_open {
                st.tap_xy.take();
            } else {
                let dt_ms = dt.as_millis();
                let swipe_dir =
                    gesture::classify_swipe(swipe_dx, swipe_dy, touch::SWIPE_MIN_DX, dt_ms);
                match swipe_dir {
                    gesture::SwipeDirection::Left => {
                        cb.page_delta.set(cb.page_delta.get() + 1);
                        debug!("swipe: LEFT dx={:.0} dy={:.0}", swipe_dx, swipe_dy);
                    }
                    gesture::SwipeDirection::Right => {
                        cb.page_delta.set(cb.page_delta.get() - 1);
                        debug!("swipe: RIGHT dx={:.0} dy={:.0}", swipe_dx, swipe_dy);
                    }
                    _ => {}
                }
                if swipe_dir == gesture::SwipeDirection::None {
                    let third = ctx.w as f32 / 3.0;
                    if dx >= third && dx < 2.0 * third {
                        let since_prev = now.duration_since(st.last_double_tap).as_millis();
                        let is_double_tap = touch::is_double_tap(dt_ms, since_prev);
                        st.last_double_tap = now;
                        if is_double_tap {
                            st.pending_tap_at = None;
                            let playing = reader.get_playing();
                            if playing {
                                reader.set_playing(false);
                                reader.set_paused(true);
                                let _ = cmd_tx.send(Cmd::Pause);
                            } else if reader.get_play_enabled() {
                                let cur = reader.get_cur_start().max(0) as usize;
                                let page_utts = page_utterances(st.current_page, &st.state);
                                let target =
                                    if page_utts.iter().any(|u| cur >= u.start && cur < u.end) {
                                        page_utts
                                            .iter()
                                            .position(|u| cur >= u.start && cur < u.end)
                                            .unwrap_or(0)
                                    } else {
                                        let (rs, re) = st
                                            .state
                                            .pages
                                            .get(st.current_page)
                                            .copied()
                                            .unwrap_or((0, 0));
                                        if let Some(rows) = st.state.all_rows.get(rs..re) {
                                            for row in rows {
                                                if row.start < row.end {
                                                    reader.set_cur_start(row.start);
                                                    reader.set_cur_end(row.end);
                                                    break;
                                                }
                                            }
                                        }
                                        0
                                    };
                                reader.set_saved_page(
                                    (st.chapter_offsets[st.current_chapter] + st.current_page)
                                        as i32,
                                );
                                st.reading_ch = st.current_chapter;
                                st.reading_pg = st.current_page;
                                let cs = reader.get_cur_start();
                                if cs > 0 {
                                    st.reading_off = cs as usize;
                                    st.reading_end = reader.get_cur_end() as usize;
                                }
                                let _ = cmd_tx.send(Cmd::Seek(target));
                                let _ = cmd_tx.send(Cmd::Play);
                                reader.set_playing(true);
                                reader.set_paused(false);
                            }
                            debug!("double-tap: toggle playback");
                        } else {
                            st.pending_tap_at = Some(now);
                        }
                    } else {
                        st.pending_tap_at = Some(now);
                    }
                }
            }
        } else if st.picker_active
            && swipe_dy.abs() > SWIPE_THRESHOLD_PX
            && swipe_dy.abs() > swipe_dx.abs()
        {
            let pitch = crate::rendering::render::row_pitch();
            let delta = if swipe_dy < 0.0 { pitch } else { -pitch };
            cb.picker_scroll_delta
                .set(cb.picker_scroll_delta.get() + delta);
            debug!("picker: scroll swipe dy={:.0} delta={}", swipe_dy, delta);
        }
    }
}
