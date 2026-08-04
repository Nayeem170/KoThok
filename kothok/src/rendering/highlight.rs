// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use crate::rendering::text_overlay::PageView;

use crate::data::mark::{Mark, MarkKind};

const HIGHLIGHT_BAND: u16 = 0xC638;
const SELECTION_BAND: u16 = 0x9999;

pub fn paint_highlight_bands(
    buf: &mut [Rgb565Pixel],
    pv: &PageView,
    marks: &[Mark],
    current_chapter: usize,
    pad_left: usize,
    pad_right: usize,
) {
    let buf_bytes = unsafe { &mut *(buf.as_mut_ptr() as *mut [u8]) };
    let w = pv.w;
    let h = pv.h;
    let (s, e) = pv.pages.get(pv.page).copied().unwrap_or((0, pv.rows.len()));
    let content_top = pv.content_top;
    let content_end = (content_top + crate::rendering::layout::content_h() as usize).min(h);
    let mut y = content_top;
    let row_count = pv.rows.get(s..e).unwrap_or(&[]).len();
    let chapter_marks: Vec<&Mark> = marks
        .iter()
        .filter(|m| m.kind == MarkKind::Highlight && m.chapter == current_chapter)
        .collect();
    if chapter_marks.is_empty() {
        return;
    }
    let content_w = w - pad_left - pad_right;
    for (ri, row) in pv.rows.get(s..e).unwrap_or(&[]).iter().enumerate() {
        let row_h = *pv.row_heights.get(s + ri).unwrap_or(&pv.line_h) as usize;
        let is_last = ri + 1 == row_count;
        let copy_h = if is_last {
            content_end.saturating_sub(y)
        } else {
            row_h
        };
        if row.start >= row.end || row_h == 0 {
            y += row_h;
            continue;
        }
        for m in &chapter_marks {
            let band_start = m.start.max(row.start as usize);
            let band_end = m.end.min(row.end as usize);
            if band_start >= band_end {
                continue;
            }
            for ry in 0..copy_h {
                let py = y + ry;
                if py >= h {
                    break;
                }
                let row_offset = py * w;
                for px in 0..content_w {
                    let idx = row_offset + pad_left + px;
                    if idx < buf.len() {
                        buf_bytes[idx * 2] = (HIGHLIGHT_BAND >> 8) as u8;
                        buf_bytes[idx * 2 + 1] = (HIGHLIGHT_BAND & 0xFF) as u8;
                    }
                }
            }
        }
        y += row_h;
    }
}

pub fn paint_selection_band(
    buf: &mut [Rgb565Pixel],
    pv: &PageView,
    anchor: usize,
    head: usize,
    pad_left: usize,
    pad_right: usize,
) {
    let buf_bytes = unsafe { &mut *(buf.as_mut_ptr() as *mut [u8]) };
    let w = pv.w;
    let h = pv.h;
    let (s, e) = pv.pages.get(pv.page).copied().unwrap_or((0, pv.rows.len()));
    let content_top = pv.content_top;
    let content_end = (content_top + crate::rendering::layout::content_h() as usize).min(h);
    let mut y = content_top;
    let row_count = pv.rows.get(s..e).unwrap_or(&[]).len();
    let sel_start = anchor.min(head);
    let sel_end = anchor.max(head);
    let content_w = w - pad_left - pad_right;
    for (ri, row) in pv.rows.get(s..e).unwrap_or(&[]).iter().enumerate() {
        let row_h = *pv.row_heights.get(s + ri).unwrap_or(&pv.line_h) as usize;
        let is_last = ri + 1 == row_count;
        let copy_h = if is_last {
            content_end.saturating_sub(y)
        } else {
            row_h
        };
        if row.start >= row.end || row_h == 0 {
            y += row_h;
            continue;
        }
        let band_start = sel_start.max(row.start as usize);
        let band_end = sel_end.min(row.end as usize);
        if band_start < band_end {
            for ry in 0..copy_h {
                let py = y + ry;
                if py >= h {
                    break;
                }
                let row_offset = py * w;
                for px in 0..content_w {
                    let idx = row_offset + pad_left + px;
                    if idx < buf.len() {
                        buf_bytes[idx * 2] = (SELECTION_BAND >> 8) as u8;
                        buf_bytes[idx * 2 + 1] = (SELECTION_BAND & 0xFF) as u8;
                    }
                }
            }
        }
        y += row_h;
    }
}
