// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::data::library::EpubEntry;
use crate::rendering::density;
pub const GRID_GAP: i32 = 14;
pub const PICKER_PAD: i32 = 10;
const GRID_TARGET_CELL_W: i32 = 300;
pub(super) const MIN_COLS: i32 = 2;
pub(super) const MAX_COLS: i32 = 3;
pub const PICKER_HEADER_H: i32 = 110;
pub const NAV_BAR_H: i32 = 96;
pub const PICKER_NAV_TOUCH_MARGIN: i32 = 100;
pub const BEZEL_DEAD_ZONE: i32 = 2;

pub(super) const GRID_ROWS: i32 = 2;
const HERO_NUM: i32 = 3;
const HERO_DEN: i32 = 4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GridCell {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
    pub idx: usize,
}

pub(super) struct CardLayout {
    pub(super) pad: i32,
    pub(super) gap: i32,
    pub(super) pills_y: i32,
    pub(super) pills_h: i32,
    pub(super) hero_x: i32,
    pub(super) hero_y: i32,
    pub(super) hero_w: i32,
    pub(super) hero_h: i32,
    pub(super) hero_cover_w: i32,
    pub(super) hero_cover_h: i32,
    pub(super) grid_y: i32,
    pub(super) cols: i32,
    pub(super) cell_w: i32,
    pub(super) row_h: i32,
    pub(super) cover_w: i32,
    pub(super) cover_h: i32,
}

pub(super) fn grid_cols_at(screen_w: i32, ppi: usize) -> i32 {
    let gap = density::dp_at(GRID_GAP, ppi).max(1);
    let step = density::dp_at(GRID_TARGET_CELL_W, ppi).max(1) + gap;
    let avail_w = screen_w - 2 * density::dp_at(PICKER_PAD, ppi).max(1);
    ((avail_w + gap) / step).clamp(MIN_COLS, MAX_COLS)
}

pub(super) fn grid_cols() -> i32 {
    grid_cols_at(crate::w() as i32, density::ppi())
}

pub(super) fn card_layout() -> CardLayout {
    card_layout_at(crate::w() as i32, crate::h() as i32, density::ppi())
}

pub(super) fn card_layout_at(screen_w: i32, screen_h: i32, ppi: usize) -> CardLayout {
    let dp = |n: i32| density::dp_at(n, ppi);
    let pad = dp(PICKER_PAD).max(1);
    let gap = dp(GRID_GAP).max(1);
    let avail_w = screen_w - 2 * pad;

    let pills_y = PICKER_HEADER_H;
    let pills_h = dp(50);

    let cols = grid_cols_at(screen_w, ppi);
    let cell_w = ((avail_w - (cols - 1) * gap) / cols).max(dp(80));

    let grid_top = pills_y + pills_h;
    let band = (screen_h - grid_top - NAV_BAR_H).max(0);
    let usable = (band - GRID_ROWS * gap).max(0);
    let row_h = (usable * HERO_DEN / (GRID_ROWS * HERO_DEN + HERO_NUM)).max(dp(120));
    let hero_h = (row_h * HERO_NUM / HERO_DEN).max(dp(90));

    let text_band = dp(56);
    let cover_h = (row_h - text_band - dp(16)).max(dp(48));
    let cover_w = (cover_h * 2 / 3).min(cell_w - dp(24)).max(dp(32));
    let cover_h = cover_w * 3 / 2;

    let hero_cover_h = (hero_h - dp(24)).max(dp(60));
    let hero_cover_w = (hero_cover_h * 2 / 3).max(dp(40));

    CardLayout {
        pad,
        gap,
        pills_y,
        pills_h,
        hero_x: pad,
        hero_y: grid_top,
        hero_w: avail_w,
        hero_h,
        hero_cover_w,
        hero_cover_h,
        grid_y: grid_top + hero_h + gap,
        cols,
        cell_w,
        row_h,
        cover_w,
        cover_h,
    }
}

pub fn picker_scroll_cells(
    books: &[EpubEntry],
    scroll: i32,
    filter: super::filter::LibraryFilter,
) -> Vec<GridCell> {
    let mut cells = Vec::new();
    if books.is_empty() {
        return cells;
    }
    let l = card_layout();
    cells.push(GridCell {
        x: l.hero_x,
        y: l.hero_y,
        w: l.hero_w,
        h: l.hero_h,
        idx: 0,
    });

    let matching = super::filter::filtered_indices(books, filter);
    let pitch = super::paging::page_pitch();
    let page = if pitch > 0 {
        (scroll / pitch).max(0)
    } else {
        0
    };
    let per = super::paging::books_per_page();
    let base = (page as usize) * per;
    for i in 0..per {
        let Some(&idx) = matching.get(base + i) else {
            break;
        };
        let r = (i as i32) / l.cols;
        let c = (i as i32) % l.cols;
        cells.push(GridCell {
            x: l.pad + c * (l.cell_w + l.gap),
            y: l.grid_y + r * (l.row_h + l.gap),
            w: l.cell_w,
            h: l.row_h,
            idx,
        });
    }
    cells
}
