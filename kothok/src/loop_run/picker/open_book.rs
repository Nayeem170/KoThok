// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;

pub(super) fn open_book_from_picker(
    idx: usize,
    dx: f32,
    dy: f32,
    st: &mut LoopState,
    ctx: &mut LoopContext,
) {
    let reader = ctx.reader;
    let window = ctx.window;
    let fb = ctx.fb;
    let cmd_tx = ctx.cmd_tx;
    let w = ctx.w;
    let h = ctx.h;
    let cfg = &mut *ctx.cfg;
    let all_books = &mut *ctx.all_books;

    st.picker_last_tap_idx = None;
    debug!(
        "picker: opened book {} (double-tap) at ({:.0},{:.0})",
        idx, dx, dy
    );
    let book_path = all_books[idx].path.clone();
    best_effort_send(cmd_tx, Cmd::Stop);
    let t_open = std::time::Instant::now();
    let (loaded_chapters, book_lang) = open_book(&book_path)
        .filter(|(c, _)| !c.is_empty())
        .unwrap_or_else(|| (vec![Chapter::from_xhtml(0, None, SAMPLE_CHAPTER)], None));
    st.chapters = loaded_chapters;
    crate::rendering::render::set_rtl(is_rtl(book_lang.as_deref()));
    if let Some(msg) = crate::device::fonts::ensure_font_for_script(book_lang.as_deref(), "") {
        reader.set_status(msg.into());
        window.request_redraw();
        window.draw_if_needed(|r| {
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
    apply_book_voice(cfg, book_lang.as_deref(), reader, Some(&cmd_tx));
    st.chapter_count = st.chapters.len();
    reader.set_chapter_count(st.chapter_count as i32);
    reader.set_loading_visible(true);
    reader.set_loading_pct(0);
    reader.set_picker_active(false);
    window.request_redraw();
    window.draw_if_needed(|r| {
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
    let pos = load_position(std::path::Path::new(POSITIONS_FILE), &book_path)
        .filter(|p| p.chapter < st.chapter_count)
        .unwrap_or(persistence::ReadingPosition {
            chapter: 0,
            page: 0,
            cur_start: 0,
            cur_end: 0,
            view_mode: crate::ViewMode::Reading,
            bookmark: None,
            progress: 0.0,
        });
    st.current_chapter = pos.chapter;
    st.current_book_path = book_path.to_string();
    st.view_mode = pos.view_mode;
    st.bookmark = pos.bookmark.filter(|bm| bm.chapter < st.chapter_count);
    set_book_meta(
        reader,
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
        cfg,
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
    book_session::apply_session(reader, &session, st.current_chapter);
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
    let pick_cn =
        crate::data::library::chapter_display_title(&st.chapters[pos.chapter], pos.chapter);
    set_chapter_name(reader, &pick_cn);
    let audio = matches!(st.view_mode, crate::ViewMode::Audio);
    reader.set_audio_mode(audio);
    reader.set_has_bookmark(st.bookmark.is_some());
    if session.show_cover {
        st.cover_page_visible = true;
        let t_cov = std::time::Instant::now();
        render_book_cover_scaled(&st.current_book_path, &mut st.buffer);
        debug!("open-timing: cover {}ms", t_cov.elapsed().as_millis());
        fb.present(rgb565_as_bytes_ref(&st.buffer), w, h, true, 0, 0, WAVE_GC16);
        st.prev_buffer.copy_from_slice(&st.buffer);
        debug!("picker: opened book, showing cover");
    } else {
        window.request_redraw();
        window.draw_if_needed(|r| {
            r.render(&mut st.buffer, w);
        });
        if !audio {
            let pv = crate::rendering::text_overlay::PageView {
                w,
                h,
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
            overlay_text(&mut st.buffer, &pv);
        }
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
        st.prev_view_mode = st.view_mode;
        if audio {
            crate::audio::glue::load_chapter_audio(&st.state, &cmd_tx);
        } else {
            load_page_audio(st.current_page, &st.state, &cmd_tx);
        }
        reader.set_status("".into());
    }
}
