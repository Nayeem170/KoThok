// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::audio::Utterance;

use super::super::sentences_with_ranges;

pub(super) fn build_utterances(body: &str) -> Vec<Utterance> {
    let raw_utts = sentences_with_ranges(body);
    raw_utts
        .iter()
        .enumerate()
        .map(|(i, (text, start, end))| {
            let is_para_end = if i + 1 < raw_utts.len() {
                let next_start = raw_utts[i + 1].1;
                body.get(*end..next_start)
                    .is_none_or(|gap| gap.contains('\n'))
            } else {
                true
            };
            Utterance {
                text: text.clone(),
                start: *start,
                end: *end,
                para_end: is_para_end,
                page_break: None,
            }
        })
        .collect()
}
