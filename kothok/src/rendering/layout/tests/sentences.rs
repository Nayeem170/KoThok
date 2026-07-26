// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;

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
