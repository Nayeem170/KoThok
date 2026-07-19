// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
mod sentences;
mod styles;

use super::paginate::paginate_with_heights_ext;
use super::*;
use kobo_core::Chapter;

pub(super) const HEAD_PX: f32 = 60.0;

#[test]
fn paginate_fits_in_single_page() {
    let heights = vec![42, 42, 42];
    let pages = paginate_with_heights_ext(&heights, 200, &[]);
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0], (0, 3));
}

#[test]
fn paginate_splits_when_exceeding_height() {
    let heights = vec![100, 100, 100, 100];
    let pages = paginate_with_heights_ext(&heights, 250, &[]);
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0], (0, 2));
    assert_eq!(pages[1], (2, 4));
}

#[test]
fn paginate_single_tall_item_gets_own_page() {
    let heights = vec![500, 42];
    let pages = paginate_with_heights_ext(&heights, 200, &[]);
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0], (0, 1));
    assert_eq!(pages[1], (1, 2));
}

#[test]
fn paginate_empty_returns_one_empty_page() {
    let pages = paginate_with_heights_ext(&[], 200, &[]);
    assert_eq!(pages.len(), 1);
    assert_eq!(pages[0], (0, 0));
}

#[test]
fn word_wrap_short_text_one_line() {
    let lines = word_wrap_bytes("Hello", 936, BODY_PX);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].text, "Hello");
}

#[test]
fn word_wrap_ranges_cover_full_text() {
    let text = "Hello world this is a test sentence for wrapping.";
    let lines = word_wrap_bytes(text, 200, BODY_PX);
    assert!(lines.len() > 1, "should wrap to multiple lines");
    assert_eq!(lines.first().unwrap().start, 0);
    assert_eq!(lines.last().unwrap().end, text.len());
}

#[test]
fn word_wrap_empty_text() {
    let lines = word_wrap_bytes("", 936, BODY_PX);
    assert!(lines.is_empty());
}

#[test]
fn paginate_ext_pushes_orphan_heading_to_next_page() {
    let heights = vec![40, 40, 40, 40, 40];
    let heading_indices = vec![2];
    let pages = paginate_with_heights_ext(&heights, 140, &heading_indices);
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0], (0, 2));
    assert_eq!(pages[1], (2, 5));
}

#[test]
fn paginate_ext_heading_at_page_end_isnt_doubled() {
    let heights = vec![40, 40, 40, 40, 40, 40];
    let heading_indices = vec![1];
    let pages = paginate_with_heights_ext(&heights, 120, &heading_indices);
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0], (0, 3));
    assert_eq!(pages[1], (3, 6));
}

#[test]
fn paginate_ext_no_headings_falls_back() {
    let heights = vec![100, 100, 100, 100];
    let pages = paginate_with_heights_ext(&heights, 250, &[]);
    assert_eq!(pages.len(), 2);
    assert_eq!(pages[0], (0, 2));
    assert_eq!(pages[1], (2, 4));
}

fn chapter_with(text: &str) -> Chapter {
    Chapter::from_xhtml(0, None, text)
}

#[test]
fn build_state_text_only_produces_rows_and_pages() {
    let xhtml = "<h1>Title</h1><p>One two three four five six seven eight nine ten.</p>";
    let mut ch = chapter_with(xhtml);
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 48);
    assert!(!st.all_rows.is_empty(), "text must yield rows");
    assert!(!st.pages.is_empty(), "must produce at least one page");
    for &(s, e) in &st.pages {
        assert!(
            s <= e && e <= st.all_rows.len(),
            "page range [{s},{e}) out of bounds"
        );
    }
}

#[test]
fn build_state_empty_chapter_safe() {
    let mut ch = chapter_with("");
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 48);
    for &(s, e) in &st.pages {
        assert!(s <= st.all_rows.len());
        assert!(e <= st.all_rows.len());
    }
}

#[test]
fn build_state_pages_cover_all_rows() {
    let mut body = String::from("<p>");
    for i in 0..60 {
        body.push_str(&format!("Line number {i} of many lines of body text. "));
    }
    body.push_str("</p>");
    let mut ch = chapter_with(&body);
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 48);
    let covered: usize = st.pages.iter().map(|(s, e)| e - s).sum();
    assert_eq!(
        covered,
        st.all_rows.len(),
        "pages must cover all rows exactly"
    );
}

#[test]
fn build_state_stable_across_font_sizes() {
    let xhtml = "<p>The quick brown fox jumps over the lazy dog repeatedly.</p>";
    for line_h in [32, 48, 64] {
        let mut ch = chapter_with(xhtml);
        let st = build_state(&mut ch, BODY_PX, HEAD_PX, line_h);
        assert!(!st.pages.is_empty(), "line_h={line_h} produced no pages");
        for &(s, e) in &st.pages {
            assert!(s <= e && e <= st.all_rows.len());
        }
    }
}

#[test]
fn build_state_row_heights_match_all_rows() {
    let xhtml = "<h1>Heading</h1><p>Body paragraph of normal length here.</p>";
    let mut ch = chapter_with(xhtml);
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 48);
    assert_eq!(
        st.row_heights.len(),
        st.all_rows.len(),
        "one height per row"
    );
    for &h in &st.row_heights {
        assert!(h > 0, "row height must be positive");
    }
}

#[test]
fn estimate_chapter_offsets_monotonic_and_uses_known_current() {
    let ch0 = chapter_with("<p>short</p>");
    let ch1 = chapter_with("<p>the current chapter we are reading right now</p>");
    let chapters = vec![ch0, ch1];
    let layout = screen_layout();
    let off = estimate_chapter_offsets(&chapters, (1, 7), 48, &layout);
    assert_eq!(off.len(), chapters.len() + 1, "offsets array is chapters+1");
    assert_eq!(off[0], 0, "first offset starts at 0");
    assert!(
        off.windows(2).all(|w| w[1] >= w[0]),
        "offsets are non-decreasing"
    );
    assert_eq!(
        off[2] - off[1],
        7,
        "current chapter uses the supplied page count"
    );
}

#[test]
fn estimate_chapter_offsets_single_chapter() {
    let chapters = vec![chapter_with("<p>solo</p>")];
    let layout = screen_layout();
    let off = estimate_chapter_offsets(&chapters, (0, 3), 48, &layout);
    assert_eq!(off, vec![0, 3]);
}

#[test]
fn count_chapter_pages_matches_build_state_pagination() {
    let xhtml = "<h1>Title</h1><p>One two three four five six seven eight.</p>";
    let mut a = chapter_with(xhtml);
    let mut b = chapter_with(xhtml);
    let st = build_state(&mut a, BODY_PX, HEAD_PX, 48);
    let layout = screen_layout();
    let counted = count_chapter_pages(&mut b, BODY_PX, 48, &layout);
    assert_eq!(
        counted,
        st.pages.len(),
        "count_chapter_pages must agree with build_state"
    );
}

#[test]
fn count_chapter_pages_empty_is_one() {
    let mut ch = chapter_with("");
    let layout = screen_layout();
    let n = count_chapter_pages(&mut ch, BODY_PX, 48, &layout);
    assert!(n >= 1, "an empty chapter still occupies one page");
}

#[test]
fn cjk_text_wraps_per_character() {
    let text = "これは日本語のテストです。これは日本語のテストです。これは日本語のテストです。";
    let lines = word_wrap_bytes(text, 100, BODY_PX);
    assert!(
        lines.len() > 1,
        "CJK text must wrap to multiple lines with narrow width"
    );
    for line in &lines {
        assert!(
            line.width <= 200.0,
            "no line should significantly exceed max width"
        );
    }
    assert_eq!(lines.first().unwrap().start, 0);
    assert_eq!(lines.last().unwrap().end, text.len());
}

#[test]
fn thai_text_wraps_per_character() {
    let text = "กระต่ายตัวหนึ่งอาศัยอยู่ในป่ามันมีหูยาวและขนสวยกระต่ายตัวหนึ่งอาศัย";
    let lines = word_wrap_bytes(text, 100, BODY_PX);
    assert!(
        lines.len() > 1,
        "Thai text must wrap to multiple lines with narrow width"
    );
}

#[test]
fn latin_text_still_uses_word_wrapping() {
    let text = "hello world this is a test of word wrapping for latin text";
    let lines = word_wrap_bytes(text, 200, BODY_PX);
    assert!(lines.len() > 1, "Latin text should wrap with narrow width");
    for line in &lines {
        assert!(
            !line.text.is_empty(),
            "no empty lines from word-based wrapping"
        );
    }
}

#[test]
fn cjk_lines_have_width_field() {
    let text = "これはテストですこれはテストですこれはテストです";
    let lines = word_wrap_bytes(text, 100, BODY_PX);
    for line in &lines {
        assert!(line.width > 0.0, "every non-empty line should have width");
    }
}

/// End to end: a stylesheet `margin-left` has to survive extraction, layout,
/// and the `Row::tag` packing, or a code listing renders flush left.
#[test]
fn indented_block_reaches_the_row_and_suppresses_prose_devices() {
    let indents = kobo_core::html_text::parse_indents(".lvl { margin-left: 2em }");
    let xhtml = r#"<p>An ordinary paragraph of prose that is long enough to wrap onto a second line somewhere.</p><p class="lvl">    if x: return x</p>"#;
    let mut ch = Chapter::from_xhtml_with_indents(0, None, xhtml, &indents);
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42);

    let body: Vec<&crate::Row> = st.all_rows.iter().filter(|r| r.kind == 0).collect();
    let flush: Vec<&&crate::Row> = body.iter().filter(|r| block_indent_px(r) == 0).collect();
    let indented: Vec<&&crate::Row> = body.iter().filter(|r| block_indent_px(r) > 0).collect();

    assert!(!flush.is_empty(), "prose rows should stay flush");
    assert_eq!(indented.len(), 1, "the one code line should be indented");
    assert_eq!(
        block_indent_px(indented[0]),
        block_indent_for(4.0, BODY_PX, text_w()),
        "indent should combine the stylesheet's 2em with the leading spaces"
    );
    assert_eq!(indented[0].tag & ROW_FLAG_JUSTIFY, 0);
    assert_eq!(indented[0].tag & ROW_FLAG_INDENT, 0);
}

#[test]
fn block_indent_never_eats_more_than_a_third_of_the_column() {
    assert!(block_indent_for(99.0, BODY_PX, text_w()) <= text_w() / 3);
    assert_eq!(block_indent_for(0.0, BODY_PX, text_w()), 0);
}

/// The packing shares `Row::tag` with the flag bits, so round-tripping must not
/// let an indent corrupt a flag or vice versa.
#[test]
fn packed_indent_and_flags_do_not_collide() {
    for px in [0usize, 1, 37, MAX_BLOCK_INDENT_PX] {
        let tag = pack_block_indent(px) | ROW_FLAG_JUSTIFY | ROW_FLAG_INDENT;
        let row = crate::Row {
            text: Default::default(),
            start: 0,
            end: 0,
            kind: 0,
            tag,
        };
        assert_eq!(block_indent_px(&row), px);
        assert_ne!(tag & ROW_FLAG_JUSTIFY, 0);
        assert_ne!(tag & ROW_FLAG_INDENT, 0);
    }
}

/// Word wrapping collapses runs of spaces, which is invisible in prose and
/// ruinous in a listing: alignment within a line is exactly what the indent work
/// set out to preserve. Code blocks must wrap by character instead.
#[test]
fn code_blocks_keep_their_internal_spacing() {
    let indents = kobo_core::html_text::parse_indents(".lvl { margin-left: 2em }");
    let xhtml = r#"<p class="lvl">x = 1      # aligned comment</p>"#;
    let mut ch = Chapter::from_xhtml_with_indents(0, None, xhtml, &indents);
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42);
    let joined: String = st
        .all_rows
        .iter()
        .filter(|r| r.kind == 0)
        .map(|r| r.text.to_string())
        .collect();
    assert!(
        joined.contains("1      #"),
        "run of spaces was collapsed: {joined:?}"
    );
}

#[test]
fn prose_still_wraps_by_word() {
    let xhtml = "<p>Ordinary prose with     irregular spacing that should normalise.</p>";
    let mut ch = Chapter::from_xhtml(0, None, xhtml);
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42);
    let joined: String = st
        .all_rows
        .iter()
        .filter(|r| r.kind == 0)
        .map(|r| r.text.to_string())
        .collect();
    assert!(
        !joined.contains("with     irregular"),
        "prose should not keep raw spacing runs: {joined:?}"
    );
}
