// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::synth::{synth_prepare, voice_for_text_explicit};
use super::types::{Cmd, Event, Utt, Utterance, PARA_GAP_FRAMES, SENTENCE_GAP_FRAMES};
use kobo_core::audio::{Player, CHUNK_FRAMES, LEAD_FRAMES, LEAD_IN_FRAMES};
use log::{debug, warn};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::task::JoinHandle;

mod cache;
mod commands;
mod pipeline;

const A2DP_OPEN_MAX_RETRIES: u32 = 5;
const A2DP_OPEN_RETRY_BACKOFF_MS: u64 = 500;
const SINK_IDLE_KEEPALIVE_SECS: u64 = 3;
const BUSY_SPIN_SLEEP_MS: u64 = 10;
const IDLE_SLEEP_MS: u64 = 50;
/// A synth (Edge-TTS over a websocket) that neither completes nor errors leaves
/// `pending` set forever, and the pipeline silently freezes. This watchdog is a
/// LAST-RESORT net for a task that never returns at all.
///
/// It must stay well above `synthesize_prepared`'s own budget, which already
/// times out and retries internally: MAX_ATTEMPTS(3) x up to
/// SYNTH_TIMEOUT_MAX_SECS(30) plus backoff, so ~91s worst case. Setting this
/// below that budget aborts the inner retries mid-flight -- which made a long
/// sentence fail identically on every run and skip the rest of the book.
const SYNTH_TIMEOUT_SECS: u64 = 120;
const MAX_SYNTH_RETRIES: u32 = 2;

pub struct DriverConfig {
    pub voice: String,
    pub bn_voice: String,
    pub rate: String,
    pub volume: u32,
}

enum VoiceParam {
    Rate,
    Voice,
    BnVoice,
}

struct ReadyUtt {
    idx: usize,
    prep: kobo_core::audio::Prepared,
    start: usize,
    end: usize,
    para_end: bool,
    page_break: Option<usize>,
}

fn send_event(tx: &mpsc::Sender<Event>, evt: Event) {
    // best-effort: the main loop may have exited and dropped the receiver;
    // a failed send is non-fatal - the worker continues its state machine.
    let _ = tx.send(evt);
}

struct DriverState {
    cmd_rx: mpsc::Receiver<Cmd>,
    evt_tx: mpsc::Sender<Event>,
    utterances: Vec<Utterance>,
    voice: String,
    bn_voice: String,
    rate: String,
    volume: f32,
    player: Option<Player>,
    want_play: bool,
    idx: usize,
    current: Option<Utt>,
    ready_queue: Vec<ReadyUtt>,
    pending: Option<JoinHandle<Result<kobo_core::audio::Prepared, String>>>,
    pending_idx: Option<usize>,
    pending_range: Option<(usize, usize, bool)>,
    pending_page_break: Option<usize>,
    /// When the in-flight synth was spawned, for the hang watchdog.
    pending_started: Option<Instant>,
    /// Consecutive watchdog retries of the current utterance, capped by
    /// `MAX_SYNTH_RETRIES` before the driver gives up with an error.
    synth_retries: u32,
    scale_buf: Vec<i16>,
    chunk_samples: usize,
    cache: cache::PcmCache,
    current_idx: usize,
}

impl DriverState {
    fn new(
        cmd_rx: mpsc::Receiver<Cmd>,
        evt_tx: mpsc::Sender<Event>,
        utterances: Vec<Utterance>,
        config: DriverConfig,
    ) -> Self {
        let chunk_samples = CHUNK_FRAMES * 2;
        DriverState {
            cmd_rx,
            evt_tx,
            volume: config.volume as f32 / 100.0,
            utterances,
            voice: config.voice,
            bn_voice: config.bn_voice,
            rate: config.rate,
            player: None,
            want_play: false,
            idx: 0,
            current: None,
            ready_queue: Vec::new(),
            pending: None,
            pending_idx: None,
            pending_range: None,
            pending_page_break: None,
            pending_started: None,
            synth_retries: 0,
            scale_buf: Vec::with_capacity(chunk_samples),
            chunk_samples,
            cache: cache::PcmCache::new(),
            current_idx: 0,
        }
    }

    fn abort_pending(&mut self) {
        if let Some(h) = self.pending.take() {
            h.abort();
        }
        self.pending_started = None;
    }

    fn reset_pipeline(&mut self) {
        self.pending_idx = None;
        self.current = None;
        self.pending_range = None;
        self.ready_queue.clear();
        self.synth_retries = 0;
    }
}

pub(crate) async fn driver(
    cmd_rx: mpsc::Receiver<Cmd>,
    evt_tx: mpsc::Sender<Event>,
    utterances: Vec<Utterance>,
    config: DriverConfig,
) {
    let mut st = DriverState::new(cmd_rx, evt_tx, utterances, config);
    loop {
        st.drain_commands().await;
        if st.want_play {
            st.try_open_sink().await;
            st.try_prefetch();
            st.try_collect().await;
            st.check_synth_timeout();
            st.try_advance();
            st.write_audio().await;
            if st.current.is_none() && st.ready_queue.is_empty() && st.pending.is_some() {
                if let Some(p) = st.player.as_mut() {
                    // best-effort: keepalive is a no-op if the sink already
                    // closed; a failed keepalive is non-fatal, the next
                    // tick re-opens the sink via try_open_sink.
                    if let Err(e) = p.keepalive().await {
                        warn!("audio keepalive failed: {e}");
                    }
                }
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        } else {
            st.idle().await;
        }
    }
}
