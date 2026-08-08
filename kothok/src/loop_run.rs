// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
// Re-exported to submodules via `use super::*`.
#![allow(unused_imports)]

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use kobo_core::{Capabilities, Chapter};
use slint::platform::software_renderer::{MinimalSoftwareWindow, Rgb565Pixel};
use slint::platform::WindowAdapter;
use slint::SharedString;

use crate::audio::glue::{
    best_effort_send, first_utt_on_page, load_chapter_audio, load_page_audio, page_utterances,
};
use crate::audio::{Cmd, Event};
use crate::book_session;
use crate::callbacks::Callbacks;
use crate::capabilities::KoboCapabilities;
use crate::data::config::{
    load_config_from_base, save_settings_for, AppConfig, BOOK_SETTINGS_FILE, CONFIG_FILE,
};
use crate::data::library::open_book;
use crate::data::library::EpubEntry;
use crate::data::persistence::{
    self, apply_book_settings, load_book_settings, load_position, push_book_settings_to_ui,
    save_position, ReadingPosition, POSITIONS_FILE,
};
use crate::device::{fonts, input, touch};
use crate::loop_state::{LoopContext, LoopState};
use crate::rendering::fb::{self, Fb, WAVE_GC16, WAVE_GL16};
use crate::rendering::layout::{self, build_state, OffsetComputation, PAD_TOP};
use crate::rendering::render::{
    self, library_max_scroll, picker_scroll_cells, pill_rects, render_book_cover_scaled,
    show_book_picker, snap_scroll, PickerRefresh, BEZEL_DEAD_ZONE, NAV_BAR_H,
    PICKER_NAV_TOUCH_MARGIN,
};
use crate::{
    apply_book_voice, is_rtl, set_book_meta, set_chapter_name, SystemState, ViewMode,
    SAMPLE_CHAPTER,
};

use std::io::Read;
use std::path::PathBuf;

use crate::app::{
    enter_sleep, process_audio_events, process_page_navigation, process_panel_callbacks,
    render_and_present, teardown, toggle_playback, wake_from_sleep, AudioFlags,
};
use crate::gesture;
use crate::rendering::render::{composite_text, overlay_text, refresh_text_cache};
use crate::rendering::text_render;
use log::{debug, error, info, warn};

use crate::device::power::frontlight_get;
use crate::device::wake::poll_touch_for_wake;
use crate::reader::{apply_page, switch_chapter, ChapterSwitchOpts};

mod callbacks;
mod link_nav;
mod picker;
mod power;
mod sleep;
mod status;
pub(crate) use status::save_position_now;
mod search;
mod touch_dispatch;
mod touch_release;
pub(crate) mod tts_sleep;

pub(super) enum LoopFlow {
    Normal,
    Continue,
    Break,
}

const EXIT_CONFIRM_WINDOW_MS: u64 = 3000;
const STATUS_REFRESH_MS: u64 = 3000;
const WIFI_TOGGLE_GRACE_MS: u64 = 30000;
const LOCK_SLEEP_SECS: u64 = 1800;
const SWIPE_THRESHOLD_PX: f32 = 60.0;
const SWIPE_DELTA_TOLERANCE_PX: i32 = 50;
const PBAR_H: f32 = 70.0;
/// Audio-mode header and footer band heights, matching `audio_player.slint`.
/// Taps between them land on the disk and are free for the double-tap gesture.
const AUDIO_HEADER_H: f32 = 110.0;
const AUDIO_FOOTER_H: f32 = 371.0;
const TAP_COOLDOWN_MS: u64 = 100;
const SLEEP_PANEL_SETTLE_MS: u64 = 400;
const PICKER_ENTER_DEBOUNCE_MS: u64 = 350;
const PICKER_DOUBLE_TAP_MS: u64 = 450;
/// Window for a second footer play-button tap to count as a double-click
/// (bookmark) instead of a single play/pause toggle.
const PLAY_BUTTON_DOUBLE_MS: u64 = 350;
/// How long a tapped-open header stays up before retracting again, when
/// auto-hide is on. Long enough to read the page number and reach a button,
/// short enough that the page gets the screen back without another tap.
pub const HEADER_REVEAL_SECS: u64 = 10;

pub fn run_loop(st: &mut LoopState, ctx: &mut LoopContext) {
    let mut iter = 0u32;
    loop {
        iter += 1;
        if iter <= 3 {
            info!("loop iter {iter}");
        }
        power::check_font_repaginate(st, ctx);

        match power::handle_power_button(st, ctx) {
            LoopFlow::Continue => continue,
            LoopFlow::Break => {
                info!("EXIT: power_button (iter {iter})");
                break;
            }
            LoopFlow::Normal => {}
        }

        match power::poll_asleep_wake(st, ctx) {
            LoopFlow::Continue => continue,
            LoopFlow::Break => {
                info!("EXIT: asleep_wake (iter {iter})");
                break;
            }
            LoopFlow::Normal => {}
        }

        status::sync_panel_close(st, ctx, "panel: CLOSED (cross button)");

        match status::handle_exit_button(st, ctx) {
            LoopFlow::Continue => continue,
            LoopFlow::Break => {
                info!("EXIT: exit_button (iter {iter})");
                break;
            }
            LoopFlow::Normal => {}
        }

        match status::handle_quit_button(st, ctx) {
            LoopFlow::Continue => continue,
            LoopFlow::Break => {
                info!("EXIT: quit_button (iter {iter})");
                break;
            }
            LoopFlow::Normal => {}
        }

        let had_event = touch_dispatch::poll_and_dispatch_touch(st, ctx);

        if had_event {
            st.last_activity = std::time::Instant::now();
        }

        status::sync_panel_close(st, ctx, "panel: CLOSED (immediate cell check)");
        status::refresh_status(st, ctx);
        status::poll_offset_rx(st, ctx);
        status::poll_voice_rx(st);

        match picker::handle_picker(st, ctx) {
            LoopFlow::Continue => continue,
            LoopFlow::Break => {
                info!("EXIT: picker (iter {iter})");
                break;
            }
            LoopFlow::Normal => {}
        }

        let (ui_changed, page_changed) = callbacks::process_loop_callbacks(st, ctx);

        // After the callbacks, so a page turn or a TTS sentence advance is
        // already reflected in the cursor this reads.
        status::autosave_position(st, ctx);

        render_and_present(st, ctx, had_event, ui_changed, page_changed);

        // Per-frame TTS sleep-timer poll (timed fire + touch reset). Runs after
        // the present and alongside power::auto_sleep, mirroring that sibling
        // timer. Event-driven arming/freeze/disarm live in app::events.
        tts_sleep::tts_sleep_timer(st, ctx, had_event);

        // Drain a sleep request raised by the TTS sleep timer (timed deadline
        // or end-of-chapter). Auto-off's activity clock is refreshed every tick
        // while audio plays, so it can never elapse during playback - the timer
        // owns the bedtime device-sleep itself. sleep_from_timer is state-aware
        // (Awake -> full sleep; Locked -> pause audio only; Asleep -> no-op).
        if st.sleep_requested {
            st.sleep_requested = false;
            sleep::sleep_from_timer(st, ctx);
            continue;
        }

        match power::auto_sleep(st, ctx) {
            LoopFlow::Continue => continue,
            LoopFlow::Break => {
                info!("EXIT: auto_sleep (iter {iter})");
                break;
            }
            LoopFlow::Normal => {}
        }
    }
}
