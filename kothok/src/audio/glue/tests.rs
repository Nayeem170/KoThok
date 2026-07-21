// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;
use crate::rendering::layout::ChapterState;
use crate::Row;

fn sample_rows() -> Vec<Row> {
    vec![
        Row {
            text: "line1".into(),
            start: 0,
            end: 10,
            kind: 0,
            tag: 0,
        },
        Row {
            text: "line2".into(),
            start: 10,
            end: 25,
            kind: 0,
            tag: 0,
        },
        Row {
            text: "line3".into(),
            start: 25,
            end: 40,
            kind: 0,
            tag: 0,
        },
        Row {
            text: "line4".into(),
            start: 40,
            end: 55,
            kind: 0,
            tag: 0,
        },
    ]
}

fn sample_utterances() -> Vec<Utterance> {
    vec![
        Utterance {
            text: "Sentence one.".into(),
            start: 0,
            end: 10,
            para_end: false,
            page_break: None,
        },
        Utterance {
            text: "Sentence two.".into(),
            start: 12,
            end: 25,
            para_end: false,
            page_break: None,
        },
        Utterance {
            text: "Sentence three.".into(),
            start: 25,
            end: 38,
            para_end: true,
            page_break: None,
        },
        Utterance {
            text: "Sentence four.".into(),
            start: 40,
            end: 52,
            para_end: false,
            page_break: None,
        },
    ]
}

fn make_chapter_state() -> ChapterState {
    ChapterState {
        all_rows: sample_rows(),
        row_heights: vec![50, 50, 50, 50],
        pages: vec![(0, 2), (2, 4)],
        utterances: sample_utterances(),
        decoded_images: std::collections::HashMap::new(),
        style_runs: Vec::new(),
    }
}

#[test]
fn utterance_index_finds_exact_start() {
    let utts = vec![
        Utterance {
            text: "A".into(),
            start: 0,
            end: 10,
            para_end: false,
            page_break: None,
        },
        Utterance {
            text: "B".into(),
            start: 10,
            end: 20,
            para_end: false,
            page_break: None,
        },
        Utterance {
            text: "C".into(),
            start: 20,
            end: 30,
            para_end: true,
            page_break: None,
        },
    ];
    assert_eq!(utterance_index_for_offset(&utts, 0), 0);
    assert_eq!(utterance_index_for_offset(&utts, 10), 1);
    assert_eq!(utterance_index_for_offset(&utts, 20), 2);
}

#[test]
fn utterance_index_skips_overlapping_sentence() {
    let utts = vec![
        Utterance {
            text: "long".into(),
            start: 0,
            end: 15,
            para_end: false,
            page_break: None,
        },
        Utterance {
            text: "next".into(),
            start: 15,
            end: 30,
            para_end: true,
            page_break: None,
        },
    ];
    assert_eq!(utterance_index_for_offset(&utts, 10), 1);
}

#[test]
fn utterance_index_offset_past_end_returns_last() {
    let utts = vec![
        Utterance {
            text: "A".into(),
            start: 0,
            end: 10,
            para_end: false,
            page_break: None,
        },
        Utterance {
            text: "B".into(),
            start: 10,
            end: 20,
            para_end: true,
            page_break: None,
        },
    ];
    assert_eq!(utterance_index_for_offset(&utts, 100), 1);
}

#[test]
fn utterance_index_empty_returns_zero() {
    let utts: Vec<Utterance> = vec![];
    assert_eq!(utterance_index_for_offset(&utts, 0), 0);
}

#[test]
fn page_utterances_returns_correct_page() {
    let state = make_chapter_state();
    let utts = page_utterances(0, &state);
    assert_eq!(utts.len(), 2);
    assert_eq!(utts[0].start, 0);
    assert_eq!(utts[1].start, 12);
}

#[test]
fn page_utterances_second_page() {
    let state = make_chapter_state();
    let utts = page_utterances(1, &state);
    assert_eq!(utts.len(), 2);
    assert_eq!(utts[0].start, 25);
    assert_eq!(utts[1].start, 40);
}

#[test]
fn page_utterances_out_of_range_returns_empty() {
    let state = make_chapter_state();
    assert!(page_utterances(99, &state).is_empty());
}

#[test]
fn page_utterances_spanning_stays_on_start_page() {
    let state = ChapterState {
        all_rows: vec![
            Row {
                text: "a".into(),
                start: 0,
                end: 10,
                kind: 0,
                tag: 0,
            },
            Row {
                text: "b".into(),
                start: 10,
                end: 30,
                kind: 0,
                tag: 0,
            },
        ],
        row_heights: vec![20, 20],
        pages: vec![(0, 1), (1, 2)],
        utterances: vec![
            Utterance {
                text: "spans".into(),
                start: 5,
                end: 25,
                para_end: false,
                page_break: None,
            },
            Utterance {
                text: "native".into(),
                start: 25,
                end: 30,
                para_end: true,
                page_break: None,
            },
        ],
        decoded_images: std::collections::HashMap::new(),
        style_runs: Vec::new(),
    };
    let utts0 = page_utterances(0, &state);
    assert_eq!(utts0.len(), 1, "spanning sentence on its start page");
    assert_eq!(utts0[0].start, 5);
    let utts1 = page_utterances(1, &state);
    assert_eq!(utts1.len(), 1, "next page has only sentence starting there");
    assert_eq!(utts1[0].start, 25);
}

#[test]
fn page_utterances_spanning_attributed_to_start_page() {
    let state = ChapterState {
        all_rows: vec![
            Row {
                text: "a".into(),
                start: 0,
                end: 10,
                kind: 0,
                tag: 0,
            },
            Row {
                text: "b".into(),
                start: 10,
                end: 30,
                kind: 0,
                tag: 0,
            },
        ],
        row_heights: vec![20, 20],
        pages: vec![(0, 1), (1, 2)],
        utterances: vec![
            Utterance {
                text: "s1".into(),
                start: 0,
                end: 8,
                para_end: false,
                page_break: None,
            },
            Utterance {
                text: "s2".into(),
                start: 8,
                end: 22,
                para_end: false,
                page_break: None,
            },
            Utterance {
                text: "s3".into(),
                start: 22,
                end: 30,
                para_end: true,
                page_break: None,
            },
        ],
        decoded_images: std::collections::HashMap::new(),
        style_runs: Vec::new(),
    };
    let p0 = page_utterances(0, &state);
    let p1 = page_utterances(1, &state);
    assert_eq!(p0.len(), 2, "page 0: s1 + s2 (s2 starts here)");
    assert_eq!(p1.len(), 1, "page 1: only s3 (starts here)");
    assert_eq!(p0.len() + p1.len(), 3, "all sentences read exactly once");
}

#[test]
fn page_utterances_empty_chapter() {
    let state = ChapterState {
        all_rows: vec![],
        row_heights: vec![],
        pages: vec![(0, 0)],
        utterances: vec![],
        decoded_images: std::collections::HashMap::new(),
        style_runs: Vec::new(),
    };
    assert!(page_utterances(0, &state).is_empty());
}

#[test]
fn utterance_index_single_utterance() {
    let utts = vec![Utterance {
        text: "only".into(),
        start: 0,
        end: 30,
        para_end: true,
        page_break: None,
    }];
    assert_eq!(utterance_index_for_offset(&utts, 0), 0);
    assert_eq!(utterance_index_for_offset(&utts, 15), 0);
    assert_eq!(utterance_index_for_offset(&utts, 999), 0);
}

#[test]
fn utterance_index_offset_between_sentences_picks_next() {
    let utts = vec![
        Utterance {
            text: "a".into(),
            start: 0,
            end: 10,
            para_end: false,
            page_break: None,
        },
        Utterance {
            text: "b".into(),
            start: 20,
            end: 30,
            para_end: true,
            page_break: None,
        },
    ];
    assert_eq!(utterance_index_for_offset(&utts, 12), 1);
    assert_eq!(utterance_index_for_offset(&utts, 19), 1);
}

#[test]
fn page_utterances_respects_offset_window() {
    let state = make_chapter_state();
    let utts = page_utterances(1, &state);
    for u in &utts {
        assert!(
            u.start >= 25 && u.start < 55,
            "utterance start {} outside page window",
            u.start
        );
    }
}

#[test]
fn load_page_audio_sends_reload_then_seek_zero() {
    let state = make_chapter_state();
    let (tx, rx) = mpsc::channel();
    load_page_audio(0, &state, &tx);
    let first = rx.try_recv().expect("first message must be Reload");
    match first {
        Cmd::Reload(utts) => {
            assert!(!utts.is_empty(), "page 0 has utterances to reload");
        }
        other => panic!("expected Reload first, got {other:?}"),
    }
    let second = rx.try_recv().expect("second message must be Seek");
    assert!(
        matches!(second, Cmd::Seek(0)),
        "expected Seek(0), got {second:?}"
    );
    assert!(rx.try_recv().is_err(), "no further messages expected");
}

#[test]
fn load_page_audio_empty_channel_is_non_fatal() {
    let state = make_chapter_state();
    let (tx, rx) = mpsc::channel::<Cmd>();
    drop(rx);
    load_page_audio(0, &state, &tx);
}

#[test]
fn load_page_audio_out_of_range_page_sends_empty_reload() {
    let state = make_chapter_state();
    let (tx, rx) = mpsc::channel();
    load_page_audio(999, &state, &tx);
    let msg = rx.try_recv().expect("Reload still sent for OOR page");
    match msg {
        Cmd::Reload(utts) => assert!(utts.is_empty(), "OOR page reloads no utterances"),
        other => panic!("expected Reload, got {other:?}"),
    }
}

#[test]
fn no_utterance_dropped_across_all_pages_realistic_text() {
    use crate::rendering::layout::{build_state, BODY_PX};
    use kobo_core::Chapter;

    let paras = [
        "In a hole in the ground there lived a hobbit. Not a nasty, dirty, wet hole, filled with the ends of worms and an oozy smell, nor yet a dry, bare, sandy hole with nothing in it to sit down on or to eat: it was a hobbit-hole, and that means comfort.",
        "It had a perfectly round door like a porthole, painted green, with a shiny yellow brass knob in the exact middle. The door opened on to a tube-shaped hall like a tunnel: a very comfortable tunnel without smoke, with panelled walls, and floors tiled and carpeted, provided with chairs, and lots and lots of pegs for hats and coats.",
        "This hobbit was a very well-to-do hobbit, and his name was Baggins. The Bagginses had lived in the neighbourhood of The Hill for time out of mind, and people considered them very respectable, not only because most of them were rich, but also because they never had any adventures or did anything unexpected.",
        "The mother of our particular hobbit was the famous Belladonna Took, one of the three remarkable daughters of the Old Took, head of the hobbits who lived across The Water, the small river that ran at the foot of The Hill. It was often said that long ago one of the Took ancestors must have taken a fairy wife.",
        "Once upon a time, in a quiet little village nestled between rolling green hills and a dense forest, there lived an old clockmaker named Elias. His shop was a treasure trove of ticking and chiming, with clocks of every shape and size covering every wall and shelf. Some were grand father clocks that stood taller than a man, while others were tiny pocket watches no bigger than a thumbnail. Every morning, Elias would unlock the creaky wooden door of his shop, step inside, and listen to the symphony of a hundred tiny heartbeats, all ticking in slightly different rhythms.",
        "The villagers said that Elias could fix any clock, no matter how old or broken. They brought him timepieces that had been silent for decades, and he would take them apart, clean each gear and spring, oil the mechanisms, and put them back together so they ran as well as the day they were made. His hands were steady and his eyes were sharp, despite his advanced age, and he took great pride in every repair.",
        "One rainy afternoon, a young girl named Lily pushed open the door of the shop, clutching a small wooden box to her chest. She was soaking wet and shivering, but her eyes were bright with determination. She placed the box on the counter and opened it to reveal a delicate silver pocket watch, its face cracked and its hands frozen at midnight. She told Elias that it had belonged to her grandfather, who had recently passed away, and that she wanted it to tick again.",
        "Elias picked up the watch and examined it carefully under his magnifying glass. The mechanism was old and intricate, with tiny gears no wider than a grain of rice. Some of the parts were corroded, and the main spring had snapped. He told Lily that it would take several days to repair, and she agreed to come back at the end of the week.",
    ];
    let xhtml = format!(
        "<html><body>{}</body></html>",
        paras
            .iter()
            .map(|p| format!("<p>{p}</p>"))
            .collect::<String>()
    );
    let mut ch = Chapter::from_xhtml(0, None, &xhtml);
    let st = build_state(&mut ch, BODY_PX, 60.0, 48);

    assert!(st.pages.len() > 1, "test text must span multiple pages");
    assert!(!st.utterances.is_empty(), "must produce utterances");

    let total: Vec<usize> = (0..st.pages.len())
        .flat_map(|p| page_utterances(p, &st))
        .map(|u| u.start)
        .collect();
    let dropped: Vec<&Utterance> = st
        .utterances
        .iter()
        .filter(|u| !total.contains(&u.start))
        .collect();
    assert!(
        dropped.is_empty(),
        "{} utterance(s) dropped across pages: {:?}",
        dropped.len(),
        dropped
            .iter()
            .map(|u| (u.start, u.end, u.text.as_str()))
            .collect::<Vec<_>>()
    );

    let mut starts = total.clone();
    starts.sort();
    starts.dedup();
    assert_eq!(
        starts.len(),
        total.len(),
        "some utterance assigned to multiple pages"
    );
}
