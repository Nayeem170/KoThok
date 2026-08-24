// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
//! TTS sleep timer: the per-frame poll (timed fire + touch reset).
//!
//! Arming (on Event::Playing), user-pause freeze, and stop disarm are
//! event-driven and live in app::events. This runs from run_loop right after
//! render_and_present / alongside power::auto_sleep, mirroring that sibling
//! timer. The countdown is mode-independent: identical in reading and audio
//! mode.
use crate::loop_run::LoopContext;
use crate::loop_state::LoopState;
use crate::Reader;

pub fn tts_sleep_timer(st: &mut LoopState, ctx: &LoopContext, had_event: bool) {
    // Keep the runtime mirror in step with the persisted mode. Covers a config
    // reload on wake, where cfg changes without the panel callback running.
    st.tts_sleep_mode = ctx.cfg.tts_sleep_mode;

    if !st.tts_sleep_armed {
        return;
    }

    let now = std::time::Instant::now();

    // R6: a touch restarts a running countdown. The label is the end-time, so
    // this repaints once - a discrete refresh on touch, not a per-second tick.
    // Only while actually playing: a touch while paused must not restart a
    // frozen countdown (the frozen remaining is restored on resume).
    if had_event && ctx.reader.get_playing() {
        if let Some(dur) = st.tts_sleep_mode.duration() {
            st.tts_sleep_deadline = Some(now + dur);
            set_end_time_label(ctx.reader, dur);
        }
    }

    // Fire only while playing. A frozen (paused) countdown holds its remaining
    // for resume; firing it during the pause would disarm a timer the reader
    // expects to resume from. On fire the device is put to sleep (auto-off's
    // activity clock is refreshed every tick while audio plays, so it can never
    // elapse during playback), via the sleep_requested flag the run loop drains
    // into sleep::sleep_from_timer (Awake -> device sleep; Locked -> pause audio
    // only; Asleep -> no-op).
    if let Some(deadline) = st.tts_sleep_deadline {
        if deadline <= now && ctx.reader.get_playing() {
            disarm(st);
            set_sleep_label(ctx.reader, "");
            st.sleep_requested = true;
            log::info!("tts-sleep: timed deadline reached, requesting sleep");
        }
    }
}

/// Arm (or re-arm) the timer for the current mode. Called on `Event::Playing`
/// (fresh arm, or resume from a frozen remaining) and on a panel mode change
/// while already armed. Off disarms and clears the label; a timed mode paints
/// the end-time caption once.
pub fn arm(st: &mut LoopState, reader: &Reader) {
    use crate::data::config::TtsSleepMode;
    match st.tts_sleep_mode {
        TtsSleepMode::Off => {
            disarm(st);
            set_sleep_label(reader, "");
        }
        timed => {
            st.tts_sleep_armed = true;
            let full = timed.duration().unwrap();
            let dur = st.tts_sleep_paused_remaining.take().unwrap_or(full);
            st.tts_sleep_deadline = Some(std::time::Instant::now() + dur);
            set_end_time_label(reader, dur);
        }
    }
}

/// Freeze a running countdown on a user pause: move the remaining time out of
/// the deadline so the per-frame poll cannot fire while paused. Resume (`arm`)
/// restores it. A no-op when not armed.
pub fn freeze(st: &mut LoopState) {
    if let Some(deadline) = st.tts_sleep_deadline.take() {
        st.tts_sleep_paused_remaining =
            Some(deadline.saturating_duration_since(std::time::Instant::now()));
    }
}

/// Clear the armed flag, deadline, and frozen remaining.
pub fn disarm(st: &mut LoopState) {
    st.tts_sleep_armed = false;
    st.tts_sleep_deadline = None;
    st.tts_sleep_paused_remaining = None;
}

/// Set the sleep-timer display text in both forms: the string (the reading
/// footer renders it as Slint Text) and a pre-rendered image (the audio caption
/// - Slint Text does not render in the audio body on device). Empty clears both.
pub fn set_sleep_label(reader: &Reader, text: &str) {
    reader.set_sleep_timer_label(text.into());
    if text.is_empty() {
        reader.set_sleep_caption_img(slint::Image::default());
        reader.set_sleep_caption_armed(false);
        return;
    }
    let max_w = crate::w().saturating_sub(280).max(120);
    let (img, _) = crate::rendering::render::text_image(text, 30.0, max_w, 1);
    reader.set_sleep_caption_img(img);
    reader.set_sleep_caption_armed(true);
}

/// Paint the end-time label once: the locale-correct clock advanced by the
/// countdown duration.
pub fn set_end_time_label(reader: &Reader, dur: std::time::Duration) {
    set_sleep_label(reader, &end_time_string(dur));
}

fn end_time_string(dur: std::time::Duration) -> String {
    let add_min = (dur.as_secs() / 60) as u32;
    match parse_clock(&crate::device::current_clock()) {
        // current_clock() is "H:MM AM" / "H:MM PM" (shells out to `date`).
        Some((h24, m)) => format_end_time(h24, m, add_min),
        // "--:--" only if `date` is missing on the device; the whole clock is
        // broken then, so a duration hint is the honest fallback.
        None => format!("Sleep +{add_min} min"),
    }
}

fn format_end_time(h24: u32, m: u32, add_min: u32) -> String {
    let total = (h24 * 60 + m + add_min) % (24 * 60);
    let nh24 = total / 60;
    let nm = total % 60;
    let (nh12, pm) = match nh24 {
        0 => (12, false),
        12 => (12, true),
        n if n < 12 => (n, false),
        n => (n - 12, true),
    };
    format!("Sleep {}:{nm:02} {}", nh12, if pm { "PM" } else { "AM" })
}

fn parse_clock(s: &str) -> Option<(u32, u32)> {
    let s = s.trim();
    let (time_part, mer) = s.rsplit_once(' ')?;
    let (hs, ms) = time_part.split_once(':')?;
    let h: u32 = hs.parse().ok()?;
    let m: u32 = ms.parse().ok()?;
    let pm = mer.eq_ignore_ascii_case("pm");
    if !pm && !mer.eq_ignore_ascii_case("am") {
        return None;
    }
    let h24 = match (h, pm) {
        (12, false) => 0,
        (12, true) => 12,
        (_, true) => h + 12,
        (_, false) => h,
    };
    Some((h24, m))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(clock: &str, mins: u32) -> String {
        let (h24, m) = parse_clock(clock).unwrap();
        format_end_time(h24, m, mins)
    }

    #[test]
    fn rolls_past_midnight() {
        assert_eq!(fmt("11:50 PM", 30), "Sleep 12:20 AM");
    }

    #[test]
    fn same_evening() {
        assert_eq!(fmt("9:05 PM", 15), "Sleep 9:20 PM");
    }

    #[test]
    fn noon_hour() {
        assert_eq!(fmt("11:45 AM", 20), "Sleep 12:05 PM");
    }

    #[test]
    fn midnight_hour() {
        assert_eq!(fmt("12:05 AM", 30), "Sleep 12:35 AM");
    }

    #[test]
    fn parse_rejects_garbage() {
        assert_eq!(parse_clock("--:--"), None);
        assert_eq!(parse_clock("nope"), None);
    }
}
