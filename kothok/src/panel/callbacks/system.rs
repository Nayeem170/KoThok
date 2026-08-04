// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use crate::callbacks::Callbacks;
use crate::data::config::{load_config_from_base, AppConfig, BOOK_SETTINGS_FILE, CONFIG_FILE};
use crate::data::persistence::{clear_book_settings, push_book_settings_to_ui};
use crate::loop_state::LoopState;
use crate::Reader;

/// Toggle the reading header between always-shown and auto-hiding.
///
/// Turning auto-hide off reveals the header immediately rather than waiting for
/// the next tap -- the setting the reader just chose is "always shown", so the
/// panel closing onto a still-hidden header would read as the switch not
/// working.
pub(super) fn handle_header_toggle(
    st: &mut LoopState,
    reader: &Reader,
    cfg: &mut AppConfig,
    cell: &Rc<Cell<bool>>,
) -> bool {
    if !cell.replace(false) {
        return false;
    }
    cfg.auto_hide_header = !cfg.auto_hide_header;
    reader.set_auto_hide_header(cfg.auto_hide_header);
    st.header_visible = !cfg.auto_hide_header;
    reader.set_header_visible(st.header_visible);
    st.header_revealed_at = None;
    log::info!("panel: auto-hide header = {}", cfg.auto_hide_header);
    true
}

/// Start a reader-initiated update check, and drain one that is already
/// running.
///
/// Both halves live here so the panel gets a status line without blocking the
/// loop: the request can take the full 15s read timeout, and the reader is
/// looking at a screen that must keep answering touches meanwhile.
pub(super) fn handle_update_check(
    st: &mut LoopState,
    reader: &Reader,
    cell: &Rc<Cell<bool>>,
) -> bool {
    let mut changed = false;
    if cell.replace(false) && st.update_check_rx.is_none() {
        reader.set_update_status("Checking...".into());
        st.update_check_rx = Some(crate::update_check::spawn_manual_check());
        changed = true;
    }
    if let Some(rx) = st.update_check_rx.take() {
        match rx.try_recv() {
            Ok(result) => {
                log::info!("panel: update check -> {}", result.label());
                reader.set_update_status(result.label().into());
                changed = true;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => st.update_check_rx = Some(rx),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                reader.set_update_status("Check failed - try again".into());
                changed = true;
            }
        }
    }
    changed
}

/// Drop this book's saved overrides and put the global defaults back.
///
/// The defaults are re-read from the config file rather than taken from the
/// in-memory `cfg`: that one already has this book's overrides applied over the
/// top, so resetting against it would restore the very values being discarded.
///
/// Reflow goes through the font slider's debounce cells instead of running
/// inline, so a reset repaginates on exactly the path a font change does --
/// including the audio driver reload that has to follow every `build_state`.
pub(super) fn handle_reset_book(
    st: &mut LoopState,
    reader: &Reader,
    cfg: &mut AppConfig,
    cb: &Callbacks,
) -> bool {
    if !cb.reset_book_cell.replace(false) {
        return false;
    }
    if st.picker_active || st.current_book_path.is_empty() {
        return false;
    }
    clear_book_settings(
        std::path::Path::new(BOOK_SETTINGS_FILE),
        &st.current_book_path,
    );
    let device_default_font = (crate::w() as i32 / 38).clamp(20, 60);
    let globals = load_config_from_base(CONFIG_FILE, device_default_font);
    cfg.font_size = globals.font_size;
    cfg.tts_rate = globals.tts_rate;
    cfg.tts_voice = globals.tts_voice;
    cfg.line_spacing_pct = globals.line_spacing_pct;
    cfg.text_justify = globals.text_justify;
    cfg.margin_px = globals.margin_px;
    crate::rendering::layout::set_margin_px(cfg.margin_px);
    st.line_spacing_pct = cfg.line_spacing_pct;
    st.text_justify = cfg.text_justify;
    push_book_settings_to_ui(reader, cfg);
    cb.font_pending_val.set(Some(cfg.font_size));
    cb.font_last_change.set(Some(Instant::now()));
    reader.set_status("Book settings reset".into());
    log::info!("panel: reset book settings for {}", st.current_book_path);
    true
}
