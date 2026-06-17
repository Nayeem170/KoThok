use super::*;
use crate::rendering::common::rgb565_as_bytes_ref;

pub(super) fn handle_picker(st: &mut LoopState, ctx: &mut LoopContext) -> LoopFlow {
    let reader = ctx.reader;
    let window = ctx.window;
    let fb = ctx.fb;
    let cb = ctx.cb;
    let cmd_tx = ctx.cmd_tx;
    let caps = ctx.caps;
    let w = ctx.w;
    let h = ctx.h;
    let cfg = &mut *ctx.cfg;
    let all_books = &mut *ctx.all_books;
    if st.picker_active {
        let sd = cb.picker_scroll_delta.replace(0);
        if sd != 0 {
            let maxs = library_max_scroll(all_books.len());
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
                    &caps.current_clock(),
                    caps.battery_pct(),
                    if st.exit_armed {
                        "Double-tap to Exit"
                    } else {
                        ""
                    },
                );
                st.picker_cells = picker_scroll_cells(&all_books, st.picker_scroll);
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
                debug!(
                    "picker: tap dx={:.0} dy={:.0}, cells={}",
                    dx,
                    dy,
                    st.picker_cells.len()
                );
                let nav_touch_top = h as f32 - NAV_BAR_H as f32 - PICKER_NAV_TOUCH_MARGIN as f32;
                let bezel_top = h as f32 - BEZEL_DEAD_ZONE as f32;
                match gesture::picker_hit_test(dx, dy, &st.picker_cells, nav_touch_top, bezel_top) {
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
                                &caps.current_clock(),
                                caps.battery_pct(),
                                "Exiting...",
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
                                &caps.current_clock(),
                                caps.battery_pct(),
                                "Double-tap to Exit",
                            );
                            st.picker_cells = picker_scroll_cells(&all_books, st.picker_scroll);
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
                                &caps.current_clock(),
                                caps.battery_pct(),
                                "Double-tap to open",
                            );
                            st.picker_cells = picker_scroll_cells(&all_books, st.picker_scroll);
                            st.prev_buffer.copy_from_slice(&st.buffer);
                            st.tap_xy = None;
                            return LoopFlow::Continue;
                        }
                        st.picker_last_tap_idx = None;
                        debug!(
                            "picker: opened book {} (double-tap) at ({:.0},{:.0})",
                            idx, dx, dy
                        );
                        let book_path = all_books[idx].path.clone();
                        let _ = cmd_tx.send(Cmd::Stop);
                        let t_open = std::time::Instant::now();
                        let (loaded_chapters, book_lang) = open_book(&book_path)
                            .filter(|(c, _)| !c.is_empty())
                            .unwrap_or_else(|| {
                                (vec![Chapter::from_xhtml(0, None, SAMPLE_CHAPTER)], None)
                            });
                        st.chapters = loaded_chapters;
                        crate::rendering::render::set_rtl(is_rtl(book_lang.as_deref()));
                        if let Some(msg) =
                            crate::device::fonts::ensure_font_for_script(book_lang.as_deref(), "")
                        {
                            reader.set_status(msg.into());
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
                        apply_book_voice(cfg, book_lang.as_deref(), &reader, Some(&cmd_tx));
                        st.chapter_count = st.chapters.len();
                        reader.set_chapter_count(st.chapter_count as i32);
                        reader.set_loading_visible(true);
                        reader.set_loading_pct(0);
                        reader.set_picker_active(false);
                        window.request_redraw();
                        let _ = window.draw_if_needed(|r| {
                            r.render(&mut st.buffer, w);
                        });
                        fb.present(rgb565_as_bytes_ref(&st.buffer), w, h, false, 0, h, WAVE_GC16);
                        st.prev_buffer.copy_from_slice(&st.buffer);
                        let pos = load_position(std::path::Path::new(POSITIONS_FILE), &book_path)
                            .filter(|p| p.chapter < st.chapter_count)
                            .unwrap_or(persistence::ReadingPosition {
                                chapter: 0,
                                page: 0,
                                cur_start: 0,
                                cur_end: 0,
                            });
                        st.current_chapter = pos.chapter;
                        st.current_book_path = book_path.to_string();
                        set_book_meta(
                            &reader,
                            &all_books[idx].title,
                            all_books[idx].author.as_deref().unwrap_or(""),
                        );
                        reader.set_book_cover_img(crate::rendering::render::cover_image(
                            all_books[idx].cover_bytes.as_deref(),
                            200,
                            300,
                        ));
                        debug!(
                            "open-timing: open_book+voice+position {}ms",
                            t_open.elapsed().as_millis()
                        );
                        let t_bs = std::time::Instant::now();
                        let session = book_session::open_book_session(
                            &mut st.chapters,
                            &pos,
                            &cfg,
                            st.body_px,
                            st.head_px,
                            st.line_h,
                            &st.current_book_path,
                        );
                        debug!("open-timing: build_state {}ms", t_bs.elapsed().as_millis());
                        st.text_dirty = true;
                        if session.offset_rx.is_none() {
                            reader.set_loading_visible(false);
                        }
                        book_session::apply_session(&reader, &session, st.current_chapter);
                        st.offset_rx = session.offset_rx;
                        st.state = session.state;
                        st.chapter_offsets = session.chapter_offsets;
                        st.current_page = session.current_page;
                        st.reading_ch = session.reading_ch;
                        st.reading_pg = session.reading_pg;
                        st.reading_off = session.reading_off;
                        st.reading_end = session.reading_end;
                        st.picker_active = false;
                        reader.set_picker_active(false);
                        reader.set_playing(false);
                        reader.set_paused(false);
                        let pick_cn = crate::data::library::chapter_display_title(
                            &st.chapters[pos.chapter],
                            pos.chapter,
                        );
                        set_chapter_name(&reader, &pick_cn);
                        if session.show_cover {
                            st.cover_page_visible = true;
                            let t_cov = std::time::Instant::now();
                            render_book_cover_scaled(&st.current_book_path, &mut st.buffer);
                            debug!("open-timing: cover {}ms", t_cov.elapsed().as_millis());
                            fb.present(
                                rgb565_as_bytes_ref(&st.buffer),
                                w,
                                h,
                                true,
                                0,
                                0,
                                WAVE_GC16,
                            );
                            st.prev_buffer.copy_from_slice(&st.buffer);
                            debug!("picker: opened book, showing cover");
                        } else {
                            window.request_redraw();
                            let _ = window.draw_if_needed(|r| {
                                r.render(&mut st.buffer, w);
                            });
                            overlay_text(
                                &mut st.buffer,
                                w,
                                h,
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
                            load_page_audio(st.current_page, &st.state, &cmd_tx);
                            reader.set_status("".into());
                        }
                        return LoopFlow::Continue;
                    }
                    gesture::PickerTarget::None => {}
                }
            }
        }
    }
    LoopFlow::Normal
}
