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
fn hero_is_first_cell_full_width_at_top() {
    let cells = picker_scroll_cells(&make_books(7), 0);
    assert!(!cells.is_empty());
    let hero = &cells[0];
    assert_eq!(hero.idx, 0, "first cell is the hero (book 0)");
    assert_eq!(hero.w, picker_avail_w(), "hero spans full width");
    assert_eq!(hero.y, PICKER_PAD, "hero pinned to the top");
}

#[test]
fn grid_cells_three_columns_below_hero() {
    let cells = picker_scroll_cells(&make_books(7), 0);
    let avail_w = picker_avail_w();
    let cols = grid_cols();
    let cell_w = (avail_w - (cols as i32 - 1) * GRID_GAP) / cols as i32;
    assert_eq!(cells[1].x, PICKER_PAD);
    assert_eq!(cells[2].x, PICKER_PAD + cell_w + GRID_GAP);
    assert_eq!(cells[3].x, PICKER_PAD + 2 * (cell_w + GRID_GAP));
}

#[test]
fn grid_cells_adapt_to_screen_width() {
    assert_eq!(
        grid_cols_for_width(800 - 20),
        2,
        "Mini-class width should use 2 columns"
    );
    assert_eq!(
        grid_cols_for_width(1072 - 20),
        3,
        "Clara-class width should use 3 columns"
    );
    assert_eq!(
        grid_cols_for_width(1440 - 20),
        4,
        "Sage-class width should use 4 columns"
    );
    assert_eq!(
        grid_cols_for_width(1404 - 20),
        4,
        "Elipsa-class width should use 4 columns"
    );
}

#[test]
fn scroll_reveals_lower_rows() {
    let pitch = row_pitch();
    let cells_top = picker_scroll_cells(&make_books(12), 0);
    let cells_scrolled = picker_scroll_cells(&make_books(12), pitch);
    assert_eq!(cells_top[0].idx, 0, "top shows the hero first");
    assert!(
        cells_scrolled[0].idx > 0,
        "after one row of scroll the hero is gone"
    );
    let last_top = cells_top.last().unwrap().idx;
    let last_scrolled = cells_scrolled.last().unwrap().idx;
    assert!(last_scrolled > last_top, "scrolling exposes later books");
}

#[test]
fn empty_library_has_no_cells() {
    assert!(picker_scroll_cells(&[], 0).is_empty());
}

#[test]
fn cells_stay_on_screen_at_top_and_scrolled() {
    let w = crate::w() as i32;
    let bottom = crate::h() as i32 - NAV_BAR_H;
    let maxs = library_max_scroll(20);
    for scroll in [0, row_pitch(), maxs] {
        for cell in &picker_scroll_cells(&make_books(20), scroll) {
            let (x, y, cw, ch) = (cell.x, cell.y, cell.w, cell.h);
            assert!(x >= 0 && x + cw <= w, "x off-screen at scroll={}", scroll);
            assert!(y >= 0, "negative y at scroll={}", scroll);
            assert!(y + ch <= bottom, "cell into nav bar at scroll={}", scroll);
        }
    }
}

#[test]
fn max_scroll_is_row_snapped_and_nonneg() {
    let maxs = library_max_scroll(20);
    assert!(maxs >= 0);
    assert_eq!(maxs % row_pitch(), 0, "max scroll snaps to a row boundary");
    assert_eq!(library_max_scroll(2), 0, "fits on screen -> no scroll");
}

#[test]
fn snap_scroll_rounds_to_nearest_row() {
    let pitch = row_pitch();
    assert_eq!(snap_scroll(0), 0);
    assert_eq!(snap_scroll(pitch / 2 - 1), 0, "below half rounds down");
    assert_eq!(snap_scroll(pitch / 2), pitch, "half rounds up");
    assert_eq!(snap_scroll(pitch * 2 + pitch / 2 + 1), pitch * 3);
    assert_eq!(snap_scroll(-100), 0, "never scrolls above the top");
}

#[test]
fn picker_grid_constants_within_screen_bounds() {
    let w = crate::w() as i32;
    assert!(NAV_EXIT_X >= 0);
    assert!(
        NAV_EXIT_X + NAV_EXIT_W <= w,
        "Exit button right edge off-screen"
    );
}
