// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;
use log::info;

impl DriverState {
    pub(super) async fn drain_commands(&mut self) {
        while let Ok(cmd) = self.cmd_rx.try_recv() {
            self.handle_command(cmd).await;
        }
    }

    async fn handle_command(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Play => {
                if !self.want_play {
                    info!(
                        "audio: Cmd::Play received (voice={}, {} utterances)",
                        self.voice,
                        self.utterances.len()
                    );
                    self.want_play = true;
                    if let Some(p) = self.player.as_mut() {
                        p.resume_clock();
                        send_event(&self.evt_tx, Event::Playing);
                    }
                }
            }
            Cmd::Pause => {
                self.want_play = false;
                if let Some(p) = self.player.as_mut() {
                    let lead = p.lead_frames();
                    debug!(
                        "PAUSE lead={}f (~{:.0}ms)",
                        lead,
                        lead as f64 / (kobo_core::audio::TARGET_RATE as f64 * 1000.0)
                    );
                    p.pause_clock();
                }
                send_event(&self.evt_tx, Event::Paused);
            }
            Cmd::Stop => self.handle_stop(),
            Cmd::Reload(new_utts) => self.handle_reload(new_utts),
            Cmd::Append(new_utts) => {
                self.utterances.extend(new_utts);
                debug!("APPEND utterances (total {})", self.utterances.len());
            }
            Cmd::Seek(target) => self.handle_seek(target).await,
            Cmd::Rate(r) => self.handle_voice_param_change(&r, VoiceParam::Rate),
            Cmd::Voice(v) => self.handle_voice_param_change(&v, VoiceParam::Voice),
            Cmd::BnVoice(v) => self.handle_voice_param_change(&v, VoiceParam::BnVoice),
            Cmd::Volume(val) => {
                self.volume = val as f32 / 100.0;
                debug!("VOLUME changed to {val}%");
            }
        }
    }

    fn handle_stop(&mut self) {
        self.want_play = false;
        self.idx = 0;
        self.abort_pending();
        self.reset_pipeline();
        drop(self.player.take());
        send_event(&self.evt_tx, Event::Stopped);
    }

    fn handle_reload(&mut self, new_utts: Vec<Utterance>) {
        self.abort_pending();
        self.reset_pipeline();
        self.idx = 0;
        self.utterances = new_utts;
        debug!("RELOAD chapter ({} utterances)", self.utterances.len());
    }

    async fn handle_seek(&mut self, target: usize) {
        self.abort_pending();
        self.reset_pipeline();
        self.idx = target.min(self.utterances.len());
        if self.want_play {
            if let Some(p) = self.player.take() {
                // best-effort: drain errors caught by sink-error recovery
                let _ = p.drain_and_stop().await;
            }
        }
        debug!("SEEK to utterance {}/{}", self.idx, self.utterances.len());
    }

    fn handle_voice_param_change(&mut self, new_val: &str, param: VoiceParam) {
        match param {
            VoiceParam::Rate => {
                self.rate = new_val.to_string();
                debug!("RATE changed to {new_val}");
            }
            VoiceParam::Voice => {
                self.voice = new_val.to_string();
                debug!("VOICE changed to {new_val}");
            }
            VoiceParam::BnVoice => {
                self.bn_voice = new_val.to_string();
                debug!("BN_VOICE changed to {new_val}");
            }
        }
        self.abort_pending();
        self.idx = self.pending_idx.take().unwrap_or(self.idx);
        self.pending_range = None;
    }
}
