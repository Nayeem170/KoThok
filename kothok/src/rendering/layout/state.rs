// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
mod body;
mod offsets;
mod rows;
mod utterances;

#[allow(unused_imports)]
pub use offsets::{
    count_chapter_pages, estimate_chapter_offsets, spawn_offset_computation, OffsetComputation,
};

pub(crate) use body::build_chapter_body;

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
    justify: bool,
) -> ChapterState {
    if chapter.body.is_empty() && !chapter.segments.is_empty() {
        let built = build_chapter_body(chapter);
        chapter.body = built.body;
        chapter.seg_body_start = built.seg_body_start;
        chapter.body_styles = built.styles;
        chapter.body_links = built.links;
    }
    let chapter_images = chapter.load_images().to_vec();
    let img_map: HashMap<&str, &[u8]> = chapter_images
        .iter()
        .map(|(n, b)| (n.as_str(), b.as_slice()))
        .collect();
    let full = &chapter.text;
    let segs = &chapter.segments;
    let body = &chapter.body;
    let seg_starts = &chapter.seg_body_start;
    let mut all_rows: Vec<Row> = Vec::new();
    let mut row_heights: Vec<i32> = Vec::new();
    let mut decoded_images: HashMap<usize, crate::rendering::text_render::DecodedImage> =
        HashMap::new();
    for (si, seg) in segs.iter().enumerate() {
        let body_start = seg_starts.get(si).copied().unwrap_or(body.len());
        if seg.src.is_some() || seg.tag == "figure" {
            rows::push_figure_rows(
                &mut all_rows,
                &mut row_heights,
                &mut decoded_images,
                seg,
                &img_map,
                body_start,
                body_px,
                line_h,
            );
            continue;
        }
        let seg_text = full.get(seg.start..seg.end).unwrap_or("");
        if rows::is_heading(&seg.tag) {
            rows::push_heading_rows(
                &mut all_rows,
                &mut row_heights,
                seg_text,
                &seg.tag,
                body_start,
                body_px,
            );
        } else if seg.tag == "pre" {
            rows::push_pre_rows(
                &mut all_rows,
                &mut row_heights,
                seg_text,
                body_start,
                body_px,
                line_h,
            );
        } else {
            rows::push_body_rows(
                &mut all_rows,
                &mut row_heights,
                seg_text,
                &seg.tag,
                seg.indent,
                body_start,
                body_px,
                line_h,
                seg,
                justify,
            );
        }
    }
    let utterances = utterances::build_utterances(body);
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
        style_runs: chapter.body_styles.clone(),
        links: chapter.body_links.clone(),
    }
}
