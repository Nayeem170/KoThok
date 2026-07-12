use crate::audio::glue::page_utterances;
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

// Let the sleep-cover waveform finish before dimming the frontlight, so the
// cover isn't captured mid-refresh.
const SLEEP_COVER_SETTLE_MS: u64 = 400;

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
///  - paused/idle -> resume from the cursor if it's on this page, else from the
///    page's first line. Shared by the footer Play/Pause button.
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
        // best-effort: channel may be full
        let _ = cmd_tx.send(Cmd::Pause);
        return PlayToggle {
            ch: current_chapter,
            pg: current_page,
            off: reader.get_cur_start().max(0) as usize,
            end: reader.get_cur_end().max(0) as usize,
        };
    }
    let cur = reader.get_cur_start().max(0) as usize;
    let page_utts = page_utterances(current_page, state);
    let target = if page_utts.iter().any(|u| cur >= u.start && cur < u.end) {
        page_utts
            .iter()
            .position(|u| cur >= u.start && cur < u.end)
            .unwrap_or(0)
    } else {
        let (rs, re) = state.pages.get(current_page).copied().unwrap_or((0, 0));
        if let Some(rows) = state.all_rows.get(rs..re) {
            for row in rows {
                if row.start < row.end {
                    reader.set_cur_start(row.start);
                    reader.set_cur_end(row.end);
                    break;
                }
            }
        }
        0
    };
    reader.set_saved_page((chapter_offsets[current_chapter] + current_page) as i32);
    let cs = reader.get_cur_start();
    let (off, end) = if cs > 0 {
        (cs as usize, reader.get_cur_end().max(0) as usize)
    } else {
        (0, 0)
    };
    let _ = cmd_tx.send(Cmd::Reload(page_utts));
    let _ = cmd_tx.send(Cmd::Seek(target));
    let _ = cmd_tx.send(Cmd::Play);
    reader.set_playing(true);
    reader.set_paused(false);
    PlayToggle {
        ch: current_chapter,
        pg: current_page,
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
/// without a framebuffer or live radios.
pub struct SleepPlan {
    /// `true` = show the book cover; `false` = the KoThok splash (library lock).
    pub show_cover: bool,
    /// Power the frontlight off on sleep.
    pub frontlight_off: bool,
    /// Power wifi off on sleep (only when it was on).
    pub wifi_off: bool,
    /// Power Bluetooth off on sleep (only when it was on). On devices with no
    /// BT adapter the dbus call hangs, so this MUST stay false when BT is off.
    pub bt_off: bool,
}

pub fn sleep_plan(
    from_picker: bool,
    fl_path: &Option<std::path::PathBuf>,
    wifi_on: bool,
    bt_on: bool,
) -> SleepPlan {
    SleepPlan {
        show_cover: !from_picker,
        frontlight_off: fl_path.is_some(),
        wifi_off: wifi_on,
        bt_off: bt_on,
    }
}
