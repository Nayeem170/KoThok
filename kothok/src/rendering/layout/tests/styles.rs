// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;
use kobo_core::Chapter;

/// Read-aloud used to skip every chapter title: heading rows carried
/// `start: 0, end: 0` and never reached the TTS body.
#[test]
fn headings_reach_read_aloud() {
    let mut ch = Chapter::from_xhtml(0, None, "<h1>Chapter Title</h1><p>Body text here.</p>");
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42, true);
    assert!(
        st.utterances
            .iter()
            .any(|u| u.text.contains("Chapter Title")),
        "heading must be spoken: {:?}",
        st.utterances.iter().map(|u| &u.text).collect::<Vec<_>>()
    );
}

#[test]
fn heading_rows_carry_a_byte_range() {
    let mut ch = Chapter::from_xhtml(0, None, "<h2>Some Heading</h2><p>Body.</p>");
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42, true);
    let head = st
        .all_rows
        .iter()
        .find(|r| r.kind == 2)
        .expect("a heading row");
    assert!(
        head.start < head.end,
        "heading needs a range for highlight and tap: {head:?}"
    );
}

/// A link inside a heading needs a range before it can be underlined or
/// tapped; without one `row_has_runs` rejected the row outright.
#[test]
fn links_inside_a_heading_are_addressable() {
    let mut ch = Chapter::from_xhtml(
        0,
        None,
        r#"<h2><a href="ch02.xhtml">Linked Heading</a></h2>"#,
    );
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42, true);
    assert_eq!(st.links.len(), 1, "heading link is captured");
    let link = &st.links[0];
    let head = st
        .all_rows
        .iter()
        .find(|r| r.kind == 2)
        .expect("a heading row");
    assert!(
        link.start >= head.start as usize && link.end <= head.end as usize,
        "link {link:?} must sit inside heading row {}..{}",
        head.start,
        head.end
    );
    assert!(st.link_at(link.start).is_some(), "lookup resolves the link");
}

/// Headings store their *level* in `tag`, which aliases the row flag bits.
/// Nothing may read those bits off a heading.
#[test]
fn heading_levels_do_not_alias_row_flags() {
    for (tag, level) in [("h1", 1), ("h2", 2), ("h4", 4)] {
        let mut ch = Chapter::from_xhtml(0, None, &format!("<{tag}>Title</{tag}><p>Body.</p>"));
        let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42, true);
        let head = st
            .all_rows
            .iter()
            .find(|r| r.kind == 2)
            .expect("a heading row");
        assert_eq!(head.tag, level, "{tag} stores its level, not flags");
    }
}

#[test]
fn emphasis_lands_on_the_right_words() {
    let xhtml = "<p>first para plain</p><p>second has <b>BOLDWORD</b> inside</p>";
    let mut ch = Chapter::from_xhtml(0, None, xhtml);
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42, true);
    assert_eq!(st.style_runs.len(), 1, "one bold run: {:?}", st.style_runs);

    let row = st
        .all_rows
        .iter()
        .find(|r| r.kind == 0 && r.text.contains("BOLDWORD"))
        .expect("row with the bold word");
    let run = &st.style_runs[0];
    assert!(
        run.start >= row.start as usize && run.end <= row.end as usize,
        "run {run:?} outside its row {}..{}",
        row.start,
        row.end
    );
    let local = run.start - row.start as usize;
    assert!(
        row.text[local..].starts_with("BOLDWORD"),
        "run points at {:?}, not BOLDWORD",
        &row.text[local..]
    );
    assert!(run.bold && !run.italic);
}

#[test]
fn style_at_resolves_offsets() {
    use kobo_core::html_text::StyleRun;
    let runs = [
        StyleRun {
            start: 5,
            end: 10,
            bold: true,
            italic: false,
            link: false,
        },
        StyleRun {
            start: 20,
            end: 25,
            bold: false,
            italic: true,
            link: false,
        },
    ];
    assert!(style_at(&runs, 4).is_plain(), "before the first run");
    assert!(style_at(&runs, 5).bold, "inclusive start");
    assert!(style_at(&runs, 9).bold);
    assert!(style_at(&runs, 10).is_plain(), "exclusive end");
    assert!(style_at(&runs, 22).italic);
    assert!(style_at(&runs, 99).is_plain(), "past every run");
    assert!(style_at(&[], 0).is_plain(), "no runs at all");
}

#[test]
fn code_rows_stay_mono_even_with_emphasis() {
    let indents = kobo_core::html_text::parse_indents(".lvl { margin-left: 2em }");
    let xhtml = r#"<p class="lvl">x = <b>1</b>  # note</p>"#;
    let mut ch = Chapter::from_xhtml_with_indents(0, None, xhtml, &indents);
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42, true);
    for row in st.all_rows.iter().filter(|r| r.kind == 0) {
        assert_ne!(row.tag & ROW_FLAG_MONO, 0, "code row lost its mono flag");
    }
}

/// Table cards indent (card layout) but are prose, not code: their rows must
/// not pick up the monospace flag. Regression for the `block_indent > 0`
/// misclassification this field exists to replace.
#[test]
fn table_card_rows_are_not_rendered_as_code() {
    let xhtml = "<table><tr><th>Key</th><th>Value</th></tr><tr><td>a</td><td>long value here</td></tr></table>";
    let mut ch = Chapter::from_xhtml(0, None, xhtml);
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42, true);
    let mono_rows: Vec<_> = st
        .all_rows
        .iter()
        .filter(|r| r.kind == 0 && (r.tag & ROW_FLAG_MONO) != 0)
        .collect();
    assert!(
        mono_rows.is_empty(),
        "table-card rows must not be monospace code: {mono_rows:?}"
    );
}

/// Nested list items indent to their depth but are prose, not code.
#[test]
fn nested_list_rows_are_not_rendered_as_code() {
    let xhtml = "<ul><li>top item<ul><li>inner item with several words</li></ul></li></ul>";
    let mut ch = Chapter::from_xhtml(0, None, xhtml);
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42, true);
    let mono_rows: Vec<_> = st
        .all_rows
        .iter()
        .filter(|r| r.kind == 0 && (r.tag & ROW_FLAG_MONO) != 0)
        .collect();
    assert!(
        mono_rows.is_empty(),
        "nested list rows must not be monospace code: {mono_rows:?}"
    );
}
