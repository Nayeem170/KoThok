// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::rc::Rc;

use slint::platform::software_renderer::MinimalSoftwareWindow;
use slint::SharedString;

use crate::rendering::common::rgb565_as_bytes_ref;
use crate::rendering::fb::{Fb, WAVE_GC16};
use crate::Reader;

use super::*;

pub(super) fn open_book_from_picker(
    idx: usize,
    _dx: f32,
    _dy: f32,
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
    let book_path = all_books[idx].path.clone();
    best_effort_send(cmd_tx, Cmd::Stop);

    show_status(reader, window, fb, w, h, st, "Opening...");

    let t0 = std::time::Instant::now();
    let (loaded_chapters, book_lang, toc_tree, word_index) = open_book(&book_path)
        .filter(|(c, _, _, _)| !c.is_empty())
        .unwrap_or_else(|| {
            (
                vec![Chapter::from_xhtml(0, None, SAMPLE_CHAPTER)],
                None,
                Vec::new(),
                Default::default(),
            )
        });
    st.word_index = word_index;
    log::info!("perf: open_book {}ms", t0.elapsed().as_millis());
    st.chapters = loaded_chapters;
    st.toc_rows = crate::data::library::toc_rows(&toc_tree, &st.chapters);
    crate::rendering::render::set_rtl(is_rtl(book_lang.as_deref()));

    let font_sample: String = st
        .chapters
        .iter()
        .take(2)
        .flat_map(|c| c.text.chars())
        .take(8192)
        .collect();

    if book_lang
        .as_deref()
        .map(crate::device::fonts::script_for_lang)
        .unwrap_or(kobo_core::rendering::text_render::detect_script(
            &font_sample,
        ))
        .is_cjk()
    {
        show_status(reader, window, fb, w, h, st, "Loading font...");
    }

    let t1 = std::time::Instant::now();
    if let Some(msg) =
        crate::device::fonts::ensure_font_for_script(book_lang.as_deref(), &font_sample)
    {
        let status = if crate::device::wifi::wifi_status() {
            let script = book_lang
                .as_deref()
                .map(crate::device::fonts::script_for_lang)
                .unwrap_or(kobo_core::rendering::text_render::Script::Latin);
            let label = crate::device::fonts::font_label_for_script(script).unwrap_or("script");
            st.font_download_rx = Some(crate::device::font_download::spawn(script));
            format!("Downloading {} font...", label)
        } else {
            msg
        };
        show_status(reader, window, fb, w, h, st, &status);
    }
    log::info!("perf: ensure_font {}ms", t1.elapsed().as_millis());
    apply_book_voice(cfg, book_lang.as_deref(), reader, Some(cmd_tx));
    st.chapter_count = st.chapters.len();
    reader.set_chapter_count(st.chapter_count as i32);
    reader.set_toc_row_count(st.toc_rows.len().max(1) as i32);
    reader.set_loading_visible(true);
    reader.set_loading_pct(0);
    reader.set_picker_active(false);
    show_status(reader, window, fb, w, h, st, "");
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
    st.current_book_path = book_path.to_string();
    st.view_mode = pos.view_mode;
    st.bookmark = pos.bookmark.filter(|bm| bm.chapter < st.chapter_count);
    st.current_chapter = pos.chapter;
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

    if st.chapters[pos.chapter]
        .segments
        .iter()
        .any(|s| !s.styles.is_empty())
        || st.chapters[pos.chapter].text.len() > 2000
    {
        show_status(reader, window, fb, w, h, st, "Laying out...");
    }

    let t2 = std::time::Instant::now();
    let session = book_session::open_book_session(
        &mut st.chapters,
        &pos,
        cfg,
        st.body_px,
        st.head_px,
        st.line_h,
        &st.current_book_path,
    );
    log::info!("perf: build_state {}ms", t2.elapsed().as_millis());
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
    let pick_cn = crate::data::library::chapter_display_title(
        &st.chapters[st.current_chapter],
        st.current_chapter,
    );
    set_chapter_name(reader, &pick_cn);
    let audio = matches!(st.view_mode, ViewMode::Audio);
    reader.set_audio_mode(audio);
    reader.set_has_bookmark(st.bookmark.is_some());
    if session.show_cover {
        st.cover_page_visible = true;
        render_book_cover_scaled(&st.current_book_path, &mut st.buffer);
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
            crate::audio::glue::load_chapter_audio(&st.state, cmd_tx);
            let off = reader.get_cur_start().max(0) as usize;
            if off > 0 {
                let idx = crate::audio::glue::utterance_index_for_offset(&st.state.utterances, off);
                crate::audio::glue::best_effort_send(cmd_tx, Cmd::Seek(idx));
            }
        } else {
            load_page_audio(st.current_page, &st.state, cmd_tx);
        }
        reader.set_status("".into());
    }
}

fn show_status(
    reader: &Reader,
    window: &Rc<MinimalSoftwareWindow>,
    fb: &Fb,
    w: usize,
    h: usize,
    st: &mut LoopState,
    msg: &str,
) {
    reader.set_status(SharedString::from(msg));
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
