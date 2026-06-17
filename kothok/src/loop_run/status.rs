use super::*;

pub(super) fn sync_panel_close(st: &mut LoopState, ctx: &LoopContext, msg: &str) {
    if st.panel_open && !ctx.cb.panel_open_cell.get() {
        st.panel_open = false;
        ctx.reader.set_panel_open(false);
        debug!("{}", msg);
        st.text_dirty = true;
    }
}

pub(super) fn handle_exit_button(st: &mut LoopState, ctx: &LoopContext) -> LoopFlow {
    if ctx.cb.exit_app.get() {
        if !st.picker_active && !st.current_book_path.is_empty() {
            save_position(
                std::path::Path::new(POSITIONS_FILE),
                &st.current_book_path,
                &persistence::ReadingPosition {
                    chapter: st.current_chapter,
                    page: st.current_page,
                    cur_start: ctx.reader.get_cur_start() as usize,
                    cur_end: ctx.reader.get_cur_end() as usize,
                },
            );
        }
        let _ = ctx.cmd_tx.send(Cmd::Stop);
        info!("EXIT: leaving app to nickel");
        return LoopFlow::Break;
    }
    LoopFlow::Normal
}

pub(super) fn handle_quit_button(st: &mut LoopState, ctx: &mut LoopContext) -> LoopFlow {
    let reader = ctx.reader;
    let cb = ctx.cb;
    if cb.quit.get() {
        if st.panel_open {
            cb.panel_open_cell.set(false);
            reader.set_panel_open(false);
        }
        if st.picker_active {
            return LoopFlow::Break;
        }
        if !st.current_book_path.is_empty() {
            save_position(
                std::path::Path::new(POSITIONS_FILE),
                &st.current_book_path,
                &persistence::ReadingPosition {
                    chapter: st.current_chapter,
                    page: st.current_page,
                    cur_start: reader.get_cur_start() as usize,
                    cur_end: reader.get_cur_end() as usize,
                },
            );
        }
        let _ = ctx.cmd_tx.send(Cmd::Stop);
        reader.set_playing(false);
        reader.set_paused(false);
        reader.set_cur_start(0);
        reader.set_cur_end(0);
        st.cover_page_visible = false;
        st.tap_xy = None;
        cb.quit.set(false);
        st.text_dirty = true;
        st.picker_scroll = 0;
        if !st.current_book_path.is_empty() {
            if let Some(pos) = ctx
                .all_books
                .iter()
                .position(|b| b.path == st.current_book_path)
            {
                if pos != 0 {
                    ctx.all_books.swap(0, pos);
                }
            }
            if let Some(b) = ctx.all_books.first_mut() {
                if b.progress <= 0.005 {
                    b.progress = 0.01;
                }
            }
        }
        show_book_picker(
            reader,
            ctx.fb,
            ctx.window,
            &mut st.buffer,
            &mut st.text_cache,
            &mut st.picker_cover_cache,
            ctx.all_books,
            st.picker_scroll,
            &ctx.caps.current_clock(),
            ctx.caps.battery_pct(),
            if st.exit_armed {
                "Double-tap to Exit"
            } else {
                ""
            },
        );
        st.picker_active = true;
        st.panel_open = false;
        reader.set_panel_open(false);
        st.picker_entered = Some(std::time::Instant::now());
        st.picker_cells = picker_scroll_cells(ctx.all_books, st.picker_scroll);
        st.prev_buffer.copy_from_slice(&st.buffer);
        return LoopFlow::Continue;
    }
    LoopFlow::Normal
}

pub(super) fn refresh_status(st: &mut LoopState, ctx: &LoopContext) {
    if st.last_status_refresh.elapsed().as_millis() as u64 >= STATUS_REFRESH_MS {
        st.last_status_refresh = std::time::Instant::now();
        let wifi = ctx.caps.network_available();
        let bt = ctx.caps.audio_sink_available();
        if crate::device::wifi_toggle_age_ms() >= WIFI_TOGGLE_GRACE_MS {
            ctx.reader.set_wifi_on(wifi);
        }
        if ctx.reader.get_wifi_on() && !st.voice_fetch_attempted {
            st.voice_fetch_attempted = true;
            st.voice_rx = Some(crate::panel::spawn_voice_fetch());
            info!("fetching voice list from Edge");
        }
        if crate::device::bt_toggle_age_ms() >= BT_TOGGLE_GRACE_MS {
            ctx.reader.set_bt_on(bt);
        }
        if let Some(n) = ctx.caps.wifi_name() {
            ctx.reader.set_wifi_name(SharedString::from(n));
        }
        if let Some(n) = ctx.caps.bt_name() {
            ctx.reader.set_bt_name(SharedString::from(n));
        }
        ctx.reader.set_play_enabled(ctx.reader.get_wifi_on() && ctx.reader.get_bt_on());
    }
}

pub(super) fn poll_voice_rx(st: &mut LoopState) {
    if let Some(rx) = &st.voice_rx {
        if let Ok(voices) = rx.try_recv() {
            let count = voices.len();
            crate::panel::save_voice_cache(&voices);
            crate::panel::set_dynamic_voices(voices);
            st.voice_rx = None;
            info!("voice list updated: {count} voices from Edge");
        }
    }
}

pub(super) fn poll_offset_rx(st: &mut LoopState, ctx: &LoopContext) {
    if let Some(ref comp) = st.offset_rx {
        while let Ok(pct) = comp.pct_rx.try_recv() {
            ctx.reader.set_loading_pct(pct);
            ctx.window.request_redraw();
        }
        if let Ok(real_offsets) = comp.result_rx.try_recv() {
            st.chapter_offsets = real_offsets;
            st.offset_rx = None;
            ctx.reader.set_loading_visible(false);
            ctx.reader
                .set_page((st.chapter_offsets[st.current_chapter] + st.current_page) as i32);
            ctx.reader
                .set_page_count(*st.chapter_offsets.last().unwrap_or(&1) as i32);
            ctx.reader
                .set_saved_page((st.chapter_offsets[st.current_chapter] + st.current_page) as i32);
            debug!(
                "offsets: background computation done, total={}",
                st.chapter_offsets.last().unwrap_or(&0)
            );
            st.text_dirty = true;
        }
    }
}
