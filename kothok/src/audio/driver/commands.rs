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
                    p.pause_clock();
                }
                send_event(&self.evt_tx, Event::Paused);
            }
            Cmd::Stop => self.handle_stop(),
            Cmd::Reload(new_utts) => self.handle_reload(new_utts),
            Cmd::Append(new_utts) => {
                self.utterances.extend(new_utts);
            }
            Cmd::Seek(target) => self.handle_seek(target),
            Cmd::Rate(r) => self.handle_voice_param_change(&r, VoiceParam::Rate),
            Cmd::Voice(v) => self.handle_voice_param_change(&v, VoiceParam::Voice),
            Cmd::BnVoice(v) => self.handle_voice_param_change(&v, VoiceParam::BnVoice),
            Cmd::Volume(val) => {
                self.volume = val as f32 / 100.0;
            }
        }
    }

    fn handle_stop(&mut self) {
        self.want_play = false;
        self.idx = 0;
        self.current_idx = 0;
        self.abort_pending();
        self.reset_pipeline();
        drop(self.player.take());
        send_event(&self.evt_tx, Event::Stopped);
    }

    fn handle_reload(&mut self, new_utts: Vec<Utterance>) {
        self.abort_pending();
        self.reset_pipeline();
        self.idx = 0;
        self.current_idx = 0;
        self.utterances = new_utts;
    }

    fn handle_seek(&mut self, target: usize) {
        self.abort_pending();
        self.reset_pipeline();
        self.idx = target.min(self.utterances.len());
        self.current_idx = self.idx;
    }

    fn handle_voice_param_change(&mut self, new_val: &str, param: VoiceParam) {
        match param {
            VoiceParam::Rate => {
                self.rate = new_val.to_string();
            }
            VoiceParam::Voice => {
                self.voice = new_val.to_string();
            }
            VoiceParam::BnVoice => {
                self.bn_voice = new_val.to_string();
            }
        }
        let resume_at = self.current_idx;
        self.abort_pending();
        self.reset_pipeline();
        self.idx = resume_at;
    }
}
