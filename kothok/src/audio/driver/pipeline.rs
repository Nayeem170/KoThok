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
            || self.ready_queue.len() + (self.current.is_some() as usize) >= 2
            || self.idx >= self.utterances.len()
        {
            return;
        }
        let (text, s, e, para_end) = (
            self.utterances[self.idx].text.clone(),
            self.utterances[self.idx].start,
            self.utterances[self.idx].end,
            self.utterances[self.idx].para_end,
        );
        let rate = self.rate.clone();
        let (utt_voice, utt_lang) =
            voice_for_text_explicit(&text, &self.voice, &self.bn_voice, crate::meta::LANG_AUTO);
        info!("audio: synth #{} voice={utt_voice} lang={utt_lang}", self.idx);
        let synth_idx = self.idx;
        self.pending = Some(tokio::task::spawn_local(synth_prepare(
            synth_idx,
            text,
            utt_voice,
            rate,
            crate::meta::LANG_AUTO.into(),
        )));
        self.pending_range = Some((s, e, para_end));
        self.pending_page_break = self.utterances.get(self.idx)
            .and_then(|u| u.page_break);
        self.pending_idx = Some(self.idx);
        self.idx += 1;
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
        match h.await {
            Ok(Ok(prep)) => {
                if let Some((s, e, para_end)) = self.pending_range.take() {
                    let synth_idx = self.pending_idx.unwrap_or(self.idx.saturating_sub(1));
                    self.ready_queue.push(ReadyUtt {
                        prep,
                        start: s,
                        end: e,
                        para_end,
                        page_break: self.pending_page_break.take(),
                    });
                    info!("audio: TTS synth OK (utt #{synth_idx}, {} ready)", self.ready_queue.len());
                }
            }
            Ok(Err(e)) => {
                info!("audio: TTS synth FAILED: {e}");
                self.want_play = false;
                send_event(&self.evt_tx, Event::Error(e));
            }
            Err(e) => {
                info!("audio: TTS synth task error: {e}");
                self.want_play = false;
                send_event(&self.evt_tx, Event::Error(format!("synth task: {e}")));
            }
        }
    }

    pub(super) fn try_advance(&mut self) {
        if self.current.is_some() {
            return;
        }
        if !self.ready_queue.is_empty() {
            let u = self.ready_queue.remove(0);
            let page_break_ticks = u.page_break
                .and_then(|break_off| compute_page_break_ticks(&u.prep, break_off));
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
            self.idx = 0;
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
            let elapsed_ticks = (utt.pos as f64 / (kobo_core::audio::TARGET_RATE as f64 * 2.0) * 10_000_000.0) as u64;
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
        tokio::time::sleep(Duration::from_secs(IDLE_SLEEP_SECS)).await;
    }
}

fn compute_page_break_ticks(
    prep: &kobo_core::audio::Prepared,
    break_text_offset: usize,
) -> Option<u64> {
    let mut acc = 0usize;
    for (ticks, word) in &prep.bounds {
        if acc >= break_text_offset {
            return Some(*ticks);
        }
        acc += word.len() + 1;
    }
    prep.bounds.last().map(|(t, _)| *t)
}
