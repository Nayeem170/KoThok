// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use kobo_core::Chapter;
use unicode_segmentation::UnicodeSegmentation;

#[derive(serde::Serialize, serde::Deserialize, Clone)]
pub struct WordHit {
    pub chapter: u16,
    pub byte_offset: u32,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Clone)]
pub struct WordIndex {
    pub words: Vec<String>,
    pub occurrences: Vec<Vec<WordHit>>,
}

impl WordIndex {
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }
}

pub fn build_word_index(chapters: &[Chapter]) -> WordIndex {
    let mut map: std::collections::BTreeMap<String, Vec<WordHit>> =
        std::collections::BTreeMap::new();
    for (ch_idx, chapter) in chapters.iter().enumerate() {
        let body = &chapter.body;
        let body_ptr = body.as_ptr() as usize;
        for word in body.unicode_words() {
            let offset = word.as_ptr() as usize - body_ptr;
            map.entry(word.to_lowercase()).or_default().push(WordHit {
                chapter: ch_idx as u16,
                byte_offset: offset as u32,
            });
        }
    }
    let (words, occurrences) = map.into_iter().unzip();
    WordIndex { words, occurrences }
}

pub const MAX_SEARCH_RESULTS: usize = 200;

#[cfg(test)]
mod tests {
    use super::*;
    use kobo_core::Chapter;

    fn ch(idx: usize, body: &str) -> Chapter {
        let xhtml = format!("<p>{body}</p>");
        let mut ch = Chapter::from_xhtml(idx, None, &xhtml);
        ch.body = body.to_string();
        ch
    }

    #[test]
    fn single_chapter_produces_sorted_unique_words() {
        let chapters = vec![ch(0, "the cat sat on the mat")];
        let idx = build_word_index(&chapters);
        assert!(idx.words.windows(2).all(|w| w[0] < w[1]), "not sorted");
        assert_eq!(
            idx.occurrences[idx.words.iter().position(|w| w == "the").unwrap()].len(),
            2,
            "the appears twice"
        );
    }

    #[test]
    fn byte_offsets_match_body_positions() {
        let chapters = vec![ch(0, "hello world")];
        let idx = build_word_index(&chapters);
        let hello_hits = &idx.occurrences[idx.words.iter().position(|w| w == "hello").unwrap()];
        assert_eq!(hello_hits[0].chapter, 0);
        assert_eq!(
            chapters[0].body[hello_hits[0].byte_offset as usize..].starts_with("hello"),
            true
        );
    }

    #[test]
    fn multi_chapter_tracks_correct_indices() {
        let chapters = vec![ch(0, "alpha beta"), ch(1, "gamma alpha")];
        let idx = build_word_index(&chapters);
        let alpha_hits = &idx.occurrences[idx.words.iter().position(|w| w == "alpha").unwrap()];
        assert_eq!(alpha_hits.len(), 2);
        assert_eq!(alpha_hits[0].chapter, 0);
        assert_eq!(alpha_hits[1].chapter, 1);
    }

    #[test]
    fn case_insensitive() {
        let chapters = vec![ch(0, "The the THE")];
        let idx = build_word_index(&chapters);
        let the_pos = idx.words.iter().position(|w| w == "the").unwrap();
        assert_eq!(idx.occurrences[the_pos].len(), 3);
    }

    #[test]
    fn empty_chapter_produces_empty_index() {
        let chapters = vec![ch(0, "")];
        let idx = build_word_index(&chapters);
        assert!(idx.is_empty());
    }

    #[test]
    fn image_only_chapter_produces_empty_index() {
        let xhtml = "<p><img src=\"cover.jpg\"/></p>";
        let mut chapter = Chapter::from_xhtml(0, None, xhtml);
        chapter.body = String::new();
        let idx = build_word_index(&[chapter]);
        assert!(idx.is_empty());
    }

    #[test]
    fn nth_occurrence_matches_nth_appearance_in_body() {
        let chapters = vec![ch(0, "cat dog cat fish cat")];
        let idx = build_word_index(&chapters);
        let cat_hits = &idx.occurrences[idx.words.iter().position(|w| w == "cat").unwrap()];
        assert_eq!(cat_hits.len(), 3);
        for hit in cat_hits {
            assert_eq!(
                chapters[0].body[hit.byte_offset as usize..].starts_with("cat"),
                true,
                "offset {} does not point to 'cat'",
                hit.byte_offset
            );
        }
    }
}
