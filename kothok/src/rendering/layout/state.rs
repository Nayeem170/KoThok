// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
mod offsets;
mod rows;
mod utterances;

#[allow(unused_imports)]
pub use offsets::{
    count_chapter_pages, estimate_chapter_offsets, spawn_offset_computation, OffsetComputation,
};

use std::collections::HashMap;

use kobo_core::Chapter;
use log::debug;

use crate::Row;

use super::paginate::paginate_with_heights_ext;
use super::{content_h, ChapterState};

pub fn build_state(
    chapter: &mut Chapter,
    body_px: f32,
    _head_px: f32,
    line_h: i32,
) -> ChapterState {
    let chapter_images = chapter.load_images().to_vec();
    let full = &chapter.text;
    let segs = &chapter.segments;
    let mut body = String::new();
    let mut style_runs: Vec<kobo_core::html_text::StyleRun> = Vec::new();
    let mut all_rows: Vec<Row> = Vec::new();
    let mut row_heights: Vec<i32> = Vec::new();
    let mut decoded_images: HashMap<usize, crate::rendering::text_render::DecodedImage> =
        HashMap::new();
    let mut img_idx = 0usize;
    for seg in segs {
        if seg.src.is_some() {
            rows::push_image_row(
                &mut all_rows,
                &mut row_heights,
                &mut decoded_images,
                seg,
                &chapter_images,
                &mut img_idx,
                line_h,
            );
            continue;
        }
        let seg_text = full.get(seg.start..seg.end).unwrap_or("");
        if rows::is_heading(&seg.tag) {
            rows::push_heading_rows(&mut all_rows, &mut row_heights, seg_text);
        } else if seg.tag == "pre" {
            rows::push_pre_rows(&mut all_rows, &mut row_heights, seg_text, body_px, line_h);
        } else {
            // Rows index into `body`, not into the chapter text, so emphasis
            // has to be rebased as the segment is appended. Doing it here keeps
            // the runs in the same coordinate space the rows use, which is what
            // lets the renderer look style up by offset.
            let seg_base = body.len() + usize::from(!body.is_empty());
            for r in &seg.styles {
                style_runs.push(kobo_core::html_text::StyleRun {
                    start: seg_base + r.start.saturating_sub(seg.start),
                    end: seg_base + r.end.saturating_sub(seg.start),
                    bold: r.bold,
                    italic: r.italic,
                });
            }
            rows::push_body_rows(
                &mut all_rows,
                &mut row_heights,
                &mut body,
                seg_text,
                &seg.tag,
                seg.indent,
                body_px,
                line_h,
            );
        }
    }
    let utterances = utterances::build_utterances(&body);
    let heading_indices: Vec<usize> = all_rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.kind == 2)
        .map(|(i, _)| i)
        .collect();
    let pages = paginate_with_heights_ext(&row_heights, content_h(), &heading_indices);
    debug!(
        "chapter: {} rows, {} pages, {} utterances, {} images",
        all_rows.len(),
        pages.len(),
        utterances.len(),
        decoded_images.len()
    );
    ChapterState {
        all_rows,
        row_heights,
        pages,
        utterances,
        decoded_images,
        style_runs,
    }
}
