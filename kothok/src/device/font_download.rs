// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
//! On-demand font download for scripts whose face is not on the device.
//!
//! Triggered at book-open when `ensure_font_for_script` reports a missing
//! font and WiFi is connected. Downloads the Noto Sans face for that script
//! from the notofonts GitHub repos, saves it to FONTS_DIR, and installs it
//! into the font cache. Runs in a background thread; the receiver yields the
//! script on success so the caller can re-render.

use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

use kobo_core::device::paths::FONTS_DIR;
use kobo_core::rendering::text_render::{install_font, Script};

use crate::device::fonts::font_filename_for_script;

const DOWNLOAD_TIMEOUT_SECS: u64 = 120;

fn font_url(script: Script) -> Option<String> {
    const NOTO: &str =
        "https://raw.githubusercontent.com/notofonts/notofonts.github.io/main/fonts";
    const CJK: &str = "https://github.com/notofonts/noto-cjk/raw/main/Sans/SubsetOTF";

    let family = match script {
        Script::Bengali => "NotoSansBengali",
        Script::Devanagari => "NotoSansDevanagari",
        Script::Arabic => "NotoSansArabic",
        Script::Hebrew => "NotoSansHebrew",
        Script::Georgian => "NotoSansGeorgian",
        Script::Armenian => "NotoSansArmenian",
        Script::Ethiopic => "NotoSansEthiopic",
        Script::Gujarati => "NotoSansGujarati",
        Script::Gurmukhi => "NotoSansGurmukhi",
        Script::Tamil => "NotoSansTamil",
        Script::Telugu => "NotoSansTelugu",
        Script::Kannada => "NotoSansKannada",
        Script::Malayalam => "NotoSansMalayalam",
        Script::Sinhala => "NotoSansSinhala",
        Script::Thai => "NotoSansThai",
        Script::Lao => "NotoSansLao",
        Script::Khmer => "NotoSansKhmer",
        Script::Myanmar => "NotoSansMyanmar",
        Script::Japanese => return Some(format!("{CJK}/JP/NotoSansJP-Regular.otf")),
        Script::Korean => return Some(format!("{CJK}/KR/NotoSansKR-Regular.otf")),
        Script::Chinese => return Some(format!("{CJK}/SC/NotoSansSC-Regular.otf")),
        Script::Latin | Script::Greek | Script::Cyrillic | Script::Other => return None,
    };
    Some(format!("{NOTO}/{family}/hinted/ttf/{family}-Regular.ttf"))
}

pub struct FontDownloadResult {
    pub script: Script,
    pub ok: bool,
}

/// Spawn a background download for the missing font. The receiver yields once
/// when the download finishes (success or failure). On success the font is
/// already saved to FONTS_DIR and installed in the cache.
pub fn spawn(script: Script) -> Receiver<FontDownloadResult> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let result = download(script);
        let _ = tx.send(result);
    });
    rx
}

fn download(script: Script) -> FontDownloadResult {
    let Some(filename) = font_filename_for_script(script) else {
        log::warn!("font-dl: no filename for {:?}", script);
        return FontDownloadResult { script, ok: false };
    };
    let Some(url) = font_url(script) else {
        log::warn!("font-dl: no URL for {:?} (embedded font)", script);
        return FontDownloadResult { script, ok: false };
    };

    log::info!("font-dl: fetching {filename} from {url}");

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout_read(Duration::from_secs(DOWNLOAD_TIMEOUT_SECS))
        .build();

    let response = match agent.get(&url).call() {
        Ok(r) => r,
        Err(e) => {
            log::warn!("font-dl: {filename} request failed: {e}");
            return FontDownloadResult { script, ok: false };
        }
    };

    let mut data = Vec::new();
    if let Err(e) = response.into_reader().take(20 * 1024 * 1024).read_to_end(&mut data) {
        log::warn!("font-dl: {filename} read failed: {e}");
        return FontDownloadResult { script, ok: false };
    }

    if data.len() < 1024 {
        log::warn!("font-dl: {filename} suspiciously small ({} bytes), rejecting", data.len());
        return FontDownloadResult { script, ok: false };
    }

    let path = PathBuf::from(FONTS_DIR).join(filename);
    if let Err(e) = fs::create_dir_all(FONTS_DIR) {
        log::warn!("font-dl: cannot create {}: {e}", FONTS_DIR);
        return FontDownloadResult { script, ok: false };
    }
    if let Err(e) = fs::write(&path, &data) {
        log::warn!("font-dl: cannot write {}: {e}", path.display());
        return FontDownloadResult { script, ok: false };
    }

    log::info!(
        "font-dl: saved {filename} ({} KB), installing",
        data.len() / 1024
    );
    if install_font(script, data) {
        log::info!("font-dl: {filename} installed");
        FontDownloadResult { script, ok: true }
    } else {
        log::warn!("font-dl: {filename} saved but install_font rejected it");
        FontDownloadResult { script, ok: false }
    }
}
