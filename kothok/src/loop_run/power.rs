use super::*;
use crate::rendering::common::rgb565_as_bytes_ref;

pub(super) fn check_font_repaginate(st: &mut LoopState, ctx: &mut LoopContext) {
    let reader = ctx.reader;
    let window = ctx.window;
    let fb = ctx.fb;
    let w = ctx.w;
    let h = ctx.h;
    if text_render::font_install_count() != st.last_font_count {
        st.last_font_count = text_render::font_install_count();
        if !st.picker_active && st.chapters.len() > st.current_chapter {
            log::debug!("font: newly installed — rebuilding st.state");
            st.state = build_state(
                &mut st.chapters[st.current_chapter],
                st.body_px,
                st.head_px,
                st.line_h,
            );
            let total = st.state.all_rows.len();
            st.current_page = st.current_page.min(total.saturating_sub(1));
            crate::reader::apply_page(
                reader,
                &st.state,
                st.current_page,
                &st.chapter_offsets,
                st.current_chapter,
            );
            window.request_redraw();
            let _ = window.draw_if_needed(|r| {
                r.render(&mut st.buffer, w);
            });
            fb.present(
                rgb565_as_bytes_ref(&st.buffer),
                w,
                h,
                false,
                0,
                h,
                WAVE_GC16,
            );
            st.prev_buffer.copy_from_slice(&st.buffer);
        }
    }
}

pub(super) fn handle_power_button(st: &mut LoopState, ctx: &mut LoopContext) -> LoopFlow {
    let reader = ctx.reader;
    let window = ctx.window;
    let fb = ctx.fb;
    let cb = ctx.cb;
    let all_books = &mut *ctx.all_books;
    let caps = ctx.caps;
    let power_pressed = ctx.power_pressed;
    let fl_path = ctx.fl_path;
    let w = ctx.w;
    let h = ctx.h;
    if power_pressed.swap(false, std::sync::atomic::Ordering::SeqCst) {
        match st.system_state {
            SystemState::Awake => {
                if st.panel_open {
                    st.panel_open = false;
                    cb.panel_open_cell.set(false);
                    reader.set_panel_open(false);
                }
                if reader.get_chapter_overlay_open() {
                    reader.set_chapter_overlay_open(false);
                    reader.set_chapter_preview_idx(-1);
                    reader.set_chapter_pending(-1);
                }
                st.saved_brightness = enter_sleep(st, ctx, st.picker_active, reader.get_bt_on());
                st.system_state = SystemState::Asleep {
                    from_picker: st.picker_active,
                };
                info!("SLEEP (swipe-up to wake)");
                st.last_activity = std::time::Instant::now();
                return LoopFlow::Continue;
            }
            SystemState::Asleep { from_picker } => {
                if st.panel_open {
                    st.panel_open = false;
                    cb.panel_open_cell.set(false);
                }
                reader.set_panel_open(false);
                reader.set_chapter_overlay_open(false);
                reader.set_chapter_preview_idx(-1);
                reader.set_chapter_pending(-1);
                reader.set_loading_visible(false);
                st.cover_page_visible = false;
                window.request_redraw();
                if from_picker {
                    show_book_picker(
                        reader,
                        fb,
                        window,
                        &mut st.buffer,
                        &mut st.text_cache,
                        &mut st.picker_cover_cache,
                        all_books,
                        st.picker_scroll,
                        &caps.current_clock(),
                        caps.battery_pct(),
                        "",
                    );
                    fb.present(rgb565_as_bytes_ref(&st.buffer), w, h, true, 0, h, WAVE_GC16);
                    std::thread::sleep(std::time::Duration::from_millis(SLEEP_PANEL_SETTLE_MS));
                    st.prev_buffer.copy_from_slice(&st.buffer);
                    st.picker_active = true;
                    reader.set_picker_active(true);
                    if let Some(ref path) = fl_path {
                        crate::device::power::restore_frontlight(path, st.saved_brightness);
                    }
                    if reader.get_wifi_on() {
                        crate::device::wifi_toggle(true);
                    }
                } else {
                    wake_from_sleep(st, ctx);
                }
                st.system_state = SystemState::Awake;
                st.last_activity = std::time::Instant::now();
                st.prev_down = false;
                if reader.get_bt_on() {
                    crate::device::bt_toggle(true);
                }
                return LoopFlow::Continue;
            }
        }
    }
    LoopFlow::Normal
}

pub(super) fn poll_asleep_wake(st: &mut LoopState, ctx: &mut LoopContext) -> LoopFlow {
    let touch_dev = &mut *ctx.touch_dev;
    let touch_fd = ctx.touch_fd;
    let touch_cfg = ctx.touch_cfg;
    let power_pressed = ctx.power_pressed;
    if matches!(st.system_state, SystemState::Asleep { .. }) {
        poll_touch_for_wake(touch_dev, touch_fd, power_pressed.clone(), touch_cfg);
        if power_pressed.load(std::sync::atomic::Ordering::SeqCst) {
            st.last_activity = std::time::Instant::now();
            return LoopFlow::Continue;
        }
    }
    LoopFlow::Normal
}

pub(super) fn auto_sleep(st: &mut LoopState, ctx: &mut LoopContext) -> LoopFlow {
    let reader = ctx.reader;
    let cb = ctx.cb;
    if !reader.get_playing() && st.last_activity.elapsed().as_secs() > AUTO_SLEEP_SECS {
        info!("AUTO-SLEEP after {}s inactivity", AUTO_SLEEP_SECS);
        if st.panel_open {
            st.panel_open = false;
            cb.panel_open_cell.set(false);
            reader.set_panel_open(false);
        }
        if reader.get_chapter_overlay_open() {
            reader.set_chapter_overlay_open(false);
            reader.set_chapter_preview_idx(-1);
            reader.set_chapter_pending(-1);
        }
        st.saved_brightness = enter_sleep(st, ctx, st.picker_active, reader.get_bt_on());
        st.system_state = SystemState::Asleep {
            from_picker: st.picker_active,
        };
        info!("AUTO-SLEEP (swipe-up to wake)");
        st.last_activity = std::time::Instant::now();
        return LoopFlow::Continue;
    }
    LoopFlow::Normal
}
