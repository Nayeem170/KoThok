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

pub struct WifiEntry {
    pub ssid: String,
    pub id: u32,
    pub connected: bool,
}

pub struct BtEntry {
    pub name: String,
    pub path: String,
    pub connected: bool,
}

pub struct WifiBtListResult {
    pub wifi: Vec<WifiEntry>,
    pub wifi_ids_valid: bool,
    pub bt: Vec<BtEntry>,
}

pub fn spawn_wifi_bt_list_fetch() -> std::sync::mpsc::Receiver<WifiBtListResult> {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        use crate::device::{bt_list_devices, wifi_list_networks, wifi_saved_ssids};

        let networks = wifi_list_networks();
        let saved = wifi_saved_ssids();
        let connected_ssid = networks
            .iter()
            .find(|n| n.connected)
            .map(|n| n.ssid.as_str())
            .unwrap_or("");

        let (wifi, wifi_ids_valid) = if networks.len() >= saved.len() && !networks.is_empty() {
            let v: Vec<WifiEntry> = networks
                .iter()
                .map(|n| WifiEntry { ssid: n.ssid.clone(), id: n.id, connected: n.connected })
                .collect();
            (v, true)
        } else if !saved.is_empty() {
            let v: Vec<WifiEntry> = saved
                .iter()
                .map(|s| {
                    let c = !connected_ssid.is_empty() && s.as_str() == connected_ssid;
                    WifiEntry { ssid: s.clone(), id: 0, connected: c }
                })
                .collect();
            (v, false)
        } else if !networks.is_empty() {
            let v: Vec<WifiEntry> = networks
                .iter()
                .map(|n| WifiEntry { ssid: n.ssid.clone(), id: n.id, connected: n.connected })
                .collect();
            (v, true)
        } else {
            (Vec::new(), true)
        };

        let devices = bt_list_devices();
        let bt: Vec<BtEntry> = devices
            .iter()
            .map(|d| BtEntry { name: d.name.clone(), path: d.path.clone(), connected: d.connected })
            .collect();

        let _ = tx.send(WifiBtListResult {
            wifi,
            wifi_ids_valid,
            bt,
        });
    });
    rx
}
