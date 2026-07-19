// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
mod about;
mod open_book;

use super::*;
use crate::rendering::common::rgb565_as_bytes_ref;

pub(super) fn handle_picker(st: &mut LoopState, ctx: &mut LoopContext) -> LoopFlow {
    let reader = ctx.reader;
    let window = ctx.window;
    let fb = ctx.fb;
    let cb = ctx.cb;
    let caps = ctx.caps;
    let w = ctx.w;
    let h = ctx.h;
    let all_books = &mut *ctx.all_books;
    if st.picker_active {
        let sd = cb.picker_scroll_delta.replace(0);
        if sd != 0 && !st.about_open {
            let maxs = library_max_scroll(all_books, st.library_filter);
            let target = snap_scroll(st.picker_scroll + sd).min(maxs);
            if target != st.picker_scroll {
                st.picker_scroll = target;
                show_book_picker(
                    &reader,
                    &fb,
                    &window,
                    &mut st.buffer,
                    &mut st.text_cache,
                    &mut st.picker_cover_cache,
                    &all_books,
                    st.picker_scroll,
                    st.library_filter,
                    &caps.current_clock(),
                    caps.battery_pct(),
                    if st.exit_armed {
                        "Double-tap to Exit"
                    } else {
                        ""
                    },
                    PickerRefresh::Grid,
                );
                st.picker_cells =
                    picker_scroll_cells(&all_books, st.picker_scroll, st.library_filter);
                st.picker_entered = Some(std::time::Instant::now());
                st.prev_buffer.copy_from_slice(&st.buffer);
            }
        }
        let in_debounce = st.picker_entered.is_some_and(|t| {
            t.elapsed() <= std::time::Duration::from_millis(PICKER_ENTER_DEBOUNCE_MS)
        });
        if in_debounce {
            st.tap_xy = None;
        }
        if !in_debounce {
            if let Some((dx, dy)) = st.tap_xy.take() {
                if st.about_open {
                    about::handle_about_close(
                        dx, dy, w, st, reader, fb, window, &all_books, &*caps,
                    );
                    return LoopFlow::Continue;
                }
                debug!(
                    "picker: tap dx={:.0} dy={:.0}, cells={}",
                    dx,
                    dy,
                    st.picker_cells.len()
                );
                let nav_touch_top = h as f32 - NAV_BAR_H as f32 - PICKER_NAV_TOUCH_MARGIN as f32;
                let bezel_top = h as f32 - BEZEL_DEAD_ZONE as f32;
                match gesture::picker_hit_test(
                    dx,
                    dy,
                    &st.picker_cells,
                    &pill_rects(all_books),
                    w as f32,
                    nav_touch_top,
                    bezel_top,
                ) {
                    gesture::PickerTarget::Filter(filter) => {
                        st.tap_xy = None;
                        if filter == st.library_filter {
                            return LoopFlow::Continue;
                        }
                        debug!("picker: filter -> {:?}", filter);
                        st.library_filter = filter;
                        // A page offset from the old filter is meaningless
                        // against the new one's shorter list.
                        st.picker_scroll = 0;
                        st.exit_armed = false;
                        st.picker_last_tap_idx = None;
                        show_book_picker(
                            &reader,
                            &fb,
                            &window,
                            &mut st.buffer,
                            &mut st.text_cache,
                            &mut st.picker_cover_cache,
                            &all_books,
                            st.picker_scroll,
                            st.library_filter,
                            &caps.current_clock(),
                            caps.battery_pct(),
                            "",
                            PickerRefresh::BelowHeader,
                        );
                        st.picker_cells =
                            picker_scroll_cells(&all_books, st.picker_scroll, st.library_filter);
                        st.prev_buffer.copy_from_slice(&st.buffer);
                        return LoopFlow::Continue;
                    }
                    gesture::PickerTarget::Logo => {
                        st.tap_xy = None;
                        st.picker_last_tap_idx = None;
                        st.exit_armed = false;
                        st.about_open = true;
                        crate::rendering::about::show_about(&fb, &mut st.buffer, &st.device_model);
                        st.prev_buffer.copy_from_slice(&st.buffer);
                        return LoopFlow::Continue;
                    }
                    gesture::PickerTarget::Exit => {
                        let now = std::time::Instant::now();
                        let within = st.exit_armed
                            && now.duration_since(st.exit_armed_time).as_millis()
                                < EXIT_CONFIRM_WINDOW_MS as u128;
                        if within {
                            debug!("picker: Exit confirmed");
                            cb.exit_app.set(true);
                            show_book_picker(
                                &reader,
                                &fb,
                                &window,
                                &mut st.buffer,
                                &mut st.text_cache,
                                &mut st.picker_cover_cache,
                                &all_books,
                                st.picker_scroll,
                                st.library_filter,
                                &caps.current_clock(),
                                caps.battery_pct(),
                                "Exiting...",
                                PickerRefresh::Grid,
                            );
                            st.prev_buffer.copy_from_slice(&st.buffer);
                        } else {
                            st.exit_armed = true;
                            st.exit_armed_time = now;
                            debug!("picker: Exit armed (confirm to exit)");
                            show_book_picker(
                                &reader,
                                &fb,
                                &window,
                                &mut st.buffer,
                                &mut st.text_cache,
                                &mut st.picker_cover_cache,
                                &all_books,
                                st.picker_scroll,
                                st.library_filter,
                                &caps.current_clock(),
                                caps.battery_pct(),
                                "Double-tap to Exit",
                                PickerRefresh::Grid,
                            );
                            st.picker_cells = picker_scroll_cells(
                                &all_books,
                                st.picker_scroll,
                                st.library_filter,
                            );
                            st.prev_buffer.copy_from_slice(&st.buffer);
                        }
                        st.tap_xy = None;
                        return LoopFlow::Continue;
                    }
                    gesture::PickerTarget::Book(idx) => {
                        let now = std::time::Instant::now();
                        let is_double = gesture::picker_book_double_tap(
                            idx,
                            st.picker_last_tap_idx,
                            now,
                            st.picker_last_tap_time,
                            std::time::Duration::from_millis(PICKER_DOUBLE_TAP_MS),
                        );
                        st.picker_last_tap_idx = Some(idx);
                        st.picker_last_tap_time = now;
                        if !is_double {
                            debug!("picker: single-tap on book {} (tap again to open)", idx);
                            st.exit_armed = false;
                            show_book_picker(
                                &reader,
                                &fb,
                                &window,
                                &mut st.buffer,
                                &mut st.text_cache,
                                &mut st.picker_cover_cache,
                                &all_books,
                                st.picker_scroll,
                                st.library_filter,
                                &caps.current_clock(),
                                caps.battery_pct(),
                                "Double-tap to open",
                                PickerRefresh::Grid,
                            );
                            st.picker_cells = picker_scroll_cells(
                                &all_books,
                                st.picker_scroll,
                                st.library_filter,
                            );
                            st.prev_buffer.copy_from_slice(&st.buffer);
                            st.tap_xy = None;
                            return LoopFlow::Continue;
                        }
                        open_book::open_book_from_picker(idx, dx, dy, st, ctx);
                        return LoopFlow::Continue;
                    }
                    gesture::PickerTarget::None => {}
                }
            }
        }
    }
    LoopFlow::Normal
}
