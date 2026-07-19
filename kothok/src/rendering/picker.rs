// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
mod filter;
mod header;
mod layout;
mod paging;
mod paint;

#[allow(unused_imports)]
pub use filter::{filtered_indices, LibraryFilter, FILTERS};
#[allow(unused_imports)]
pub use header::{header_exit_x, HEADER_BTN_PX};
#[allow(unused_imports)]
pub use layout::{
    picker_scroll_cells, GridCell, BEZEL_DEAD_ZONE, GRID_GAP, NAV_BAR_H, PICKER_HEADER_H,
    PICKER_NAV_TOUCH_MARGIN, PICKER_PAD,
};
#[allow(unused_imports)]
pub use paging::{
    books_per_page, library_max_scroll, library_pages, library_pages_for, page_pitch, snap_scroll,
    PickerRefresh,
};
pub use paint::{pill_rects, PillRect};

#[cfg(test)]
use layout::{card_layout, card_layout_at, grid_cols, grid_cols_at, GRID_ROWS, MAX_COLS, MIN_COLS};

use log::debug;
use slint::platform::software_renderer::{MinimalSoftwareWindow, Rgb565Pixel};
use slint::{ModelRc, VecModel};

use crate::data::library::EpubEntry;
use crate::rendering::common::{rgb565_as_bytes, rgb565_as_bytes_ref};
use crate::rendering::covers::CoverCache;
use crate::rendering::fb::{dump_ppm, Fb};
use crate::{Reader, Row};

pub fn show_book_picker(
    reader: &Reader,
    fb: &Fb,
    window: &MinimalSoftwareWindow,
    buffer: &mut [Rgb565Pixel],
    text_cache: &mut [Rgb565Pixel],
    cover_cache: &mut CoverCache,
    books: &[EpubEntry],
    scroll: i32,
    filter: LibraryFilter,
    clock: &str,
    battery: i32,
    prompt: &str,
    refresh: PickerRefresh,
) {
    let l = layout::card_layout();
    reader.set_rows(ModelRc::new(VecModel::from(Vec::<Row>::new())));
    reader.set_picker_active(true);
    window.request_redraw();
    // best-effort: Slint draw may be no-op if no redraw pending
    let _ = window.draw_if_needed(|r| {
        r.render(buffer, crate::w());
    });
    buffer.fill(Rgb565Pixel(0xFFFF));
    text_cache.fill(Rgb565Pixel(0xFFFF));
    let buf_bytes = rgb565_as_bytes(text_cache);

    header::paint_library_header(buf_bytes, crate::w(), crate::h());
    paint::paint_pills(buf_bytes, books, filter);

    let n_matching = filtered_indices(books, filter).len();
    if let Some(hero) = books.first() {
        paint::paint_hero_card(buf_bytes, cover_cache, hero, &l);
        for cell in picker_scroll_cells(books, scroll, filter) {
            if cell.idx == 0 {
                continue;
            }
            if let Some(book) = books.get(cell.idx) {
                paint::paint_book_card(buf_bytes, cover_cache, book, cell.x, cell.y, &l);
            }
        }
        if n_matching == 0 && filter != LibraryFilter::All {
            paint::paint_empty_filter(buf_bytes, &l, filter);
        }
    }

    let nav_y = crate::h().saturating_sub(NAV_BAR_H as usize);
    let pages = library_pages_for(n_matching);
    let pitch = page_pitch();
    let page = if pitch > 0 {
        (scroll / pitch).max(0) + 1
    } else {
        1
    };
    let scope = if filter == LibraryFilter::All {
        format!("{} books", books.len())
    } else {
        format!("{} {}", n_matching, filter.label().to_lowercase())
    };
    let hint = match (page, pages) {
        (_, p) if p <= 1 => "",
        (1, _) => " - swipe left for more",
        (c, p) if c >= p => " - swipe right to go back",
        _ => " - swipe left or right",
    };
    let center = if !prompt.is_empty() {
        prompt.to_string()
    } else if pages > 1 {
        format!("{} - {}/{}{}", scope, page, pages, hint)
    } else {
        scope
    };
    header::paint_picker_nav_bar(
        buf_bytes,
        crate::w(),
        crate::h(),
        nav_y,
        NAV_BAR_H as usize,
        &center,
        clock,
        battery,
    );

    buffer.copy_from_slice(text_cache);
    if cfg!(feature = "ppm-dump") {
        dump_ppm(
            crate::data::config::PPM_DEBUG,
            rgb565_as_bytes_ref(buffer),
            crate::w(),
            crate::h(),
        );
        // best-effort: debug copy for post-mortem
        let _ = std::fs::copy(
            crate::data::config::PPM_DEBUG,
            crate::data::config::PPM_DEPLOY,
        );
    }
    let (top, waveform) = refresh.band(&l);
    fb.present(
        rgb565_as_bytes_ref(buffer),
        crate::w(),
        crate::h(),
        refresh == PickerRefresh::Full,
        top,
        crate::h().saturating_sub(top),
        waveform,
    );
    debug!(
        "picker: page={} filter={:?} refresh={:?} ({} books)",
        scroll / pitch.max(1),
        filter,
        refresh,
        books.len()
    );
}

#[cfg(test)]
mod tests;
