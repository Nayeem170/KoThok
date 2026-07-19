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
use crate::data::config::AppConfig;
use crate::data::library::open_book;
use crate::data::library::EpubEntry;
use crate::data::persistence::{
    self, load_position, save_position, ReadingPosition, POSITIONS_FILE,
};
use crate::device::{fonts, input, touch};
use crate::loop_state::{LoopContext, LoopState};
use crate::rendering::fb::{self, Fb, WAVE_GC16};
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
mod picker;
mod power;
mod status;
mod touch_dispatch;
mod touch_release;

pub(super) enum LoopFlow {
    Normal,
    Continue,
    Break,
}

const EXIT_CONFIRM_WINDOW_MS: u64 = 3000;
const STATUS_REFRESH_MS: u64 = 3000;
const BT_TOGGLE_GRACE_MS: u64 = 15000;
const WIFI_TOGGLE_GRACE_MS: u64 = 30000;
const AUTO_SLEEP_SECS: u64 = 60;
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

        let render_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            render_and_present(st, ctx, had_event, ui_changed, page_changed)
        }));
        if let Err(payload) = render_result {
            let msg = payload
                .downcast_ref::<&str>()
                .copied()
                .or_else(|| payload.downcast_ref::<String>().map(|s| s.as_str()))
                .unwrap_or("<non-string>");
            error!("PANIC caught in render_and_present: {msg}");
            st.text_dirty = true;
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
