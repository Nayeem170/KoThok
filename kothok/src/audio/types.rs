use std::time::Duration;

pub use kobo_core::audio::{PARA_GAP_FRAMES, SENTENCE_GAP_FRAMES};

pub const KEEPALIVE_PACE: Duration = Duration::from_millis(50);

#[derive(Debug, Clone)]
pub struct Utterance {
    pub text: String,
    pub start: usize,
    pub end: usize,
    pub para_end: bool,
    pub page_break: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Cmd {
    Play,
    Pause,
    Stop,
    Reload(Vec<Utterance>),
    Append(Vec<Utterance>),
    Seek(usize),
    Rate(String),
    Voice(String),
    BnVoice(String),
    Volume(u32),
}

#[derive(Debug, Clone)]
pub enum Event {
    Playing,
    Paused,
    Stopped,
    Ended,
    Sentence { start: usize, end: usize },
    PageBreak,
    Error(String),
}

pub(crate) struct Utt {
    pub prep: kobo_core::audio::Prepared,
    pub pos: usize,
    pub para_end: bool,
    pub page_break_ticks: Option<u64>,
    pub page_break_fired: bool,
}

#[cfg(test)]
mod tests {
    use kobo_core::audio::TARGET_RATE;

use super::*;

    fn ms_of(frames: usize) -> f32 {
        frames as f32 / TARGET_RATE as f32 * 1000.0
    }

    #[test]
    fn sentence_gap_frames_is_400ms() {
        assert_eq!(SENTENCE_GAP_FRAMES, TARGET_RATE / 1000 * 400);
        let ms = ms_of(SENTENCE_GAP_FRAMES);
        assert!((ms - 400.0).abs() < 3.0, "sentence gap ~400ms, got {ms}");
    }

    #[test]
    fn para_gap_frames_is_700ms() {
        assert_eq!(PARA_GAP_FRAMES, TARGET_RATE / 1000 * 700);
        let ms = ms_of(PARA_GAP_FRAMES);
        assert!((ms - 700.0).abs() < 3.0, "para gap ~700ms, got {ms}");
        assert!(
            PARA_GAP_FRAMES > SENTENCE_GAP_FRAMES,
            "para gap must exceed sentence gap"
        );
    }

    #[test]
    fn cmd_variants_construct_without_panic() {
        let _ = Cmd::Play;
        let _ = Cmd::Pause;
        let _ = Cmd::Stop;
        let _ = Cmd::Reload(vec![]);
        let _ = Cmd::Append(vec![]);
        let _ = Cmd::Seek(42);
        let _ = Cmd::Rate("+10%".into());
        let _ = Cmd::Voice("en-US-EmmaMultilingualNeural".into());
        let _ = Cmd::BnVoice("bn-BD-NabanitaNeural".into());
        let _ = Cmd::Volume(80);
    }

    #[test]
    fn event_variants_construct_without_panic() {
        let _ = Event::Playing;
        let _ = Event::Paused;
        let _ = Event::Stopped;
        let _ = Event::Ended;
        let _ = Event::Sentence { start: 0, end: 10 };
        let _ = Event::PageBreak;
        let _ = Event::Error("boom".into());
    }
}
