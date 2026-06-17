use std::collections::HashMap;
use std::sync::mpsc;
use std::thread;

use kobo_core::Chapter;
use log::{debug, warn};

use crate::audio::Utterance;
use crate::data::persistence::{cache_path, save_offset_cache};
use crate::rendering::text_render;
use crate::{Row, SharedString};

use super::paginate::paginate_with_heights_ext;
use super::{
    content_h, sentences_with_ranges, text_w, word_wrap_bytes, ChapterState, HEADING_GAP,
    HEADING_H, PARA_GAP,
};

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
    let mut all_rows: Vec<Row> = Vec::new();
    let mut row_heights: Vec<i32> = Vec::new();
    let mut decoded_images: HashMap<usize, crate::rendering::text_render::DecodedImage> =
        HashMap::new();
    let mut img_idx = 0usize;
    for seg in segs {
        if seg.src.is_some() {
            push_image_row(
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
        if is_heading(&seg.tag) {
            push_heading_rows(&mut all_rows, &mut row_heights, seg_text);
        } else {
            push_body_rows(
                &mut all_rows,
                &mut row_heights,
                &mut body,
                seg_text,
                body_px,
                line_h,
            );
        }
    }
    let utterances = build_utterances(&body);
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
    }
}

fn is_heading(tag: &str) -> bool {
    matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

fn push_image_row(
    all_rows: &mut Vec<Row>,
    row_heights: &mut Vec<i32>,
    decoded_images: &mut HashMap<usize, crate::rendering::text_render::DecodedImage>,
    seg: &kobo_core::TextSegment,
    chapter_images: &[(String, Vec<u8>)],
    img_idx: &mut usize,
    line_h: i32,
) {
    if let Some(raw_bytes) = chapter_images.get(*img_idx).map(|(_, b)| b.as_slice()) {
        if let Some(img) = text_render::decode_image(raw_bytes, text_w(), content_h() as usize - 20)
        {
            let row_idx = all_rows.len();
            let cap = seg.caption.as_deref().unwrap_or("");
            let display_text = if cap.is_empty() {
                String::new()
            } else {
                cap.to_string()
            };
            let display_h = img.height as i32 + if cap.is_empty() { 4 } else { line_h + 4 };
            all_rows.push(Row {
                text: SharedString::from(display_text),
                start: 0,
                end: 0,
                kind: 1,
                tag: display_h,
            });
            row_heights.push(display_h);
            decoded_images.insert(row_idx, img);
            debug!(
                "image row {}: {}x{} -> display_h={}",
                row_idx, decoded_images[&row_idx].width, decoded_images[&row_idx].height, display_h
            );
        } else {
            warn!("image decode failed for segment {}", img_idx);
        }
    }
    *img_idx += 1;
}

fn push_heading_rows(all_rows: &mut Vec<Row>, row_heights: &mut Vec<i32>, seg_text: &str) {
    all_rows.push(Row {
        text: SharedString::from(seg_text.trim()),
        start: 0,
        end: 0,
        kind: 2,
        tag: 0,
    });
    row_heights.push(HEADING_H);
    all_rows.push(Row {
        text: SharedString::from(""),
        start: 0,
        end: 0,
        kind: 3,
        tag: HEADING_GAP,
    });
    row_heights.push(HEADING_GAP);
}

fn push_body_rows(
    all_rows: &mut Vec<Row>,
    row_heights: &mut Vec<i32>,
    body: &mut String,
    seg_text: &str,
    body_px: f32,
    line_h: i32,
) {
    if !all_rows.is_empty() {
        if let Some(last) = all_rows.last() {
            if last.kind != 3 {
                all_rows.push(Row {
                    text: SharedString::from(""),
                    start: 0,
                    end: 0,
                    kind: 3,
                    tag: PARA_GAP,
                });
                row_heights.push(PARA_GAP);
            }
        }
    }
    if !body.is_empty() {
        body.push('\n');
    }
    let cs = body.len();
    body.push_str(seg_text);
    for l in word_wrap_bytes(seg_text, text_w(), body_px) {
        all_rows.push(Row {
            text: SharedString::from(l.text.clone()),
            start: (cs + l.start) as i32,
            end: (cs + l.end) as i32,
            kind: 0,
            tag: 0,
        });
        row_heights.push(line_h);
    }
}

fn build_utterances(body: &str) -> Vec<Utterance> {
    let raw_utts = sentences_with_ranges(body);
    raw_utts
        .iter()
        .enumerate()
        .map(|(i, (text, start, end))| {
            let is_para_end = if i + 1 < raw_utts.len() {
                let next_start = raw_utts[i + 1].1;
                body.get(*end..next_start)
                    .is_none_or(|gap| gap.contains('\n'))
            } else {
                true
            };
            Utterance {
                text: text.clone(),
                start: *start,
                end: *end,
                para_end: is_para_end,
                page_break: None,
            }
        })
        .collect()
}

pub fn estimate_chapter_offsets(
    chapters: &[Chapter],
    current_ch: usize,
    current_pages: usize,
    line_h: i32,
) -> Vec<usize> {
    let chars_per_line = (text_w() as f32 / (line_h as f32 * 0.6)) as usize;
    let lines_per_page = (content_h() / line_h) as usize;
    let chars_per_page = chars_per_line * lines_per_page;
    let mut offsets = vec![0usize; chapters.len() + 1];
    for (i, ch) in chapters.iter().enumerate() {
        let est = if i == current_ch {
            current_pages
        } else {
            ((ch.text.chars().count() as f64 / chars_per_page as f64).ceil() as usize).max(1)
        };
        offsets[i + 1] = offsets[i] + est;
    }
    offsets
}

pub fn count_chapter_pages(chapter: &mut Chapter, body_px: f32, line_h: i32) -> usize {
    let chapter_images = chapter.load_images().to_vec();
    let full = &chapter.text;
    let segs = &chapter.segments;
    let mut row_heights: Vec<i32> = Vec::new();
    let mut heading_indices: Vec<usize> = Vec::new();
    let mut prev_was_gap = false;
    let mut img_idx = 0usize;
    let is_heading = |t: &str| matches!(t, "h1" | "h2" | "h3" | "h4" | "h5" | "h6");
    for seg in segs {
        if seg.src.is_some() {
            let cap = seg.caption.as_deref().unwrap_or("");
            let h = if let Some(raw) = chapter_images.get(img_idx).map(|(_, b)| b.as_slice()) {
                text_render::decode_image(raw, text_w(), content_h() as usize - 20)
                    .map(|img| img.height as i32 + if cap.is_empty() { 4 } else { line_h + 4 })
                    .unwrap_or(line_h + 4)
            } else {
                line_h + 4
            };
            row_heights.push(h);
            prev_was_gap = false;
            img_idx += 1;
            continue;
        }
        let seg_text = full.get(seg.start..seg.end).unwrap_or("");
        if is_heading(seg.tag.as_str()) {
            heading_indices.push(row_heights.len());
            row_heights.push(HEADING_H);
            row_heights.push(HEADING_GAP);
            prev_was_gap = true;
        } else {
            if !row_heights.is_empty() && !prev_was_gap {
                row_heights.push(PARA_GAP);
            }
            let lines = word_wrap_bytes(seg_text, text_w(), body_px);
            for _ in &lines {
                row_heights.push(line_h);
            }
            prev_was_gap = false;
        }
    }
    paginate_with_heights_ext(&row_heights, content_h(), &heading_indices).len()
}

pub struct OffsetComputation {
    pub pct_rx: mpsc::Receiver<i32>,
    pub result_rx: mpsc::Receiver<Vec<usize>>,
}

pub fn spawn_offset_computation(
    chapters: Vec<Chapter>,
    body_px: f32,
    _head_px: f32,
    line_h: i32,
    font_size: i32,
    book_path: String,
) -> OffsetComputation {
    let (pct_tx, pct_rx) = mpsc::channel();
    let (tx, rx) = mpsc::channel();
    let total = chapters.len();
    thread::spawn(move || {
        let mut offsets = vec![0usize; chapters.len() + 1];
        let mut chapters = chapters;
        for (i, ch) in chapters.iter_mut().enumerate() {
            let pages = count_chapter_pages(ch, body_px, line_h);
            offsets[i + 1] = offsets[i] + pages;
            let pct = ((i + 1) * 100 / total) as i32;
            // best-effort: progress update; receiver may be dropped
            let _ = pct_tx.send(pct);
        }
        let cp = cache_path(&book_path, font_size);
        // best-effort: cache write; disk may be full
        save_offset_cache(&cp, &offsets);
        let _ = tx.send(offsets);
    });
    OffsetComputation {
        pct_rx,
        result_rx: rx,
    }
}
