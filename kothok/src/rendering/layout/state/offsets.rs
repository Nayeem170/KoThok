// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::sync::mpsc;
use std::thread;

use kobo_core::rendering::layout::ScreenLayout;
use kobo_core::Chapter;

use crate::data::persistence::{cache_path, save_offset_cache};

pub use kobo_core::rendering::layout::{count_chapter_pages, estimate_chapter_offsets};

pub struct OffsetComputation {
    pub pct_rx: mpsc::Receiver<i32>,
    pub result_rx: mpsc::Receiver<Vec<usize>>,
}

pub fn spawn_offset_computation(
    chapters: Vec<Chapter>,
    body_px: f32,
    line_h: i32,
    font_size: i32,
    book_path: String,
    layout: ScreenLayout,
) -> OffsetComputation {
    let (pct_tx, pct_rx) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    let total = chapters.len();
    thread::spawn(move || {
        let mut offsets = vec![0usize; chapters.len() + 1];
        let mut chapters = chapters;
        for (i, ch) in chapters.iter_mut().enumerate() {
            let pages = count_chapter_pages(ch, body_px, line_h, &layout);
            offsets[i + 1] = offsets[i] + pages;
            let pct = ((i + 1) * 100 / total) as i32;
            // best-effort: progress update; receiver may be dropped
            // best-effort: main loop may have dropped the progress receiver
            let _ = pct_tx.send(pct);
        }
        let cp = cache_path(&book_path, font_size);
        // best-effort: cache write; disk may be full
        save_offset_cache(&cp, &offsets);
        // best-effort: main loop may have closed the book before this finishes
        let _ = tx.send(offsets);
    });
    OffsetComputation {
        pct_rx,
        result_rx: rx,
    }
}
