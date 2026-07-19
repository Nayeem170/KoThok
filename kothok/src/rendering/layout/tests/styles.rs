// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;
use kobo_core::Chapter;

#[test]
fn emphasis_lands_on_the_right_words() {
    let xhtml = "<p>first para plain</p><p>second has <b>BOLDWORD</b> inside</p>";
    let mut ch = Chapter::from_xhtml(0, None, xhtml);
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42);
    assert_eq!(st.style_runs.len(), 1, "one bold run: {:?}", st.style_runs);

    let row = st
        .all_rows
        .iter()
        .find(|r| r.kind == 0 && r.text.contains("BOLDWORD"))
        .expect("row with the bold word");
    let run = st.style_runs[0];
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
        },
        StyleRun {
            start: 20,
            end: 25,
            bold: false,
            italic: true,
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
    let st = build_state(&mut ch, BODY_PX, HEAD_PX, 42);
    for row in st.all_rows.iter().filter(|r| r.kind == 0) {
        assert_ne!(row.tag & ROW_FLAG_MONO, 0, "code row lost its mono flag");
    }
}
