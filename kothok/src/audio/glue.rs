use crate::audio::{Cmd, Utterance};
use crate::rendering::layout::ChapterState;
use log::debug;
use std::sync::mpsc;

/// Byte offset of the first content row on `page` (the top of the page text).
fn page_first_offset(page: usize, state: &ChapterState) -> usize {
    let &(row_start, row_end) = match state.pages.get(page) {
        Some(range) => range,
        None => return 0,
    };
    state.all_rows[row_start..row_end]
        .iter()
        .find(|r| r.start < r.end)
        .map(|r| r.start as usize)
        .unwrap_or(0)
}

/// Byte offset just past the last content row on `page` (the bottom of the text).
fn page_last_offset(page: usize, state: &ChapterState) -> usize {
    let &(row_start, row_end) = match state.pages.get(page) {
        Some(range) => range,
        None => return 0,
    };
    let first = page_first_offset(page, state);
    state.all_rows[row_start..row_end]
        .iter()
        .rev()
        .find(|r| r.start < r.end)
        .map(|r| r.end as usize)
        .unwrap_or(first)
}

/// The page an utterance belongs to: the page containing its START offset.
/// A sentence that spans a page boundary is read on the page where it begins,
/// so:
///   - auto-advance: TTS reads the full sentence, then page turns to the next
///     page which starts with the following sentence (no re-reading)
///   - manual start: the broken sentence's continuation is on the previous
///     page, so the new page starts with the first complete sentence
fn utterance_page(u: &Utterance, bounds: &[(usize, usize)]) -> Option<usize> {
    let npages = bounds.len();
    if npages == 0 {
        return None;
    }
    for (i, (f, l)) in bounds.iter().enumerate() {
        if u.start >= *f && u.start < *l {
            return Some(i);
        }
    }
    None
}

pub fn page_utterances(page: usize, state: &ChapterState) -> Vec<Utterance> {
    let npages = state.pages.len();
    if page >= npages {
        return Vec::new();
    }
    let bounds: Vec<(usize, usize)> = (0..npages)
        .map(|p| (page_first_offset(p, state), page_last_offset(p, state)))
        .collect();
    let page_last = bounds[page].1;
    state
        .utterances
        .iter()
        .filter(|u| utterance_page(u, &bounds) == Some(page))
        .map(|u| {
            let mut utt = u.clone();
            utt.page_break = if u.end > page_last {
                Some(page_last.saturating_sub(u.start))
            } else {
                None
            };
            utt
        })
        .collect()
}

pub fn load_page_audio(page: usize, state: &ChapterState, cmd_tx: &mpsc::Sender<Cmd>) {
    let utts = page_utterances(page, state);
    best_effort_send(cmd_tx, Cmd::Reload(utts));
    best_effort_send(cmd_tx, Cmd::Seek(0));
    debug!(
        "audio: loaded page {}/{} ({} utterances)",
        page + 1,
        state.pages.len(),
        state.utterances.len()
    );
}

/// Send an audio command to the worker, ignoring a full/closed channel. The
/// worker drives its own queue and re-derives state, so a dropped command
/// (saturated channel, or the worker already torn down) is benign. Centralises
/// the CODE_CONVENTIONS §5 best-effort reason so it isn't repeated at every
/// bare `let _ = tx.send(..)` call site.
pub fn best_effort_send(tx: &mpsc::Sender<Cmd>, cmd: Cmd) {
    let _ = tx.send(cmd);
}

pub fn utterance_index_for_offset(utterances: &[Utterance], offset: usize) -> usize {
    for (i, u) in utterances.iter().enumerate() {
        if u.start >= offset {
            return i;
        }
    }
    utterances.len().saturating_sub(1)
}

#[cfg(test)]
mod tests;
