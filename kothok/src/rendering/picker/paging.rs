// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::data::library::EpubEntry;
use crate::rendering::fb::WAVE_GC16;
use crate::rendering::fb::WAVE_GL16;

use super::filter::{filtered_indices, LibraryFilter};
use super::layout::{card_layout, grid_cols, CardLayout, GRID_ROWS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerRefresh {
    Full,
    BelowHeader,
    Grid,
}

impl PickerRefresh {
    pub(super) fn band(self, l: &CardLayout) -> (usize, u32) {
        match self {
            PickerRefresh::Full => (0, WAVE_GC16),
            PickerRefresh::BelowHeader => (l.pills_y.max(0) as usize, WAVE_GL16),
            PickerRefresh::Grid => (l.grid_y.max(0) as usize, WAVE_GL16),
        }
    }
}

pub fn books_per_page() -> usize {
    (grid_cols() * GRID_ROWS).max(1) as usize
}

pub fn page_pitch() -> i32 {
    let l = card_layout();
    l.cols * (l.cell_w + l.gap)
}

pub fn library_pages_for(n_matching: usize) -> i32 {
    (n_matching.div_ceil(books_per_page()).max(1)) as i32
}

pub fn library_pages(books: &[EpubEntry], filter: LibraryFilter) -> i32 {
    library_pages_for(filtered_indices(books, filter).len())
}

pub fn library_max_scroll(books: &[EpubEntry], filter: LibraryFilter) -> i32 {
    (library_pages(books, filter) - 1).max(0) * page_pitch()
}

pub fn snap_scroll(scroll: i32) -> i32 {
    let pitch = page_pitch();
    if pitch <= 0 {
        return 0;
    }
    let snapped = (scroll + (pitch + 1) / 2) / pitch * pitch;
    snapped.max(0)
}
