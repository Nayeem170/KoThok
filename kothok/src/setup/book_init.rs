// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use kobo_core::{Capabilities, Chapter};
use slint::platform::software_renderer::{MinimalSoftwareWindow, Rgb565Pixel};

use crate::capabilities::KoboCapabilities;
use crate::data::config::AppConfig;
use crate::data::library::{self, EpubEntry};
use crate::data::persistence::{self, POSITIONS_FILE};
use crate::rendering::common::rgb565_as_bytes_ref;
use crate::rendering::fb::{Fb, WAVE_GC16};
use crate::rendering::layout::{ChapterState, OffsetComputation, PAD_TOP};
use crate::rendering::render::{self, render_book_cover_scaled, CoverCache, GridCell};
use crate::{apply_book_voice, is_rtl, set_book_meta, set_chapter_name, SAMPLE_CHAPTER};

use log::debug;

pub(super) struct ReaderSetup {
    pub reader: crate::Reader,
    pub cfg: AppConfig,
    pub body_px: f32,
    pub head_px: f32,
    pub line_h: i32,
    pub dummy_ch: Chapter,
    pub fl_path: Option<std::path::PathBuf>,
    pub caps: KoboCapabilities,
}

pub(super) struct ScreenCtx<'a> {
    pub fb: &'a Fb,
    pub window: &'a MinimalSoftwareWindow,
    pub w: usize,
    pub h: usize,
}

pub(super) type PickerInit = (
    bool,
    i32,
    Vec<GridCell>,
    CoverCache,
    Option<std::time::Instant>,
);

pub(super) fn init_picker(
    setup: &ReaderSetup,
    screen: &ScreenCtx,
    all_books: &[EpubEntry],
) -> Option<PickerInit> {
    let picker_scroll = 0;
    let mut buffer = vec![Rgb565Pixel(0); screen.w * screen.h];
    let mut text_cache = vec![Rgb565Pixel(0); screen.w * screen.h];
    let mut picker_cover_cache: CoverCache = std::collections::HashMap::new();
    render::show_book_picker(
        &setup.reader,
        screen.fb,
        screen.window,
        &mut buffer,
        &mut text_cache,
        &mut picker_cover_cache,
        all_books,
        picker_scroll,
        render::LibraryFilter::default(),
        &setup.caps.current_clock(),
        setup.caps.battery_pct(),
        "",
        render::PickerRefresh::Full,
    );
    let picker_cells =
        render::picker_scroll_cells(all_books, picker_scroll, render::LibraryFilter::default());
    Some((
        true,
        picker_scroll,
        picker_cells,
        picker_cover_cache,
        Some(std::time::Instant::now()),
    ))
}

pub(super) type BookInit = (
    Vec<Chapter>,
    usize,
    Vec<usize>,
    Option<OffsetComputation>,
    usize,
    ChapterState,
    usize,
    usize,
    usize,
    usize,
    usize,
    String,
    bool,
    bool,
    crate::ViewMode,
    Option<crate::Bookmark>,
    Option<std::time::Instant>,
);

pub(super) fn init_book(
    setup: &mut ReaderSetup,
    screen: &ScreenCtx,
    all_books: &[EpubEntry],
    initial_path: &Option<String>,
) -> (Option<BookInit>, Option<PickerInit>) {
    let reader = &setup.reader;
    let book_path = initial_path.clone().unwrap_or_default();
    let t_open = std::time::Instant::now();
    let (loaded_chapters, book_lang) = initial_path
        .as_ref()
        .and_then(|p| library::open_book(p))
        .unwrap_or_else(|| (vec![Chapter::from_xhtml(0, None, SAMPLE_CHAPTER)], None));
    debug!(
        "startup-timing: open_book {}ms",
        t_open.elapsed().as_millis()
    );

    let chapters = loaded_chapters;
    let current_book_path = book_path.clone();
    render::set_rtl(is_rtl(book_lang.as_deref()));
    apply_book_voice(&mut setup.cfg, book_lang.as_deref(), reader, None);
    if let Some(b) = all_books.iter().find(|b| b.path == book_path) {
        set_book_meta(reader, &b.title, b.author.as_deref().unwrap_or(""));
        reader.set_book_cover_img(render::cover_image(b.cover_bytes.as_deref(), 200, 300));
    }
    let chapter_count = chapters.len();
    reader.set_chapter_count(chapter_count as i32);
    reader.set_loading_visible(true);
    reader.set_loading_pct(0);
    reader.set_picker_active(false);
    screen.window.request_redraw();
    let mut buffer = vec![Rgb565Pixel(0); screen.w * screen.h];
    screen.window.draw_if_needed(|r| {
        r.render(&mut buffer, screen.w);
    });
    screen.fb.present(
        rgb565_as_bytes_ref(&buffer),
        screen.w,
        screen.h,
        true,
        0,
        screen.h,
        WAVE_GC16,
    );

    let pos = persistence::load_position(std::path::Path::new(POSITIONS_FILE), &book_path)
        .filter(|p| p.chapter < chapter_count)
        .unwrap_or(persistence::ReadingPosition {
            chapter: 0,
            page: 0,
            cur_start: 0,
            cur_end: 0,
            view_mode: crate::ViewMode::Reading,
            bookmark: None,
            progress: 0.0,
        });
    let current_chapter = pos.chapter;
    let initial_view_mode = pos.view_mode;
    let initial_bookmark = pos.bookmark;

    let t_bs = std::time::Instant::now();
    let mut chapters_mut = chapters.clone();
    let session = crate::book_session::open_book_session(
        &mut chapters_mut,
        &pos,
        &mut setup.cfg,
        setup.body_px,
        setup.head_px,
        setup.line_h,
        &book_path,
    );
    debug!(
        "startup-timing: build_state {}ms",
        t_bs.elapsed().as_millis()
    );
    crate::book_session::apply_session(reader, &session, current_chapter);

    let offset_rx = session.offset_rx;
    if offset_rx.is_none() {
        reader.set_loading_visible(false);
    }
    let state = session.state;
    let chapter_offsets = session.chapter_offsets;
    let current_page = session.current_page;
    let reading_ch = session.reading_ch;
    let reading_pg = session.reading_pg;
    let reading_off = session.reading_off;
    let reading_end = session.reading_end;
    let text_dirty = true;
    let cover_page_visible = session.show_cover;

    let init_cn = library::chapter_display_title(&chapters_mut[pos.chapter], pos.chapter);
    set_chapter_name(reader, &init_cn);

    if cover_page_visible {
        render_book_cover_scaled(&book_path, &mut buffer);
        screen.fb.present(
            rgb565_as_bytes_ref(&buffer),
            screen.w,
            screen.h,
            true,
            0,
            0,
            WAVE_GC16,
        );
    } else {
        screen.window.request_redraw();
        screen.window.draw_if_needed(|r| {
            r.render(&mut buffer, screen.w);
        });
        render::overlay_text(
            &mut buffer,
            &render::PageView {
                w: screen.w,
                h: screen.h,
                rows: &state.all_rows,
                page: current_page,
                pages: &state.pages,
                content_top: PAD_TOP,
                row_heights: &state.row_heights,
                decoded_images: &state.decoded_images,
                body_px: setup.body_px,
                head_px: setup.head_px,
                line_h: setup.line_h,
                style_runs: &state.style_runs,
            },
        );
        screen.fb.present(
            rgb565_as_bytes_ref(&buffer),
            screen.w,
            screen.h,
            false,
            0,
            screen.h,
            WAVE_GC16,
        );
    }

    (
        Some((
            chapters_mut,
            chapter_count,
            chapter_offsets,
            offset_rx,
            current_chapter,
            state,
            current_page,
            reading_ch,
            reading_pg,
            reading_off,
            reading_end,
            current_book_path,
            cover_page_visible,
            text_dirty,
            initial_view_mode,
            initial_bookmark,
            None,
        )),
        None,
    )
}
