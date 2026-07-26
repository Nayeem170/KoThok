// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::data::library::EpubEntry;

const FINISHED_AT: f32 = 0.99;
const STARTED_AT: f32 = 0.005;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LibraryFilter {
    #[default]
    All,
    Reading,
    Finished,
    New,
}

impl LibraryFilter {
    pub fn matches(self, book: &EpubEntry) -> bool {
        match self {
            LibraryFilter::All => true,
            LibraryFilter::Reading => book.progress >= STARTED_AT && book.progress < FINISHED_AT,
            LibraryFilter::Finished => book.progress >= FINISHED_AT,
            LibraryFilter::New => book.progress < STARTED_AT,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            LibraryFilter::All => "All",
            LibraryFilter::Reading => "Reading",
            LibraryFilter::Finished => "Finished",
            LibraryFilter::New => "New",
        }
    }
}

pub const FILTERS: [LibraryFilter; 4] = [
    LibraryFilter::All,
    LibraryFilter::Reading,
    LibraryFilter::Finished,
    LibraryFilter::New,
];

pub fn filtered_indices(books: &[EpubEntry], filter: LibraryFilter) -> Vec<usize> {
    books
        .iter()
        .enumerate()
        .skip(1)
        .filter(|(_, b)| filter.matches(b))
        .map(|(i, _)| i)
        .collect()
}
