// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::audio::glue::{best_effort_send, page_utterances};
use crate::audio::Cmd;
use crate::rendering::layout::ChapterState;
use crate::Reader;

pub mod events;
pub mod render;
pub mod sleep_wake;

pub use events::*;
pub use render::*;
pub use sleep_wake::*;

pub use crate::panel::process_panel_callbacks;

#[cfg(test)]
mod tests;

pub struct AudioFlags {
    pub ui_changed: bool,
    pub page_changed: bool,
    pub text_dirty: bool,
}

/// Updated reading cursor after a play/pause toggle (so "Reading" can return
/// to the line that resumed).
pub struct PlayToggle {
    pub ch: usize,
    pub pg: usize,
    pub off: usize,
    pub end: usize,
}

/// Toggle playback with the same resume rules as the centre double-tap:
///  - playing -> pause
///  - paused/idle -> play from the cursor if it's on this page, else from the
///    page's first line. Shared by the footer Play/Pause button.
///
/// Resume never trusts the driver's queue position. Browsing (swipe, progress
/// drag, jump-to-reading) reloads the driver with the browsed-to page at
/// utterance 0 while the cursor stays where the reader stopped, so a bare
/// `Cmd::Play` resumed at the top of that page instead of at the cursor. The
/// queue is rebuilt from the cursor on every start; a pause/resume with no
/// navigation in between replays the current sentence from its beginning
/// (its PCM is cached, so nothing is re-synthesised).
pub fn toggle_playback(
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    state: &ChapterState,
    current_page: usize,
    chapter_offsets: &[usize],
    current_chapter: usize,
) -> PlayToggle {
    if reader.get_playing() {
        reader.set_playing(false);
        reader.set_paused(true);
        best_effort_send(cmd_tx, Cmd::Pause);
        return PlayToggle {
            ch: current_chapter,
            pg: current_page,
            off: reader.get_cur_start().max(0) as usize,
            end: reader.get_cur_end().max(0) as usize,
        };
    }
    let cur = reader.get_cur_start().max(0) as usize;
    let page = current_page;
    let play_utts = queue_from_cursor(cur, page_utterances(page, state));
    match play_utts.first() {
        Some(u) => log::info!(
            "play-start: cursor={cur} page={} queue={} head=[{},{})",
            page + 1,
            play_utts.len(),
            u.start,
            u.end
        ),
        None => log::info!("play-start: cursor={cur} page={} queue empty", page + 1),
    }
    reader.set_saved_page((*chapter_offsets.get(current_chapter).unwrap_or(&0) + page) as i32);
    let cs = reader.get_cur_start();
    let (off, end) = if cs > 0 {
        (cs as usize, reader.get_cur_end().max(0) as usize)
    } else {
        (0, 0)
    };
    best_effort_send(cmd_tx, Cmd::Reload(play_utts));
    best_effort_send(cmd_tx, Cmd::Seek(0));
    best_effort_send(cmd_tx, Cmd::Play);
    reader.set_playing(true);
    reader.set_paused(false);
    PlayToggle {
        ch: current_chapter,
        pg: page,
        off,
        end,
    }
}

// Map raw audio/TTS error strings to short, user-facing messages (issue 5).
// The raw text is still logged via warn!.
fn friendly_error(m: &str) -> String {
    let lower = m.to_ascii_lowercase();
    if lower.contains("a2dp") || lower.contains("speaker") {
        "Speaker not connected - check Bluetooth".to_string()
    } else if lower.contains("ws connect")
        || lower.contains("lookup address")
        || lower.contains("try again")
        || lower.contains("synth")
        || lower.contains("tts")
    {
        "WiFi unavailable - can't reach the voice service".to_string()
    } else {
        "Playback error - see log".to_string()
    }
}

/// Pure: the decisions `enter_sleep` will act on. Extracting this makes the
/// cover-vs-splash, frontlight, and wifi power-down choices unit-testable
/// without a framebuffer or live radios. `wifi_on`/`bt_on` are user-intent
/// flags, not live connection status.
pub struct SleepPlan {
    /// Power the frontlight off on sleep.
    pub frontlight_off: bool,
    /// Power wifi off on sleep (only when the user had it on).
    pub wifi_off: bool,
    /// Power Bluetooth off on sleep (only when the user had it on). On devices
    /// with no BT adapter the dbus call hangs, so this MUST stay false when
    /// the user does not have BT enabled.
    pub bt_off: bool,
}

pub fn sleep_plan(fl_path: &Option<std::path::PathBuf>, wifi_on: bool, bt_on: bool) -> SleepPlan {
    SleepPlan {
        frontlight_off: fl_path.is_some(),
        wifi_off: wifi_on,
        bt_off: bt_on,
    }
}

/// Pure: the queue to hand the driver when playback starts at `cursor`.
///
/// Utterances before the cursor are dropped so the queue head is what plays
/// first (`Cmd::Seek(0)`), and the head is trimmed to the cursor so a start
/// from mid-sentence does not re-read the words already heard. `page_break` is
/// an offset from the utterance start, so trimming shifts it by the same amount
/// or the auto page-turn fires early.
pub fn queue_from_cursor(
    cursor: usize,
    mut utts: Vec<crate::audio::Utterance>,
) -> Vec<crate::audio::Utterance> {
    let target = resolve_start_target(cursor, &utts);
    let mut queue = if target > 0 && target < utts.len() {
        utts.split_off(target)
    } else {
        utts
    };
    if let Some(first) = queue.first_mut() {
        // Only trim when the cursor is genuinely inside the head utterance. A
        // cursor left on another page is past every offset here, and trimming
        // by that difference cut the sentence at a meaningless byte.
        if cursor > first.start && cursor < first.end {
            let off = cursor - first.start;
            if off < first.text.len() && first.text.is_char_boundary(off) {
                first.text = first.text[off..].to_string();
                first.page_break = first.page_break.map(|b| b.saturating_sub(off));
                first.start = cursor;
            }
        }
    }
    queue
}

/// Pure: which utterance index to seek to when starting playback from `cursor`.
/// Returns the utterance whose `[start, end)` range contains the cursor, or 0
/// if the cursor is outside every utterance (caller falls back to the page's
/// first text row).
pub fn resolve_start_target(cursor: usize, utts: &[crate::audio::Utterance]) -> usize {
    utts.iter()
        .position(|u| cursor >= u.start && cursor < u.end)
        .unwrap_or(0)
}
