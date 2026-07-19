// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
//! KoThok reader - on-device EPUB render + Read Aloud (Layer 3a).
//! Renders the page to the framebuffer, drives Slint via raw touch evdev, and
//! runs the Edge-TTS -> A2DP `Player` on a worker thread (Play/Pause/Stop).

mod app;
mod audio;
mod book_session;
mod callbacks;
mod capabilities;
mod data;
mod device;
mod gesture;
mod logger;
mod loop_run;
mod loop_state;
mod meta;
mod panel;
mod platform;
mod reader;
mod rendering;
mod setup;

pub use logger::{FileLogger, KLOG};

pub(crate) use data::persistence::Bookmark;
pub(crate) use meta::{
    apply_book_voice, clean_ws, has_bangla, is_rtl, set_book_meta, set_chapter_name, BN_VOICE,
    SAMPLE_CHAPTER,
};

use data::config;
use device::{hw, input, touch};
use rendering::{layout, text_render};

use log::info;
use slint::SharedString;

use audio::Cmd;

use crate::app::teardown;
use crate::audio::glue::page_utterances;
use crate::device::input::{query_abs_max, EVIOCGABS_Y};
use crate::device::wake::spawn_power_monitor;
use crate::loop_state::LoopContext;

slint::include_modules!();

use std::sync::atomic::{AtomicUsize, Ordering};

pub static W: AtomicUsize = AtomicUsize::new(1072);
pub static H: AtomicUsize = AtomicUsize::new(1448);

pub fn w() -> usize {
    W.load(Ordering::Relaxed)
}
pub fn h() -> usize {
    H.load(Ordering::Relaxed)
}
pub fn content_h() -> i32 {
    (h() - layout::PAD_TOP - layout::FOOTER_H as usize) as i32
}

pub const VERSION: &str = "1.0.0";
pub const BUILD_TAG: &str = "L163-fix-audio-panel-close";

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ViewMode {
    Reading,
    Audio,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SystemState {
    Awake,
    Asleep { from_picker: bool },
    Locked,
}

fn main() {
    let mut init = match setup::run() {
        Some(i) => i,
        None => return,
    };

    let (mut touch_dev, touch_fd, touch_cfg) =
        init_touch(&init.input_devs, &init.hw_cfg, init.w, init.h);
    let (power_pressed, exit_flag) = init_power(&init.input_devs);
    let (cmd_tx, evt_rx) = init_audio(
        &init.cfg,
        init.st.current_page,
        &init.st.state,
        init.st.view_mode,
    );

    init.st.last_font_count = text_render::font_install_count();
    let _ = power_pressed.swap(false, Ordering::SeqCst);

    let cb = callbacks::register(&init.reader);

    let mut ctx = LoopContext {
        reader: &init.reader,
        window: &init.window,
        fb: &init.fb,
        cmd_tx: &cmd_tx,
        evt_rx: &evt_rx,
        cb: &cb,
        cfg: &mut init.cfg,
        all_books: &mut init.all_books,
        caps: &init.caps,
        touch_dev: &mut touch_dev,
        touch_fd,
        touch_cfg: &touch_cfg,
        content_h: init.content_h,
        w: init.w,
        h: init.h,
        power_pressed: &power_pressed,
        fl_path: &init.fl_path,
    };

    loop_run::run_loop(&mut init.st, &mut ctx);

    info!(
        "interactive loop done (taps={}, quit={})",
        init.reader.get_taps(),
        cb.quit.get()
    );

    teardown(
        &init.fb,
        &exit_flag,
        &init.input_devs.power_dev,
        init.w,
        init.h,
        &session_summary(&init.reader),
    );
}

/// One line for the closing splash: what you were reading and how far in.
///
/// Falls back to the app name when there is no book open (exiting straight from
/// the library), so the line is never blank or half-formed.
fn session_summary(reader: &Reader) -> String {
    let title = reader.get_book_title();
    let title = title.trim();
    if title.is_empty() {
        return "Read | Listen | Anywhere".into();
    }
    let pct = (reader.get_book_progress() * 100.0)
        .clamp(0.0, 100.0)
        .round() as i32;
    format!("{title} | {pct}%")
}

fn init_touch(
    input_devs: &hw::InputDevices,
    hw_cfg: &hw::DeviceConfig,
    w: usize,
    h: usize,
) -> (std::fs::File, i32, touch::TouchConfig) {
    use std::os::unix::io::AsRawFd;
    let touch_dev = std::fs::OpenOptions::new()
        .read(true)
        .open(&input_devs.touch_dev)
        .unwrap_or_else(|e| panic!("open touch device {}: {}", input_devs.touch_dev, e));
    let touch_fd = touch_dev.as_raw_fd();
    // SAFETY: touch_fd is the valid, owned TOUCH_DEV descriptor (touch_dev alive for the whole
    // loop). F_GETFL/F_SETFL take/return int flags; setting O_NONBLOCK on our own descriptor is
    // sound and only changes this fd's read blocking mode.
    unsafe {
        let flags = libc::fcntl(touch_fd, libc::F_GETFL);
        libc::fcntl(touch_fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    let raw_y_max = query_abs_max(touch_fd, EVIOCGABS_Y);
    let raw_x_max = query_abs_max(touch_fd, input::EVIOCGABS_X);
    let touch_cfg = touch::TouchConfig {
        switch_xy: hw_cfg.touch_switch_xy,
        mirrored_x: hw_cfg.touch_mirrored_x,
        mirrored_y: hw_cfg.touch_mirrored_y,
        raw_x_max,
        raw_y_max,
        screen_w: w as i32,
        screen_h: h as i32,
    };
    info!(
        "touch: raw {}x{} display {}x{} switch={} mirror=({},{})",
        raw_x_max,
        raw_y_max,
        w,
        h,
        hw_cfg.touch_switch_xy,
        hw_cfg.touch_mirrored_x,
        hw_cfg.touch_mirrored_y
    );
    (touch_dev, touch_fd, touch_cfg)
}

fn init_power(
    input_devs: &hw::InputDevices,
) -> (
    std::sync::Arc<std::sync::atomic::AtomicBool>,
    std::sync::Arc<std::sync::atomic::AtomicBool>,
) {
    let power_pressed = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let exit_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    spawn_power_monitor(
        power_pressed.clone(),
        exit_flag.clone(),
        &input_devs.power_dev,
    );
    (power_pressed, exit_flag)
}

fn init_audio(
    cfg: &config::AppConfig,
    current_page: usize,
    state: &layout::ChapterState,
    view_mode: ViewMode,
) -> (
    std::sync::mpsc::Sender<Cmd>,
    std::sync::mpsc::Receiver<audio::Event>,
) {
    let initial_utts = match view_mode {
        ViewMode::Audio => audio::glue::chapter_utterances(state),
        ViewMode::Reading => page_utterances(current_page, state),
    };
    let init_rate = config::rate_string(cfg.tts_rate);
    audio::spawn(
        initial_utts,
        audio::DriverConfig {
            voice: cfg.tts_voice.clone(),
            bn_voice: BN_VOICE.into(),
            rate: init_rate,
            volume: cfg.volume as u32,
        },
    )
}
