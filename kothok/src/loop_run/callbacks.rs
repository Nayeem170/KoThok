use super::*;

pub(super) fn process_loop_callbacks(st: &mut LoopState, ctx: &mut LoopContext) -> (bool, bool) {
    let reader = ctx.reader;
    let cb = ctx.cb;
    let cmd_tx = ctx.cmd_tx;
    let mut ui_changed = false;
    let mut page_changed = false;
    let af = process_audio_events(st, ctx.evt_rx, reader, cmd_tx);
    ui_changed |= af.ui_changed;
    page_changed |= af.page_changed;
    st.text_dirty |= af.text_dirty;
    if reader.get_playing() && !st.picker_active {
        st.reading_ch = st.current_chapter;
        st.reading_pg = st.current_page;
        let cs = reader.get_cur_start();
        let ce = reader.get_cur_end();
        if cs > 0 {
            st.reading_off = cs as usize;
            st.reading_end = ce as usize;
        }
    }
    let pre_nav_ch = st.current_chapter;
    let pre_nav_pg = st.current_page;
    let (nav_text, nav_ui) =
        process_page_navigation(st, reader, cmd_tx, &cb.page_delta, &cb.progress_target);
    st.text_dirty |= nav_text;
    ui_changed |= nav_ui;
    if st.current_chapter != pre_nav_ch || st.current_page != pre_nav_pg {
        st.last_nav = std::time::Instant::now();
    }
    reader.set_nav_recent(st.last_nav.elapsed().as_secs() < 3);

    st.text_dirty |= process_panel_callbacks(st, reader, cmd_tx, ctx.cfg, ctx.fl_path, cb);

    if cb.play_toggle_cell.replace(false) && !st.picker_active && !st.current_book_path.is_empty() {
        let pt = toggle_playback(
            reader,
            cmd_tx,
            &st.state,
            st.current_page,
            &st.chapter_offsets,
            st.current_chapter,
        );
        st.reading_ch = pt.ch;
        st.reading_pg = pt.pg;
        st.reading_off = pt.off;
        st.reading_end = pt.end;
        debug!("play-pause: button toggled");
    }

    let overlay_now = reader.get_chapter_overlay_open();
    if overlay_now && !st.prev_chapter_overlay {
        st.chapter_scroll = 0;
    }
    st.prev_chapter_overlay = overlay_now;

    if cb.jump_to_reading_cell.replace(false) && !st.picker_active {
        st.panel_open = false;
        cb.panel_open_cell.set(false);
        reader.set_panel_open(false);
        if st.reading_ch < st.chapters.len() {
            if st.reading_ch != st.current_chapter {
                st.current_chapter = st.reading_ch;
                st.state = build_state(
                    &mut st.chapters[st.reading_ch],
                    st.body_px,
                    st.head_px,
                    st.line_h,
                );
                let cn = crate::data::library::chapter_display_title(
                    &st.chapters[st.reading_ch],
                    st.reading_ch,
                );
                set_chapter_name(reader, &cn);
                let _ = cmd_tx.send(Cmd::Reload(st.state.utterances.clone()));
            }
            if st.reading_off > 0 {
                st.current_page = st
                    .state
                    .pages
                    .iter()
                    .enumerate()
                    .find(|(_, (rs, re))| {
                        st.state.all_rows[*rs..*re].iter().any(|r| {
                            r.start as usize <= st.reading_off && r.end as usize > st.reading_off
                        })
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(st.reading_pg);
            } else {
                st.current_page = st.reading_pg;
            }
            st.current_page = st.current_page.min(st.state.pages.len().saturating_sub(1));
            apply_page(
                reader,
                &st.state,
                st.current_page,
                &st.chapter_offsets,
                st.current_chapter,
            );
            if st.reading_off > 0 {
                reader.set_cur_start(st.reading_off as i32);
                reader.set_cur_end(st.reading_end as i32);
            }
            reader
                .set_saved_page((st.chapter_offsets[st.current_chapter] + st.current_page) as i32);
            load_page_audio(st.current_page, &st.state, cmd_tx);
            st.text_dirty = true;
            ctx.window.request_redraw();
            debug!(
                "jump-to-reading: ch={} pg={} off={}",
                st.reading_ch + 1,
                st.current_page + 1,
                st.reading_off
            );
        }
    }

    (ui_changed, page_changed)
}
