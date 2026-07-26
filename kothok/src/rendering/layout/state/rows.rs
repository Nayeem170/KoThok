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

/// Emit a figure: the picture, then its caption on its own wrapped rows.
///
/// The image is looked up by `src`, not by position. `load_images` skips any
/// resource it cannot find in the archive, so a positional index silently
/// shifted every later figure onto the previous figure's bitmap.
///
/// The caption is emitted whether or not the picture decodes -- a missing
/// image is not a reason to lose the words describing it -- and goes into the
/// TTS body like any other prose.
pub(super) fn push_figure_rows(
    all_rows: &mut Vec<Row>,
    row_heights: &mut Vec<i32>,
    decoded_images: &mut HashMap<usize, crate::rendering::text_render::DecodedImage>,
    body: &mut String,
    seg: &kobo_core::TextSegment,
    chapter_images: &HashMap<&str, &[u8]>,
    body_px: f32,
    line_h: i32,
) {
    // Separate the figure from the prose above it, the way a paragraph
    // separates itself. Without this a figure butts against the preceding
    // paragraph while still leaving a gap below it.
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
    if !body.is_empty() {
        body.push('\n');
    }
    let cs = body.len();
    body.push_str(cap);
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

/// A heading, wrapped and sized by level.
///
/// The text goes into the TTS body like any other block. Headings used to be
/// excluded, which meant read-aloud silently skipped every chapter title, and
/// a link inside a heading had no byte range so it could be neither underlined
/// nor tapped.
pub(super) fn push_heading_rows(
    all_rows: &mut Vec<Row>,
    row_heights: &mut Vec<i32>,
    body: &mut String,
    seg_text: &str,
    tag: &str,
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
    if !body.is_empty() {
        body.push('\n');
    }
    let cs = body.len();
    body.push_str(trimmed);
    let lines = word_wrap_bytes(trimmed, text_w(), heading_px);
    for l in &lines {
        all_rows.push(Row {
            text: SharedString::from(l.text.clone()),
            start: (cs + l.start) as i32,
            end: (cs + l.end) as i32,
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
    body: &mut String,
    seg_text: &str,
    tag: &str,
    indent_em: f32,
    body_px: f32,
    line_h: i32,
    seg: &kobo_core::TextSegment,
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
    let marker = seg.list_marker();
    let marker_str = marker.as_deref().unwrap_or("");
    let full_text = format!("{marker_str}{seg_text}");
    body.push_str(&full_text);
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
    // A Calibre code-listing margin is the one indent that should render as
    // code; list items and table cards also indent but through other fields,
    // so they render as prose.
    let is_list = seg.list.is_some() || tag == "li";
    let is_code_block = seg.code_indent;
    // A wrapped list item hangs: the marker sits in the left inset and every
    // continuation line clears it, rather than running back underneath it.
    let marker_w = if marker_str.is_empty() {
        0
    } else {
        text_render::word_width(marker_str, body_px) as usize
    };
    // Clamp to what the packed `tag` field can actually carry, and wrap against
    // that same number. `pack_block_indent` clamps silently, so subtracting the
    // unclamped sum here would wrap the continuation lines for a narrower
    // column than they are then drawn in -- reachable for a list nested inside
    // a blockquote.
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
    // Wrap the item's own text: the marker is prepended to the first line
    // afterwards, so it cannot be split off onto a line of its own.
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
        if word_spacing && !is_last && !is_code_block && marker_str.is_empty() {
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
        // Offsets stay in `full_text` space, which is what `body` holds, so the
        // marker's bytes belong to the first row and TTS highlighting and the
        // link table line up with what is drawn.
        let (text, start, end) = if marker_w > 0 && first {
            (
                format!("{marker_str}{}", l.text),
                cs,
                cs + marker_str.len() + l.end,
            )
        } else {
            (
                l.text.clone(),
                cs + marker_str.len() + l.start,
                cs + marker_str.len() + l.end,
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

/// Verbatim code block. Emits one row per source line, preserving indentation;
/// lines wider than the text column char-wrap (keeping spaces) rather than
/// reflowing as prose.
///
/// The text goes into the TTS body like any other block. A `<pre>` is not
/// always code -- transcripts, worked examples and terminal sessions are all
/// prose the reader expects to be read aloud -- and leaving it out silently
/// skipped part of the chapter.
fn symbol_density(text: &str) -> f32 {
    if text.is_empty() {
        return 0.0;
    }
    let total = text.chars().count() as f32;
    let symbol = text
        .chars()
        .filter(|c| !c.is_alphanumeric() && !c.is_whitespace())
        .count() as f32;
    symbol / total
}

const CODE_BLOCK_DENSITY_THRESHOLD: f32 = 0.15;

/// Byte range for the next visual row of a dense (placeholder-spoken) block.
///
/// Exactly one row in the whole block -- the first -- gets the placeholder's
/// real range in `body`, so TTS speaks it once. Every other row, on any
/// physical line, gets `(end, end)`: a zero-width range at a valid boundary,
/// the same "not readable" convention gap/image rows already use. Handing a
/// later row `base + l.start`/`base + l.end` measured against the *original*
/// long line -- what this replaced -- pointed past the short placeholder into
/// whatever real text follows later in `body`, or past `body`'s end entirely.
fn dense_row_range(cs: usize, placeholder_len: usize, first: &mut bool) -> (usize, usize) {
    let end = cs + placeholder_len;
    if *first {
        *first = false;
        (cs, end)
    } else {
        (end, end)
    }
}

pub(super) fn push_pre_rows(
    all_rows: &mut Vec<Row>,
    row_heights: &mut Vec<i32>,
    body: &mut String,
    seg_text: &str,
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
    let dense = symbol_density(seg_text) >= CODE_BLOCK_DENSITY_THRESHOLD;
    let placeholder = "Code block.";
    let body_text: &str = if dense { placeholder } else { seg_text };
    if !body.is_empty() {
        body.push('\n');
    }
    let cs = body.len();
    body.push_str(body_text);
    let mono = text_render::TextStyle {
        mono: true,
        ..Default::default()
    };
    let mut dense_first = true;
    let mut line_off = 0usize;
    for line in seg_text.split('\n') {
        let base = cs + line_off;
        line_off += line.len() + 1;
        if line.trim().is_empty() {
            text_row(all_rows, row_heights, "", 0, 0);
            continue;
        }
        // Every physical line wraps, dense or not -- a dense block is not
        // always a single line (pretty-printed JSON spans several), and a
        // continuation line skipping the wrap the first line gets would
        // overflow the column exactly the way issue 16 already fixed for the
        // first line only.
        let wrapped = word_wrap_char_based_styled(line, text_w(), pre_px, mono);
        if wrapped.is_empty() {
            let (s, e) = if dense {
                dense_row_range(cs, placeholder.len(), &mut dense_first)
            } else {
                (base, base + line.len())
            };
            text_row(all_rows, row_heights, line, s, e);
        } else {
            for l in &wrapped {
                let (s, e) = if dense {
                    dense_row_range(cs, placeholder.len(), &mut dense_first)
                } else {
                    (base + l.start, base + l.end)
                };
                text_row(all_rows, row_heights, &l.text, s, e);
            }
        }
    }
    gap_row(all_rows, row_heights);
}

#[cfg(test)]
mod tests {
    use super::{push_pre_rows, symbol_density};
    use crate::rendering::layout::ROW_FLAG_MONO;

    #[test]
    fn density_zero_for_plain_text() {
        assert!(symbol_density("Hello world this is prose") < 0.05);
    }

    #[test]
    fn density_high_for_json() {
        let json = r#"{"key": "value", "list": [1, 2, 3]}"#;
        assert!(symbol_density(json) > 0.15);
    }

    #[test]
    fn density_zero_for_empty() {
        assert_eq!(symbol_density(""), 0.0);
    }

    #[test]
    fn density_low_for_transcript() {
        let transcript = "Speaker A: Hello there.\nSpeaker B: Good morning. How can I help you?\nSpeaker A: I have a question about the report.";
        assert!(symbol_density(transcript) < 0.15);
    }

    #[test]
    fn density_high_for_xml() {
        let xml = "<root><item attr=\"val\">text</item></root>";
        assert!(symbol_density(xml) > 0.15);
    }

    #[test]
    fn density_high_for_code_with_braces() {
        let code = "fn main() {\n    let x = 42;\n    println!(\"{}\", x);\n}";
        assert!(symbol_density(code) > 0.15);
    }

    /// The bug this pins: a wrapped continuation row of a dense block used to
    /// get `base + l.start`/`base + l.end` measured against the *original*
    /// long line, not the short placeholder actually written into `body`. For
    /// a placeholder eleven bytes long, that pointed past it into whatever
    /// text follows later in the chapter, or past `body`'s end outright.
    /// Exactly one row -- the first -- may carry a real (readable) range;
    /// every other row in the block must be a valid zero-width boundary.
    #[test]
    fn dense_block_has_exactly_one_readable_row_at_the_placeholder() {
        // Repeated `"k":1,` pairs rather than one long alphanumeric run --
        // alphanumeric filler (e.g. a repeated letter) doesn't count toward
        // symbol density and would make this fixture wrap without being dense.
        let long_json = format!("{{{}}}", "\"k\":1,".repeat(40));
        let mut all_rows = Vec::new();
        let mut row_heights = Vec::new();
        let mut body = String::from("preceding text. ");
        push_pre_rows(
            &mut all_rows,
            &mut row_heights,
            &mut body,
            &long_json,
            36.0,
            48,
        );
        // Real chapter content keeps appending after this block, the same way
        // `build_state` would for a later segment -- if a row's range leaked
        // past the placeholder, this is what it would incorrectly point into.
        body.push_str("trailing prose that must not be misread as code.");

        let mono_rows: Vec<_> = all_rows
            .iter()
            .filter(|r| r.kind == 0 && (r.tag & ROW_FLAG_MONO) != 0)
            .collect();
        assert!(
            mono_rows.len() >= 2,
            "a 200+ char line must wrap to more than one row: {}",
            mono_rows.len()
        );

        let readable: Vec<_> = mono_rows
            .iter()
            .filter(|r| (r.start as usize) < (r.end as usize))
            .collect();
        assert_eq!(
            readable.len(),
            1,
            "exactly one row may be readable: {mono_rows:?}"
        );
        let (s, e) = (readable[0].start as usize, readable[0].end as usize);
        assert_eq!(
            &body[s..e],
            "Code block.",
            "the readable row must slice to exactly the placeholder"
        );

        for r in mono_rows.iter().filter(|r| r.start == r.end) {
            let at = r.start as usize;
            assert!(
                at <= body.len(),
                "an unreadable row's boundary must stay a valid index into body: {r:?}"
            );
            assert_ne!(
                at, s,
                "an unreadable row must not alias the placeholder's own start"
            );
        }
    }

    /// The companion path: a low-density block (a prose transcript, per issue
    /// 7) keeps its real per-row byte ranges -- only dense blocks fall back to
    /// a placeholder, and this must not regress for the common case.
    #[test]
    fn low_density_block_keeps_real_byte_ranges_per_row() {
        let transcript =
            "Thought: The user wants to fix a failing test in the checkout module.\nAction: run the test suite and read the failure output.\nObservation: the total is off by one cent on rounding.";
        let mut all_rows = Vec::new();
        let mut row_heights = Vec::new();
        let mut body = String::new();
        push_pre_rows(
            &mut all_rows,
            &mut row_heights,
            &mut body,
            transcript,
            36.0,
            48,
        );

        let mono_rows: Vec<_> = all_rows
            .iter()
            .filter(|r| r.kind == 0 && (r.tag & ROW_FLAG_MONO) != 0 && !r.text.is_empty())
            .collect();
        assert!(!mono_rows.is_empty());
        for r in &mono_rows {
            let (s, e) = (r.start as usize, r.end as usize);
            assert!(s < e, "transcript rows must stay readable: {r:?}");
            assert_eq!(
                &body[s..e],
                r.text.as_str(),
                "row's own text must match what it points at in body"
            );
        }
    }
}
