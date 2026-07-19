// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::collections::HashMap;

use log::{debug, warn};

use crate::rendering::text_render;
use crate::{Row, SharedString};

use super::super::{
    block_indent_for, content_h, first_line_indent, pack_block_indent, text_w, word_wrap_bytes,
    word_wrap_char_based, word_wrap_char_based_styled, word_wrap_indent, HEADING_GAP, HEADING_H,
    PARA_GAP, ROW_FLAG_INDENT, ROW_FLAG_JUSTIFY, ROW_FLAG_MONO,
};

pub(super) fn is_heading(tag: &str) -> bool {
    matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

pub(super) fn push_image_row(
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

pub(super) fn push_heading_rows(
    all_rows: &mut Vec<Row>,
    row_heights: &mut Vec<i32>,
    seg_text: &str,
) {
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

pub(super) fn push_body_rows(
    all_rows: &mut Vec<Row>,
    row_heights: &mut Vec<i32>,
    body: &mut String,
    seg_text: &str,
    tag: &str,
    indent_em: f32,
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
    // Justified, first-line-indented paragraphs for word-spacing scripts (the
    // Kobo/Kindle look). The first line wraps narrower to hold its indent; the
    // last line stays ragged (never justified). `overlay_text` reads the flags.
    let word_spacing = text_render::detect_script(seg_text).uses_word_spacing();
    // A stylesheet indent means the book placed this block deliberately - in
    // Calibre-converted technical books that is how a code listing's nesting
    // level is encoded. Such a block is set as code: the indent applies to
    // every line, and neither justification nor a first-line indent (both prose
    // devices, both of which shift columns around) is applied.
    let block_indent = block_indent_for(indent_em, body_px, text_w());
    let is_code_block = block_indent > 0;
    let avail = text_w().saturating_sub(block_indent);
    // Only true paragraphs take a first-line indent. A list item is already set
    // off as its own block, so indenting its first line reads as a stray
    // hanging line - technical books (lots of `li`) look broken with it.
    let indent_w = if word_spacing && tag != "li" && !is_code_block {
        first_line_indent(body_px)
    } else {
        0
    };
    // Code wraps by character, prose by word. Word wrapping normalises runs of
    // spaces to one, which is harmless in a sentence and destroys a code
    // listing: it is exactly the alignment inside a line - a continued
    // argument list, a comment column - that the indent work set out to keep.
    // Char wrapping also avoids reflowing an over-long line as though it were a
    // paragraph.
    let code_style = text_render::TextStyle {
        mono: true,
        ..Default::default()
    };
    let lines = if is_code_block {
        word_wrap_char_based_styled(seg_text, avail, body_px, code_style)
    } else if indent_w > 0 {
        word_wrap_indent(seg_text, avail, indent_w, body_px)
    } else {
        word_wrap_bytes(seg_text, avail, body_px)
    };
    let n = lines.len();
    let packed_indent = pack_block_indent(block_indent);
    for (i, l) in lines.iter().enumerate() {
        let is_last = i + 1 == n;
        let mut tag = packed_indent;
        if word_spacing && !is_last && !is_code_block {
            tag |= ROW_FLAG_JUSTIFY;
        }
        if i == 0 && indent_w > 0 {
            tag |= ROW_FLAG_INDENT;
        }
        if is_code_block {
            tag |= ROW_FLAG_MONO;
        }
        all_rows.push(Row {
            text: SharedString::from(l.text.clone()),
            start: (cs + l.start) as i32,
            end: (cs + l.end) as i32,
            kind: 0,
            tag,
        });
        row_heights.push(line_h);
    }
}

/// Verbatim code block. Emits one row per source line, preserving indentation;
/// lines wider than the text column char-wrap (keeping spaces) rather than
/// reflowing as prose. Not added to the TTS body and not highlight-mapped.
pub(super) fn push_pre_rows(
    all_rows: &mut Vec<Row>,
    row_heights: &mut Vec<i32>,
    seg_text: &str,
    body_px: f32,
    line_h: i32,
) {
    let gap_row = |all_rows: &mut Vec<Row>, row_heights: &mut Vec<i32>| {
        all_rows.push(Row {
            text: SharedString::from(""),
            start: 0,
            end: 0,
            kind: 3,
            tag: PARA_GAP,
        });
        row_heights.push(PARA_GAP);
    };
    let text_row = |all_rows: &mut Vec<Row>, row_heights: &mut Vec<i32>, s: &str| {
        all_rows.push(Row {
            text: SharedString::from(s),
            start: 0,
            end: 0,
            kind: 0,
            tag: 0,
        });
        row_heights.push(line_h);
    };

    if all_rows.last().is_some_and(|r| r.kind != 3) {
        gap_row(all_rows, row_heights);
    }
    for line in seg_text.split('\n') {
        if line.trim().is_empty() {
            text_row(all_rows, row_heights, "");
            continue;
        }
        let wrapped = word_wrap_char_based(line, text_w(), body_px);
        if wrapped.is_empty() {
            text_row(all_rows, row_heights, line);
        } else {
            for l in wrapped {
                text_row(all_rows, row_heights, &l.text);
            }
        }
    }
    gap_row(all_rows, row_heights);
}
