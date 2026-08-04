// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::collections::HashMap;

use log::warn;

use crate::rendering::text_render;
use crate::{Row, SharedString};

use super::super::{
    block_indent_for, content_h, first_line_indent, pack_block_indent, text_w, word_wrap_bytes,
    word_wrap_char_based_styled, word_wrap_indent, HEADING_GAP, HEADING_H, MAX_BLOCK_INDENT_PX,
    PARA_GAP, ROW_FLAG_BQ, ROW_FLAG_CENTER, ROW_FLAG_INDENT, ROW_FLAG_JUSTIFY, ROW_FLAG_MONO,
};

pub(super) fn is_heading(tag: &str) -> bool {
    matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

pub(super) fn push_figure_rows(
    all_rows: &mut Vec<Row>,
    row_heights: &mut Vec<i32>,
    decoded_images: &mut HashMap<usize, crate::rendering::text_render::DecodedImage>,
    seg: &kobo_core::TextSegment,
    chapter_images: &HashMap<&str, &[u8]>,
    body_start: usize,
    body_px: f32,
    line_h: i32,
) {
    if all_rows.last().is_some_and(|r| r.kind != 3) {
        all_rows.push(Row {
            text: SharedString::from(""),
            start: 0,
            end: 0,
            kind: 3,
            tag: PARA_GAP,
        });
        row_heights.push(PARA_GAP);
    }
    let raw_bytes = seg
        .src
        .as_deref()
        .and_then(|src| chapter_images.get(src).copied());
    let decoded = raw_bytes.and_then(|b| {
        text_render::decode_image(b, text_w(), (content_h() as usize).saturating_sub(20))
    });
    if decoded.is_none() {
        if let Some(src) = seg.src.as_deref() {
            warn!("image missing or undecodable: {src}");
        }
    }
    if let Some(img) = decoded {
        let has_gap_before = all_rows.last().map(|r| r.kind == 3).unwrap_or(true);
        if !has_gap_before {
            all_rows.push(Row {
                text: SharedString::from(""),
                start: 0,
                end: 0,
                kind: 3,
                tag: PARA_GAP,
            });
            row_heights.push(PARA_GAP);
        }
        let row_idx = all_rows.len();
        let display_h = img.height as i32 + 4;
        all_rows.push(Row {
            text: SharedString::from(""),
            start: 0,
            end: 0,
            kind: 1,
            tag: display_h,
        });
        row_heights.push(display_h);
        decoded_images.insert(row_idx, img);
    }
    let Some(cap) = seg.caption.as_deref().filter(|c| !c.is_empty()) else {
        return;
    };
    let cs = body_start;
    for l in word_wrap_bytes(cap, text_w(), body_px) {
        all_rows.push(Row {
            text: SharedString::from(l.text.clone()),
            start: (cs + l.start) as i32,
            end: (cs + l.end) as i32,
            kind: 0,
            tag: ROW_FLAG_CENTER,
        });
        row_heights.push(line_h);
    }
    all_rows.push(Row {
        text: SharedString::from(""),
        start: 0,
        end: 0,
        kind: 3,
        tag: PARA_GAP,
    });
    row_heights.push(PARA_GAP);
}

pub(super) fn push_heading_rows(
    all_rows: &mut Vec<Row>,
    row_heights: &mut Vec<i32>,
    seg_text: &str,
    tag: &str,
    body_start: usize,
    body_px: f32,
) {
    let level: u32 = tag
        .strip_prefix('h')
        .and_then(|n| n.parse::<u32>().ok())
        .unwrap_or(1);
    let heading_px = body_px * super::super::heading_scale(level);
    let heading_line_h = text_render::line_height(heading_px) as i32;
    let heading_h = heading_line_h.max(HEADING_H);
    let trimmed = seg_text.trim();
    let lines = word_wrap_bytes(trimmed, text_w(), heading_px);
    for l in &lines {
        all_rows.push(Row {
            text: SharedString::from(l.text.clone()),
            start: (body_start + l.start) as i32,
            end: (body_start + l.end) as i32,
            kind: 2,
            tag: level as i32,
        });
        row_heights.push(heading_h);
    }
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
    seg_text: &str,
    tag: &str,
    indent_em: f32,
    body_start: usize,
    body_px: f32,
    line_h: i32,
    seg: &kobo_core::TextSegment,
    justify: bool,
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
    let marker = seg.list_marker();
    let marker_str = marker.as_deref().unwrap_or("");
    let marker_len = marker_str.len();
    let full_text = format!("{marker_str}{seg_text}");
    let word_spacing = text_render::detect_script(seg_text).uses_word_spacing();
    let block_indent = block_indent_for(indent_em, body_px, text_w());
    let bq_indent = if matches!(
        seg.blockquote,
        kobo_core::html_text::BlockquoteKind::Leaf | kobo_core::html_text::BlockquoteKind::Children
    ) {
        (body_px * 1.0) as usize
    } else {
        0
    };
    let is_list = seg.list.is_some() || tag == "li";
    let is_code_block = seg.code_indent;
    let marker_w = if marker_str.is_empty() {
        0
    } else {
        text_render::word_width(marker_str, body_px) as usize
    };
    let base_indent = (block_indent + bq_indent).min(MAX_BLOCK_INDENT_PX);
    let hanging_indent = (base_indent + marker_w).min(MAX_BLOCK_INDENT_PX);
    let avail = text_w().saturating_sub(hanging_indent);
    let indent_w = if word_spacing && !is_list && !is_code_block {
        first_line_indent(body_px)
    } else {
        0
    };
    let code_style = text_render::TextStyle {
        mono: true,
        ..Default::default()
    };
    let wrap_src = if marker_w > 0 { seg_text } else { &full_text };
    let lines = if is_code_block {
        let code_px = body_px * super::super::MONO_SCALE;
        word_wrap_char_based_styled(wrap_src, avail, code_px, code_style)
    } else if indent_w > 0 {
        word_wrap_indent(wrap_src, avail, indent_w, body_px)
    } else {
        word_wrap_bytes(wrap_src, avail, body_px)
    };
    let n = lines.len();
    let packed_indent = pack_block_indent(base_indent);
    let packed_hanging = pack_block_indent(hanging_indent);
    for (i, l) in lines.iter().enumerate() {
        let is_last = i + 1 == n;
        let first = i == 0;
        let mut tag = if first || marker_w == 0 {
            packed_indent
        } else {
            packed_hanging
        };
        if justify && word_spacing && !is_last && !is_code_block && marker_str.is_empty() {
            tag |= ROW_FLAG_JUSTIFY;
        }
        if first && indent_w > 0 {
            tag |= ROW_FLAG_INDENT;
        }
        if is_code_block {
            tag |= ROW_FLAG_MONO;
        }
        if bq_indent > 0 {
            tag |= ROW_FLAG_BQ;
        }
        let (text, start, end) = if marker_w > 0 && first {
            (
                format!("{marker_str}{}", l.text),
                body_start,
                body_start + marker_len + l.end,
            )
        } else {
            (
                l.text.clone(),
                body_start + marker_len + l.start,
                body_start + marker_len + l.end,
            )
        };
        all_rows.push(Row {
            text: SharedString::from(text),
            start: start as i32,
            end: end as i32,
            kind: 0,
            tag,
        });
        let row_h = if is_code_block {
            text_render::line_height(body_px * super::super::MONO_SCALE) as i32
        } else {
            line_h
        };
        row_heights.push(row_h);
    }
}

pub(super) fn push_pre_rows(
    all_rows: &mut Vec<Row>,
    row_heights: &mut Vec<i32>,
    seg_text: &str,
    body_start: usize,
    body_px: f32,
    _line_h: i32,
) {
    let pre_px = body_px * super::super::MONO_SCALE;
    let pre_lh = text_render::line_height(pre_px) as i32;
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
    let text_row =
        |all_rows: &mut Vec<Row>, row_heights: &mut Vec<i32>, s: &str, start: usize, end: usize| {
            all_rows.push(Row {
                text: SharedString::from(s),
                start: start as i32,
                end: end as i32,
                kind: 0,
                tag: ROW_FLAG_MONO,
            });
            row_heights.push(pre_lh);
        };

    if all_rows.last().is_some_and(|r| r.kind != 3) {
        gap_row(all_rows, row_heights);
    }
    let mono = text_render::TextStyle {
        mono: true,
        ..Default::default()
    };
    let mut line_off = 0usize;
    for line in seg_text.split('\n') {
        let base = body_start + line_off;
        line_off += line.len() + 1;
        if line.trim().is_empty() {
            text_row(all_rows, row_heights, "", 0, 0);
            continue;
        }
        let wrapped = word_wrap_char_based_styled(line, text_w(), pre_px, mono);
        if wrapped.is_empty() {
            text_row(all_rows, row_heights, line, base, base + line.len());
        } else {
            for l in &wrapped {
                text_row(all_rows, row_heights, &l.text, base + l.start, base + l.end);
            }
        }
    }
    gap_row(all_rows, row_heights);
}

#[cfg(test)]
mod tests {
    use super::push_pre_rows;
    use crate::rendering::layout::ROW_FLAG_MONO;

    #[test]
    fn pre_block_all_rows_readable() {
        let long_json = format!("{{{}}}", "\"k\":1,".repeat(40));
        let mut all_rows = Vec::new();
        let mut row_heights = Vec::new();
        push_pre_rows(&mut all_rows, &mut row_heights, &long_json, 19, 36.0, 48);

        let mono_rows: Vec<_> = all_rows
            .iter()
            .filter(|r| r.kind == 0 && (r.tag & ROW_FLAG_MONO) != 0 && !r.text.is_empty())
            .collect();
        assert!(
            mono_rows.len() >= 2,
            "a 200+ char line must wrap to more than one row: {}",
            mono_rows.len()
        );

        for r in &mono_rows {
            let (s, e) = (r.start as usize, r.end as usize);
            assert!(s < e, "every mono row must be readable: {r:?}");
            assert_eq!(
                r.text.as_str(),
                &long_json[s - 19..e - 19],
                "row's own text must match what it points at in seg_text"
            );
            assert!(
                !r.text.contains("Code block."),
                "placeholder must never appear: {r:?}"
            );
        }
    }

    #[test]
    fn low_density_block_keeps_real_byte_ranges_per_row() {
        let transcript =
            "Thought: The user wants to fix a failing test in the checkout module.\nAction: run the test suite and read the failure output.\nObservation: the total is off by one cent on rounding.";
        let mut all_rows = Vec::new();
        let mut row_heights = Vec::new();
        push_pre_rows(&mut all_rows, &mut row_heights, transcript, 0, 36.0, 48);

        let mono_rows: Vec<_> = all_rows
            .iter()
            .filter(|r| r.kind == 0 && (r.tag & ROW_FLAG_MONO) != 0 && !r.text.is_empty())
            .collect();
        assert!(!mono_rows.is_empty());
        for r in &mono_rows {
            let (s, e) = (r.start as usize, r.end as usize);
            assert!(s < e, "transcript rows must stay readable: {r:?}");
            assert_eq!(
                r.text.as_str(),
                &transcript[s..e],
                "row's own text must match what it points at in seg_text"
            );
        }
    }
}
