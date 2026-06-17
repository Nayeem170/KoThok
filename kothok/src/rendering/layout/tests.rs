use super::paginate::paginate_with_heights_ext;
use super::*;
use kobo_core::Chapter;

const HEAD_PX: f32 = 60.0;

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
fn sentences_split_on_period() {
    let s = sentences_with_ranges("First one. Second one. Third.");
    assert_eq!(s.len(), 3);
    assert_eq!(s[0].0, "First one.");
    assert_eq!(s[1].0, "Second one.");
    assert_eq!(s[2].0, "Third.");
}

#[test]
fn sentences_handle_exclamation_and_question() {
    let s = sentences_with_ranges("What? Yes! Done.");
    assert_eq!(s.len(), 3);
}

#[test]
fn sentences_empty_text() {
    let s = sentences_with_ranges("");
    assert!(s.is_empty());
}

#[test]
fn sentences_keep_abbreviation_intact() {
    let s = sentences_with_ranges("Mr. Smith went home.");
    assert_eq!(s.len(), 1, "Mr. must not start its own sentence");
    assert_eq!(s[0].0, "Mr. Smith went home.");
}

#[test]
fn sentences_keep_decimal_intact() {
    let s = sentences_with_ranges("The price is 3.14 dollars today. It is cheap.");
    assert_eq!(s.len(), 2, "decimal 3.14 must not split");
    assert!(s[0].0.contains("3.14"));
}

#[test]
fn sentences_keep_initials_intact() {
    let s = sentences_with_ranges("J. K. Rowling wrote many books.");
    assert_eq!(s.len(), 1, "single-letter initials must not split");
}

#[test]
fn sentences_keep_eg_ie_intact() {
    let s = sentences_with_ranges("Fruits, e.g. apples, are healthy. Vegetables too.");
    assert_eq!(s.len(), 2, "e.g. must not split");
    assert!(s[0].0.contains("e.g."));
}

#[test]
fn sentences_keep_ellipsis_intact() {
    let s = sentences_with_ranges("He paused... then continued. The end.");
    assert_eq!(s.len(), 2, "ellipsis must not split into fragments");
    assert!(s[0].0.contains("..."));
}

#[test]
fn sentences_preserve_byte_ranges() {
    let text = "First one. Second one.";
    let s = sentences_with_ranges(text);
    assert_eq!(s.len(), 2);
    assert_eq!(&text[s[0].1..s[0].2], "First one.");
    assert_eq!(&text[s[1].1..s[1].2], "Second one.");
}

#[test]
fn sentences_split_on_danda() {
    let s = sentences_with_ranges("আমি যাচ্ছি। সে আসছে।");
    assert_eq!(s.len(), 2, "Bengali danda must split sentences");
}

#[test]
fn sentences_split_on_cjk_ideographic_stop() {
    let s = sentences_with_ranges("これは日本語です。あれも日本語です。");
    assert_eq!(s.len(), 2, "CJK ideographic stop (。) must split");
}

#[test]
fn sentences_split_on_fullwidth_punctuation() {
    let s = sentences_with_ranges("大丈夫ですか？はい、元気です！");
    assert_eq!(s.len(), 2, "fullwidth ? and ! must split CJK sentences");
}

#[test]
fn sentences_no_split_before_lowercase() {
    let s = sentences_with_ranges("config. json files are common. End.");
    assert_eq!(s.len(), 2, "period before lowercase is mid-sentence");
}

#[test]
fn sentences_split_after_common_word_no() {
    let s = sentences_with_ranges("I said no. She left.");
    assert_eq!(s.len(), 2, "'no.' at a real sentence end must split");
}

#[test]
fn sentences_split_after_name_max() {
    let s = sentences_with_ranges("His name is Max. He is tall.");
    assert_eq!(s.len(), 2, "'Max.' as a name must split");
}

#[test]
fn sentences_number_abbreviation_still_intact() {
    let s = sentences_with_ranges("See No. 5 for details. It is clear.");
    assert_eq!(s.len(), 2, "'No. 5' must not split");
    assert!(s[0].0.contains("No. 5"));
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
    let off = estimate_chapter_offsets(&chapters, 1, 7, 48);
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
    let off = estimate_chapter_offsets(&chapters, 0, 3, 48);
    assert_eq!(off, vec![0, 3]);
}

#[test]
fn count_chapter_pages_matches_build_state_pagination() {
    let xhtml = "<h1>Title</h1><p>One two three four five six seven eight.</p>";
    let mut a = chapter_with(xhtml);
    let mut b = chapter_with(xhtml);
    let st = build_state(&mut a, BODY_PX, HEAD_PX, 48);
    let counted = count_chapter_pages(&mut b, BODY_PX, 48);
    assert_eq!(
        counted,
        st.pages.len(),
        "count_chapter_pages must agree with build_state"
    );
}

#[test]
fn count_chapter_pages_empty_is_one() {
    let mut ch = chapter_with("");
    let n = count_chapter_pages(&mut ch, BODY_PX, 48);
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
