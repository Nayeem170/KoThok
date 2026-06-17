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
