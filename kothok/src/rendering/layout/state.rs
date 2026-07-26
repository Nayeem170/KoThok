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
    let img_map: HashMap<&str, &[u8]> = chapter_images
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let full = &chapter.text;
    let segs = &chapter.segments;
    let mut body = String::new();
    let mut style_runs: Vec<kobo_core::html_text::StyleRun> = Vec::new();
    let mut all_rows: Vec<Row> = Vec::new();
    let mut row_heights: Vec<i32> = Vec::new();
    let mut decoded_images: HashMap<usize, crate::rendering::text_render::DecodedImage> =
        HashMap::new();
    let mut links: Vec<kobo_core::html_text::LinkRun> = Vec::new();
    for seg in segs {
        if seg.src.is_some() || seg.tag == "figure" {
            rows::push_figure_rows(
                &mut all_rows,
                &mut row_heights,
                &mut decoded_images,
                &mut body,
                seg,
                &img_map,
                body_px,
                line_h,
            );
            continue;
        }
        let seg_text = full.get(seg.start..seg.end).unwrap_or("");
        if rows::is_heading(&seg.tag) {
            // A heading contributes to `body` like any other block, so its
            // style and link runs rebase the same way. `trim()` inside
            // `push_heading_rows` only ever removes leading whitespace the
            // extractor already stripped, so the offsets line up.
            let seg_base = body.len() + usize::from(!body.is_empty());
            let shift = |off: usize| seg_base + off.saturating_sub(seg.start);
            for r in &seg.styles {
                style_runs.push(kobo_core::html_text::StyleRun {
                    start: shift(r.start),
                    end: shift(r.end),
                    bold: r.bold,
                    italic: r.italic,
                    link: r.link,
                });
            }
            for l in &seg.links {
                links.push(kobo_core::html_text::LinkRun {
                    start: shift(l.start),
                    end: shift(l.end),
                    href: l.href.clone(),
                });
            }
            rows::push_heading_rows(
                &mut all_rows,
                &mut row_heights,
                &mut body,
                seg_text,
                &seg.tag,
                body_px,
            );
        } else if seg.tag == "pre" {
            rows::push_pre_rows(
                &mut all_rows,
                &mut row_heights,
                &mut body,
                seg_text,
                body_px,
                line_h,
            );
        } else {
            let marker = seg.list_marker();
            let marker_len = marker.as_ref().map_or(0, |m| m.len());
            let seg_base = body.len() + usize::from(!body.is_empty());
            // Chapter-text offsets rebase onto `body`, which is what rows carry
            // and what the tap hit-test resolves against.
            let shift = |off: usize| seg_base + marker_len + off.saturating_sub(seg.start);
            for r in &seg.styles {
                style_runs.push(kobo_core::html_text::StyleRun {
                    start: shift(r.start),
                    end: shift(r.end),
                    bold: r.bold,
                    italic: r.italic,
                    link: r.link,
                });
            }
            for l in &seg.links {
                links.push(kobo_core::html_text::LinkRun {
                    start: shift(l.start),
                    end: shift(l.end),
                    href: l.href.clone(),
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
                seg,
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
    ChapterState {
        all_rows,
        row_heights,
        pages,
        utterances,
        decoded_images,
        style_runs,
        links,
    }
}
