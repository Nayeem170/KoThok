// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;
use log::info;

impl DriverState {
    pub(super) async fn try_open_sink(&mut self) {
        if self.player.is_some() {
            if let Some(p) = self.player.as_mut() {
                p.resume_clock();
            }
            return;
        }
        info!("audio: opening A2DP sink...");
        for attempt in 1..=A2DP_OPEN_MAX_RETRIES {
            match Player::open().await {
                Ok(mut p) => {
                    let lead_in = vec![0i16; LEAD_IN_FRAMES * 2];
                    // best-effort: a rejected lead-in just means the sink starts mid-stream
                    let _ = p.write_chunk(&lead_in).await;
                    self.player = Some(p);
                    send_event(&self.evt_tx, Event::Playing);
                    info!("audio: A2DP sink opened (attempt {attempt})");
                    return;
                }
                Err(e) => {
                    info!("audio: A2DP open attempt {attempt}/{A2DP_OPEN_MAX_RETRIES} failed: {e}");
                    if attempt < A2DP_OPEN_MAX_RETRIES {
                        self.drain_commands_during_backoff().await;
                        if !self.want_play {
                            return;
                        }
                        tokio::time::sleep(Duration::from_millis(A2DP_OPEN_RETRY_BACKOFF_MS)).await;
                    }
                }
            }
        }
        if self.want_play {
            self.want_play = false;
            info!("audio: A2DP sink FAILED after {A2DP_OPEN_MAX_RETRIES} retries");
            send_event(
                &self.evt_tx,
                Event::Error(format!(
                    "A2DP open failed after {A2DP_OPEN_MAX_RETRIES} retries"
                )),
            );
        }
    }

    async fn drain_commands_during_backoff(&mut self) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            match cmd {
                Cmd::Pause | Cmd::Stop => {
                    self.want_play = false;
                    return;
                }
                Cmd::Reload(u) => {
                    self.abort_pending();
                    self.reset_pipeline();
                    self.idx = 0;
                    self.utterances = u;
                }
                Cmd::Seek(t) => {
                    self.abort_pending();
                    self.reset_pipeline();
                    self.idx = t;
                }
                _ => {}
            }
        }
    }

    pub(super) fn try_prefetch(&mut self) {
        if self.pending.is_some()
            || self.ready_queue.len() + (self.current.is_some() as usize) >= 4
            || self.idx >= self.utterances.len()
        {
            return;
        }
        let text = self.utterances[self.idx].text.clone();
        let rate = self.rate.clone();
        let (utt_voice, utt_lang) =
            voice_for_text_explicit(&text, &self.voice, &self.bn_voice, crate::meta::LANG_AUTO);

        if let Some(cached) = self.cache.lookup(&text, &utt_voice, &rate) {
            let start = self.utterances[self.idx].start;
            let end = self.utterances[self.idx].end;
            let para_end = self.utterances[self.idx].para_end;
            let page_break = self.utterances.get(self.idx).and_then(|u| u.page_break);
            info!(
                "audio: PCM cache hit #{} ({} ready)",
                self.idx,
                self.ready_queue.len() + 1,
            );
            self.ready_queue.push(ReadyUtt {
                idx: self.idx,
                prep: cached.prep,
                start,
                end,
                para_end,
                page_break,
            });
            self.idx += 1;
            return;
        }

        info!(
            "audio: synth #{} voice={utt_voice} lang={utt_lang}",
            self.idx
        );
        let synth_idx = self.idx;
        self.pending = Some(tokio::task::spawn_local(synth_prepare(
            synth_idx,
            text.clone(),
            utt_voice,
            rate.clone(),
            crate::meta::LANG_AUTO.into(),
        )));
        self.pending_range = Some((
            self.utterances[synth_idx].start,
            self.utterances[synth_idx].end,
            self.utterances[synth_idx].para_end,
        ));
        self.pending_page_break = self.utterances.get(self.idx).and_then(|u| u.page_break);
        self.pending_idx = Some(self.idx);
        self.pending_started = Some(Instant::now());
        self.idx += 1;
    }

    /// Watchdog for a hung synth. Edge-TTS runs over a websocket; a flaky link
    /// can leave the request neither completing nor erroring, so `pending` would
    /// sit forever and playback silently freezes (no `Ended`, no `Error`). If a
    /// synth outruns the deadline, abort and retry the SAME utterance a few
    /// times; give up with an `Error` once retries are exhausted so the UI can
    /// reflect the stop instead of showing a frozen "playing".
    pub(super) fn check_synth_timeout(&mut self) {
        let hung = matches!(
            self.pending_started,
            Some(t) if t.elapsed().as_secs() >= SYNTH_TIMEOUT_SECS
        );
        if self.pending.is_none() || !hung {
            return;
        }
        self.abort_pending();
        self.fail_current_synth("synth timeout".into());
    }

    /// The synth for the current utterance produced no audio (network error or
    /// hang). Retry the SAME utterance a few times before giving up. Crucial:
    /// `try_prefetch` already advanced `idx` past this utterance, so without
    /// rewinding, a failure would silently drop it -- and a run of failures on a
    /// flaky link drops several in a row, which is heard as the reader "skipping
    /// a long portion" and resuming from a separate block. The caller must have
    /// cleared `self.pending` already.
    fn fail_current_synth(&mut self, reason: String) {
        let idx = self
            .pending_idx
            .take()
            .unwrap_or(self.idx.saturating_sub(1));
        self.pending_range = None;
        self.pending_page_break = None;
        // Rewind to the failed utterance in BOTH cases: `try_prefetch` already
        // stepped `idx` past it, so leaving it there drops the utterance. On a
        // retry we re-synth it now; on give-up we stay anchored to it and pause,
        // so pressing Play retries the same spot instead of skipping ahead. This
        // is the guarantee that a synth failure never silently skips content.
        self.idx = idx;
        // Name the offending sentence in the log. Without it a "stops at the
        // same place" report cannot be traced to a specific utterance, and the
        // text is exactly what is needed to reproduce the synth off-device.
        let snippet: String = self
            .utterances
            .get(idx)
            .map(|u| u.text.chars().take(90).collect())
            .unwrap_or_else(|| "<out of range>".into());
        let chars = self
            .utterances
            .get(idx)
            .map_or(0, |u| u.text.chars().count());
        if self.synth_retries < MAX_SYNTH_RETRIES {
            self.synth_retries += 1;
            info!(
                "audio: synth #{idx} failed ({reason}), retry {}/{MAX_SYNTH_RETRIES} | {chars} chars | {snippet:?}",
                self.synth_retries
            );
        } else {
            self.synth_retries = 0;
            self.want_play = false;
            info!(
                "audio: synth #{idx} failed ({reason}), paused at utt (retries on resume) | {chars} chars | {snippet:?}"
            );
            send_event(&self.evt_tx, Event::Error(reason));
        }
    }

    pub(super) async fn try_collect(&mut self) {
        let collect = if let Some(handle) = &self.pending {
            handle.is_finished()
        } else {
            false
        };
        if !collect {
            return;
        }
        let Some(h) = self.pending.take() else {
            return;
        };
        self.pending_started = None;
        match h.await {
            Ok(Ok(prep)) => {
                self.synth_retries = 0;
                if let Some((s, e, para_end)) = self.pending_range.take() {
                    let synth_idx = self.pending_idx.unwrap_or(self.idx.saturating_sub(1));
                    let ru = ReadyUtt {
                        idx: synth_idx,
                        prep,
                        start: s,
                        end: e,
                        para_end,
                        page_break: self.pending_page_break.take(),
                    };
                    let cache_key = (
                        self.utterances[synth_idx].text.clone(),
                        voice_for_text_explicit(
                            &self.utterances[synth_idx].text,
                            &self.voice,
                            &self.bn_voice,
                            crate::meta::LANG_AUTO,
                        ).0,
                        self.rate.clone(),
                    );
                    self.cache.store(cache_key, ReadyUtt {
                        idx: ru.idx,
                        prep: ru.prep.clone(),
                        start: ru.start,
                        end: ru.end,
                        para_end: ru.para_end,
                        page_break: ru.page_break,
                    });
                    self.ready_queue.push(ru);
                    info!(
                        "audio: TTS synth OK (utt #{synth_idx}, {} ready)",
                        self.ready_queue.len()
                    );
                }
            }
            Ok(Err(e)) => {
                info!("audio: TTS synth FAILED: {e}");
                self.fail_current_synth(e);
            }
            Err(e) => {
                info!("audio: TTS synth task error: {e}");
                self.fail_current_synth(format!("synth task: {e}"));
            }
        }
    }

    pub(super) fn try_advance(&mut self) {
        if self.current.is_some() {
            return;
        }
        if !self.ready_queue.is_empty() {
            let u = self.ready_queue.remove(0);
            self.current_idx = u.idx;
            let page_break_ticks = u
                .page_break
                .and_then(|break_off| u.prep.tick_at_offset(break_off));
            send_event(
                &self.evt_tx,
                Event::Sentence {
                    start: u.start,
                    end: u.end,
                },
            );
            self.current = Some(Utt {
                prep: u.prep,
                pos: 0,
                para_end: u.para_end,
                page_break_ticks,
                page_break_fired: false,
            });
        } else if self.pending.is_none() && self.idx >= self.utterances.len() {
            if let Some(p) = &self.player {
                if p.socket_buffered_frames() > 0 {
                    return;
                }
            }
            self.want_play = false;
            send_event(&self.evt_tx, Event::Ended);
        }
    }

    pub(super) async fn write_audio(&mut self) {
        let utt_done = matches!(&self.current, Some(u) if u.pos >= u.prep.stereo.len());
        if utt_done {
            let has_more = !self.ready_queue.is_empty()
                || self.pending.is_some()
                || self.idx < self.utterances.len();
            if has_more && self.current.as_ref().is_none_or(|u| u.para_end) {
                if let Some(p) = self.player.as_mut() {
                    let extra = PARA_GAP_FRAMES.saturating_sub(SENTENCE_GAP_FRAMES);
                    let gap = vec![0i16; extra * 2];
                    // best-effort: a dropped inter-sentence gap only tightens cadence
                    let _ = p.write_chunk(&gap).await;
                }
            }
            self.current = None;
            return;
        }
        let (Some(p), Some(utt)) = (self.player.as_mut(), self.current.as_mut()) else {
            return;
        };

        if let (Some(break_ticks), false) = (utt.page_break_ticks, utt.page_break_fired) {
            let elapsed_ticks = (utt.pos as f64 / (kobo_core::audio::TARGET_RATE as f64 * 2.0)
                * 10_000_000.0) as u64;
            if elapsed_ticks >= break_ticks {
                utt.page_break_fired = true;
                send_event(&self.evt_tx, Event::PageBreak);
            }
        }

        if p.socket_buffered_frames() > LEAD_FRAMES {
            tokio::time::sleep(Duration::from_millis(BUSY_SPIN_SLEEP_MS)).await;
            return;
        }
        let end = (utt.pos + self.chunk_samples).min(utt.prep.stereo.len());
        let chunk = utt.prep.stereo.get(utt.pos..end).unwrap_or(&[]);
        let write_result = if self.volume < 0.999 {
            self.scale_buf.clear();
            self.scale_buf
                .extend(chunk.iter().map(|&s| (s as f32 * self.volume) as i16));
            p.write_chunk(&self.scale_buf).await
        } else {
            p.write_chunk(chunk).await
        };
        match write_result {
            Ok(()) => {
                if utt.pos == 0 {
                    info!("audio: first PCM write OK ({} samples)", chunk.len() / 2);
                }
                utt.pos = end;
            }
            Err(e) => {
                info!("audio: A2DP write error, reopening: {e}");
                self.player = None;
                if let Some(u) = self.current.as_mut() {
                    u.pos = 0;
                }
            }
        }
    }

    pub(super) async fn idle(&mut self) {
        let mut drop_sink = false;
        if let Some(p) = self.player.as_mut() {
            if p.paused_since()
                .is_none_or(|d| d.as_secs() < SINK_IDLE_KEEPALIVE_SECS)
            {
                let t0 = Instant::now();
                if let Err(e) = p.keepalive().await {
                    debug!("keepalive err, dropping sink: {e}");
                    drop_sink = true;
                } else {
                    let elapsed = t0.elapsed();
                    if elapsed < super::super::types::KEEPALIVE_PACE {
                        tokio::time::sleep(super::super::types::KEEPALIVE_PACE - elapsed).await;
                    }
                }
            } else {
                drop_sink = true;
                debug!("pwr: sink dropping (paused > {SINK_IDLE_KEEPALIVE_SECS}s)");
            }
        }
        if drop_sink {
            self.player = None;
            if let Some(u) = self.current.as_mut() {
                u.pos = 0;
            }
        }
        tokio::time::sleep(Duration::from_millis(IDLE_SLEEP_MS)).await;
    }
}
