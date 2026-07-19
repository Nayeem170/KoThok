// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;
use crate::data::library::EpubEntry;

fn make_books(n: usize) -> Vec<EpubEntry> {
    (0..n)
        .map(|i| EpubEntry {
            title: format!("Book {}", i),
            author: None,
            path: format!("/b{}.epub", i),
            cover_bytes: None,
            progress: 0.0,
        })
        .collect()
}

#[test]
fn hero_is_first_cell_full_width_below_pills() {
    let cells = picker_scroll_cells(&make_books(7), 0, LibraryFilter::All);
    assert!(!cells.is_empty());
    let hero = &cells[0];
    assert_eq!(hero.idx, 0, "first cell is the hero (book 0)");
    let l = card_layout();
    assert_eq!(
        hero.w,
        crate::w() as i32 - 2 * l.pad,
        "hero spans the content width"
    );
    assert_eq!(
        hero.y,
        l.pills_y + l.pills_h,
        "hero sits below header + pills"
    );
}

#[test]
fn cards_fill_rows_left_to_right_under_the_hero() {
    let l = card_layout();
    let cells = picker_scroll_cells(&make_books(20), 0, LibraryFilter::All);
    let cards = &cells[1..];
    assert_eq!(cards.len(), books_per_page(), "a full page of cards");

    // Row-major: the first `cols` cards share the top row, then the next row.
    for (i, cell) in cards.iter().enumerate() {
        let r = (i as i32) / l.cols;
        let c = (i as i32) % l.cols;
        assert_eq!(cell.idx, 1 + i, "cards run in book order");
        assert_eq!(cell.x, l.pad + c * (l.cell_w + l.gap), "column {}", c);
        assert_eq!(cell.y, l.grid_y + r * (l.row_h + l.gap), "row {}", r);
    }
}

#[test]
fn hero_is_pinned_and_paging_swaps_the_whole_card_set() {
    let pitch = page_pitch();
    let per = books_per_page();
    let page0 = picker_scroll_cells(&make_books(20), 0, LibraryFilter::All);
    let page1 = picker_scroll_cells(&make_books(20), pitch, LibraryFilter::All);

    assert_eq!(page0[0].idx, 0);
    assert_eq!(page1[0].idx, 0, "hero stays put across pages");
    assert_eq!(
        page0[0].x, page1[0].x,
        "pinned hero does not move horizontally"
    );
    assert_eq!(
        page0[0].y, page1[0].y,
        "pinned hero does not move vertically"
    );

    // A page turn advances by exactly one page of books, and the two pages
    // share no book.
    assert_eq!(page1[1].idx, page0[1].idx + per);
    let on0: Vec<usize> = page0[1..].iter().map(|c| c.idx).collect();
    assert!(
        page1[1..].iter().all(|c| !on0.contains(&c.idx)),
        "no book appears on both pages"
    );
}

#[test]
fn cards_do_not_move_within_a_page() {
    // Cell rects are a function of the page, not the raw offset: the geometry
    // is identical on every page, only the book indices change.
    let a = picker_scroll_cells(&make_books(20), 0, LibraryFilter::All);
    let b = picker_scroll_cells(&make_books(20), page_pitch(), LibraryFilter::All);
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!((x.x, x.y, x.w, x.h), (y.x, y.y, y.w, y.h));
    }
}

#[test]
fn empty_library_has_no_cells() {
    assert!(picker_scroll_cells(&[], 0, LibraryFilter::All).is_empty());
}

#[test]
fn single_book_shows_hero_only() {
    let cells = picker_scroll_cells(&make_books(1), 0, LibraryFilter::All);
    assert_eq!(cells.len(), 1, "one book => hero, no cards");
    assert_eq!(cells[0].idx, 0);
}

#[test]
fn last_page_is_partial_not_padded() {
    let per = books_per_page();
    // One book past a full page: the last page holds exactly that one card.
    let n = 1 + per + 1;
    let books = make_books(n);
    let last = library_max_scroll(&books, LibraryFilter::All);
    let cells = picker_scroll_cells(&books, last, LibraryFilter::All);
    assert_eq!(cells.len(), 2, "hero + the single trailing card");
    assert_eq!(cells[1].idx, n - 1);
}

/// Books with a spread of progress values, so each filter matches a known set.
/// Index 0 is the hero and is deliberately excluded from every filter.
fn mixed_books() -> Vec<EpubEntry> {
    let progress = [0.5, 0.0, 0.3, 1.0, 0.0, 0.995, 0.75, 0.0];
    progress
        .iter()
        .enumerate()
        .map(|(i, &p)| EpubEntry {
            title: format!("Book {}", i),
            author: None,
            path: format!("/b{}.epub", i),
            cover_bytes: None,
            progress: p,
        })
        .collect()
}

#[test]
fn filters_partition_the_books_after_the_hero() {
    let books = mixed_books();
    // Hero (index 0, progress 0.5) is in none of them.
    assert_eq!(
        filtered_indices(&books, LibraryFilter::All),
        vec![1, 2, 3, 4, 5, 6, 7]
    );
    assert_eq!(filtered_indices(&books, LibraryFilter::Reading), vec![2, 6]);
    assert_eq!(
        filtered_indices(&books, LibraryFilter::Finished),
        vec![3, 5]
    );
    assert_eq!(filtered_indices(&books, LibraryFilter::New), vec![1, 4, 7]);

    // Reading / Finished / New are disjoint and together cover All.
    let mut union: Vec<usize> = [
        LibraryFilter::Reading,
        LibraryFilter::Finished,
        LibraryFilter::New,
    ]
    .iter()
    .flat_map(|f| filtered_indices(&books, *f))
    .collect();
    union.sort_unstable();
    assert_eq!(union, filtered_indices(&books, LibraryFilter::All));
}

#[test]
fn filtered_cells_keep_indices_into_the_full_list() {
    let books = mixed_books();
    let cells = picker_scroll_cells(&books, 0, LibraryFilter::Finished);
    assert_eq!(cells[0].idx, 0, "hero shows whatever the filter");
    let shown: Vec<usize> = cells[1..].iter().map(|c| c.idx).collect();
    assert_eq!(
        shown,
        vec![3, 5],
        "cells carry original indices, not filtered ones"
    );
    // Every index still addresses the book the caller expects.
    for idx in shown {
        assert!(books[idx].progress >= 0.99);
    }
}

#[test]
fn hero_survives_a_filter_that_matches_nothing() {
    let mut books = mixed_books();
    books.truncate(1); // hero only
    let cells = picker_scroll_cells(&books, 0, LibraryFilter::Finished);
    assert_eq!(cells.len(), 1, "hero remains as the resume target");
    assert_eq!(cells[0].idx, 0);
    assert_eq!(library_max_scroll(&books, LibraryFilter::Finished), 0);
    assert_eq!(
        library_pages(&books, LibraryFilter::Finished),
        1,
        "still a valid page 0"
    );
}

#[test]
fn pills_are_laid_out_left_to_right_inside_the_content_width() {
    let books = mixed_books();
    let pills = pill_rects(&books);
    assert!(!pills.is_empty());
    let l = card_layout();
    assert_eq!(pills[0].filter, LibraryFilter::All, "All comes first");
    assert_eq!(pills[0].x, l.pad);
    for pair in pills.windows(2) {
        assert!(pair[1].x > pair[0].x + pair[0].w, "pills do not overlap");
    }
    let last = pills.last().unwrap();
    assert!(
        last.x + last.w <= crate::w() as i32 - l.pad,
        "pills stay inside the content width"
    );
}

#[test]
fn pills_sit_above_the_grid_and_below_the_header() {
    let l = card_layout();
    for pill in pill_rects(&mixed_books()) {
        assert!(pill.y >= PICKER_HEADER_H, "pill overlaps the header");
        assert!(
            pill.y + pill.h <= l.grid_y,
            "pill {:?} overlaps the card grid",
            pill.filter
        );
    }
}

#[test]
fn cells_stay_within_the_screen() {
    let w = crate::w() as i32;
    let bottom = crate::h() as i32 - NAV_BAR_H;
    let maxs = library_max_scroll(&make_books(20), LibraryFilter::All);
    for scroll in [0, page_pitch(), maxs] {
        for cell in &picker_scroll_cells(&make_books(20), scroll, LibraryFilter::All) {
            assert!(cell.x >= 0, "negative x at scroll={}", scroll);
            assert!(
                cell.x + cell.w <= w,
                "cell past right edge at scroll={}",
                scroll
            );
            assert!(cell.y >= 0, "negative y at scroll={}", scroll);
            assert!(
                cell.y + cell.h <= bottom,
                "cell into nav bar at scroll={}",
                scroll
            );
        }
    }
}

#[test]
fn grid_columns_stay_within_the_clamp() {
    let cols = grid_cols();
    assert!(
        (MIN_COLS..=MAX_COLS).contains(&cols),
        "cols {} outside {}..={}",
        cols,
        MIN_COLS,
        MAX_COLS
    );
    assert_eq!(books_per_page(), (cols * GRID_ROWS) as usize);
}

/// Every panel in the density table, evaluated without touching the latched
/// globals. The layout is only honest about "works on all Kobo devices" if the
/// whole fleet is checked, not the 1072x1448 the test defaults happen to use.
const FLEET: [(&str, i32, i32); 7] = [
    ("Touch / Mini", 600, 800),
    ("Glo / Aura / Nia", 758, 1024),
    ("Glo HD / Clara", 1072, 1448),
    ("Aura H2O", 1080, 1430),
    ("Libra H2O / Libra 2", 1264, 1680),
    ("Elipsa", 1404, 1872),
    ("Forma / Sage", 1440, 1920),
];

#[test]
fn layout_holds_on_every_panel_in_the_fleet() {
    for (name, w, h) in FLEET {
        let ppi = crate::rendering::density::ppi_for(w as usize, h as usize);
        let l = card_layout_at(w, h, ppi);
        let cols = grid_cols_at(w, ppi);

        assert!(
            (MIN_COLS..=MAX_COLS).contains(&cols),
            "{}: cols {} outside the clamp",
            name,
            cols
        );

        // Columns fit the content width.
        let used = cols * l.cell_w + (cols - 1) * l.gap;
        assert!(
            used <= w - 2 * l.pad,
            "{}: {} columns need {}px, content width is {}px",
            name,
            cols,
            used,
            w - 2 * l.pad
        );

        // Hero + two rows + gaps fit between the pills and the nav bar.
        let bottom = l.grid_y + GRID_ROWS * l.row_h + (GRID_ROWS - 1) * l.gap;
        assert!(
            bottom <= h - NAV_BAR_H,
            "{}: grid ends at {}, nav bar starts at {}",
            name,
            bottom,
            h - NAV_BAR_H
        );

        // The cover has to leave room for the title line under it.
        assert!(
            l.cover_h > 0 && l.cover_h < l.row_h,
            "{}: cover {} does not fit row {}",
            name,
            l.cover_h,
            l.row_h
        );
        assert!(
            l.cover_w <= l.cell_w,
            "{}: cover {} wider than cell {}",
            name,
            l.cover_w,
            l.cell_w
        );
        assert!(
            l.hero_cover_h <= l.hero_h,
            "{}: hero cover {} taller than hero {}",
            name,
            l.hero_cover_h,
            l.hero_h
        );
    }
}

#[test]
fn cells_are_physically_thumb_sized_on_every_panel() {
    // A cell has to stay comfortably past a ~9mm touch target on the smallest
    // panel, or the 2-column fallback is not buying anything.
    for (name, w, h) in FLEET {
        let ppi = crate::rendering::density::ppi_for(w as usize, h as usize);
        let l = card_layout_at(w, h, ppi);
        let mm = (l.cell_w as f32 / ppi as f32) * 25.4;
        assert!(
            mm >= 20.0,
            "{}: cell is {:.1}mm wide ({}px at {}ppi)",
            name,
            mm,
            l.cell_w,
            ppi
        );
    }
}

#[test]
fn max_scroll_is_page_snapped_and_nonneg() {
    let per = books_per_page();
    let maxs = library_max_scroll(&make_books(20), LibraryFilter::All);
    assert!(maxs >= 0);
    assert_eq!(maxs % page_pitch(), 0, "max scroll snaps to a page");
    assert_eq!(
        library_max_scroll(&make_books(1 + per), LibraryFilter::All),
        0,
        "one full page of cards -> no scroll"
    );
    assert_eq!(
        library_max_scroll(&make_books(1), LibraryFilter::All),
        0,
        "hero only -> no scroll"
    );
    assert_eq!(
        library_max_scroll(&make_books(1 + per + 1), LibraryFilter::All),
        page_pitch(),
        "one book past a page -> exactly one page of scroll"
    );
}

#[test]
fn snap_scroll_rounds_to_nearest_page() {
    let pitch = page_pitch();
    assert_eq!(snap_scroll(0), 0);
    assert_eq!(snap_scroll(pitch / 2 - 1), 0, "below half rounds down");
    assert_eq!(snap_scroll(pitch / 2), pitch, "half rounds up");
    assert_eq!(snap_scroll(pitch * 2 + pitch / 2 + 1), pitch * 3);
    assert_eq!(snap_scroll(-100), 0, "never scrolls before the start");
}

#[test]
fn picker_header_exit_within_screen_bounds() {
    let w = crate::w();
    let exit_x = header_exit_x(w);
    assert!(
        exit_x + HEADER_BTN_PX <= w,
        "Exit button right edge off-screen"
    );
}
