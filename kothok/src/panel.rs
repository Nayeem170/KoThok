pub mod callbacks;

pub use callbacks::process_panel_callbacks;

pub use kothok_edge_tts::{
    normalize_lang, set_dynamic_voices, spawn_voice_fetch, voice_label, voices_for_lang,
    DEFAULT_VOICE_BN, DEFAULT_VOICE_EN,
};

const FONT_DEBOUNCE_MS: u64 = 600;
const VOICE_CACHE_FILE: &str = "/mnt/onboard/.adds/kothok/voices.json";

pub fn load_voice_cache() -> Vec<kothok_edge_tts::VoiceInfo> {
    kothok_edge_tts::load_voice_cache(VOICE_CACHE_FILE)
}

pub fn save_voice_cache(voices: &[kothok_edge_tts::VoiceInfo]) {
    kothok_edge_tts::save_voice_cache(VOICE_CACHE_FILE, voices);
}
