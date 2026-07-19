// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
mod book_init;
mod loop_init;

use book_init::{init_book, init_picker, BookInit, PickerInit, ReaderSetup, ScreenCtx};
use loop_init::build_loop_state;
use std::sync::atomic::Ordering;

use kobo_core::{Capabilities, Chapter};
use slint::platform::software_renderer::{MinimalSoftwareWindow, RepaintBufferType, Rgb565Pixel};
use slint::SharedString;

use crate::capabilities::KoboCapabilities;
use crate::data::config::{self, AppConfig};
use crate::data::library::{self, scan_epubs, EpubEntry, BOOK_DIR};
use crate::data::persistence::{last_book_path, POSITIONS_FILE};
use crate::device::{fonts, hw};
use crate::platform::KoboPlatform;
use crate::rendering::common::rgb565_as_bytes_ref;
use crate::rendering::fb::{Fb, WAVE_GC16, WAVE_GL16};
use crate::rendering::layout::{self, init_layout, FOOTER_H, PAD_TOP};
use crate::rendering::render;
use crate::{FileLogger, BUILD_TAG, H, KLOG, W};

use log::info;

pub struct InitResult {
    pub fb: Fb,
    pub w: usize,
    pub h: usize,
    pub content_h: i32,
    pub window: std::rc::Rc<MinimalSoftwareWindow>,
    pub reader: crate::Reader,
    pub cfg: AppConfig,
    pub all_books: Vec<EpubEntry>,
    pub caps: KoboCapabilities,
    pub fl_path: Option<std::path::PathBuf>,
    pub hw_cfg: hw::DeviceConfig,
    pub input_devs: hw::InputDevices,
    pub st: crate::loop_state::LoopState,
}

pub fn run() -> Option<InitResult> {
    init_logger();
    let (hw_cfg, input_devs) = detect_hardware();
    let _wl = crate::device::power::WakeLock::acquire();

    let fb = Fb::open()?;
    let w = fb.xres;
    let h = fb.yres;
    let window = init_platform(w, h);
    let content_h: i32 = (h - PAD_TOP - FOOTER_H as usize) as i32;

    let cli_path = std::env::args().nth(1);
    let (initial_path, all_books) = scan_and_resolve(&fb, w, h, &cli_path);

    let mut setup = init_reader_and_config(w, &hw_cfg);

    let (book, picker) = init_book_or_picker(
        &mut setup,
        &fb,
        &window,
        w,
        h,
        &all_books,
        &cli_path,
        &initial_path,
    );

    let st = build_loop_state(
        book,
        picker,
        setup.body_px,
        setup.head_px,
        setup.line_h,
        w,
        h,
        hw_cfg.model,
    );

    setup
        .reader
        .set_audio_mode(matches!(st.view_mode, crate::ViewMode::Audio));
    setup.reader.set_has_bookmark(st.bookmark.is_some());

    Some(InitResult {
        fb,
        w,
        h,
        content_h,
        window,
        reader: setup.reader,
        cfg: setup.cfg,
        all_books,
        caps: setup.caps,
        fl_path: setup.fl_path,
        hw_cfg,
        input_devs,
        st,
    })
}

/// Ink the next splash word and update the status line.
///
/// Both regions present on `WAVE_GL16`. The words are a **monotone** addition --
/// those pixels go from paper to ink once and never back -- which would suit the
/// faster `WAVE_DU`, except DU is a two-level waveform: it would flatten the
/// brand red and green of "Read" and "Listen" to solid black. Sixteen levels are
/// the price of keeping the colour.
///
/// That is affordable here for the same reason the old spinner was not. Flicker
/// is area times frequency, and this is four small rects over an entire boot,
/// against a 112x112 badge re-driven continuously for as long as loading took.
///
/// The status line needs a clearing waveform regardless, since its text is
/// replaced rather than added to.
fn advance_splash(fb: &Fb, splash: &mut [Rgb565Pixel], w: usize, h: usize, stage: usize) {
    let layout = render::splash_layout(w, h);
    let status = render::OPENING_STATUS
        .get(stage - 1)
        .copied()
        .unwrap_or_default();
    render::paint_splash(splash, stage, status);

    let present = |r: kobo_core::rendering::loader::Rect, wave: u32| {
        let x = r.x.max(0) as usize;
        let y = r.y.max(0) as usize;
        let rw = (r.w.max(0) as usize).min(w.saturating_sub(x));
        let rh = (r.h.max(0) as usize).min(h.saturating_sub(y));
        if rw == 0 || rh == 0 {
            return;
        }
        fb.present_rect(
            rgb565_as_bytes_ref(splash),
            w,
            h,
            &kobo_core::device::fb::UpdateRegion { x, y, w: rw, h: rh },
            wave,
        );
        fb.wait_for_update_complete();
    };

    if let Some(r) = layout.stage_rect(stage) {
        present(r, WAVE_GL16);
    }
    present(layout.status_rect(w), WAVE_GL16);
}

fn scan_and_resolve(
    fb: &Fb,
    w: usize,
    h: usize,
    cli_path: &Option<String>,
) -> (Option<String>, Vec<EpubEntry>) {
    let scan_handle = std::thread::spawn(move || scan_epubs(BOOK_DIR).unwrap_or_default());

    // Ink Bloom: one word per startup milestone. The reveal is driven by real
    // work finishing rather than by a timer, so it can neither stall while the
    // device is still busy nor finish while it is not.
    let mut splash = vec![Rgb565Pixel(0); w * h];
    render::paint_splash(&mut splash, 1, render::OPENING_STATUS[0]);
    fb.present(rgb565_as_bytes_ref(&splash), w, h, true, 0, 0, WAVE_GC16);

    crate::device::init_wpa_detection();

    fonts::log_available_fonts();
    let font_handle = std::thread::spawn(|| {
        fonts::load_cached_fonts();
    });

    fb.wait_for_update_complete();

    font_handle.join().unwrap_or(());
    advance_splash(fb, &mut splash, w, h, 2);

    let all_books = scan_handle.join().unwrap_or_default();
    advance_splash(fb, &mut splash, w, h, 3);

    let initial_path = resolve_initial_book(cli_path, &all_books);
    advance_splash(fb, &mut splash, w, h, 4);
    info!(
        "books: {} scanned, initial={:?}, cli={:?}",
        all_books.len(),
        initial_path,
        cli_path
    );
    (initial_path, all_books)
}

fn init_platform(w: usize, h: usize) -> std::rc::Rc<MinimalSoftwareWindow> {
    let window = MinimalSoftwareWindow::new(RepaintBufferType::SwappedBuffers);
    slint::platform::set_platform(Box::new(KoboPlatform {
        window: window.clone(),
        start: std::time::Instant::now(),
    }))
    .expect("set_platform");
    W.store(w, Ordering::Relaxed);
    H.store(h, Ordering::Relaxed);
    crate::rendering::density::init_ppi(w, h);
    init_layout(w, h);
    window.set_size(slint::PhysicalSize::new(w as u32, h as u32));
    window
}

fn init_reader_and_config(w: usize, hw_cfg: &hw::DeviceConfig) -> ReaderSetup {
    let reader = crate::Reader::new().expect("Reader::new");
    let device_default_font = (w as i32 / 30).clamp(20, 60);
    let cfg = config::load_config_from_base(config::CONFIG_FILE, device_default_font);
    let body_px: f32 = cfg.font_size as f32;
    let head_px: f32 = cfg.font_size as f32 * layout::HEADING_SCALE;
    let line_h: i32 = (cfg.font_size as f32 * layout::LINE_HEIGHT_SCALE) as i32;
    reader.set_tts_lang(SharedString::from(cfg.tts_lang.clone()));
    reader.set_tts_voice(SharedString::from(cfg.tts_voice.clone()));
    reader.set_tts_speed(cfg.tts_rate);
    reader.set_font_size_val(cfg.font_size);
    let caps = KoboCapabilities;
    reader.set_wifi_on(caps.network_available());
    reader.set_bt_on(caps.audio_sink_available());
    reader.set_play_enabled(caps.read_aloud_available());
    if let Some(n) = caps.wifi_name() {
        reader.set_wifi_connected_name(SharedString::from(n));
    }
    if let Some(n) = caps.bt_name() {
        reader.set_bt_connected_name(SharedString::from(n));
    }
    reader.set_clock(SharedString::from(caps.current_clock()));
    let dummy_ch = Chapter::from_xhtml(0, None, "");
    let fl_path = crate::device::power::frontlight_path(&hw_cfg.frontlight);
    info!("frontlight: {:?}", fl_path);
    reader.set_brightness_val(cfg.brightness);
    if let Some(ref path) = fl_path {
        crate::device::power::frontlight_set(path, cfg.brightness as u32);
    }
    ReaderSetup {
        reader,
        cfg,
        body_px,
        head_px,
        line_h,
        dummy_ch,
        fl_path,
        caps,
    }
}

fn init_book_or_picker(
    setup: &mut ReaderSetup,
    fb: &Fb,
    window: &std::rc::Rc<MinimalSoftwareWindow>,
    w: usize,
    h: usize,
    all_books: &[EpubEntry],
    cli_path: &Option<String>,
    initial_path: &Option<String>,
) -> (BookInit, PickerInit) {
    let screen = ScreenCtx { fb, window, w, h };
    let (book_state, picker_state) = if cli_path.is_none() && all_books.len() >= 2 {
        (None, init_picker(setup, &screen, all_books))
    } else {
        init_book(setup, &screen, all_books, initial_path)
    };
    let book = book_state.unwrap_or_else(|| {
        let st = layout::build_state(
            &mut setup.dummy_ch.clone(),
            setup.body_px,
            setup.head_px,
            setup.line_h,
        );
        (
            vec![setup.dummy_ch.clone()],
            1,
            vec![0, 1],
            None,
            0,
            st,
            0,
            0,
            0,
            0,
            0,
            String::new(),
            false,
            true,
            crate::ViewMode::Reading,
            None,
            None,
        )
    });
    let picker = picker_state
        .unwrap_or_else(|| (false, 0, Vec::new(), std::collections::HashMap::new(), None));
    (book, picker)
}

fn init_logger() {
    // best-effort: log-file creation failure means no persistent log; app continues
    let _ = std::fs::File::create(KLOG);
    // best-effort: if logger setup fails, app runs without file logging
    let _ = log::set_boxed_logger(Box::new(FileLogger));
    log::set_max_level(log::LevelFilter::Info);
    info!("KoThok reader starting (BUILD_TAG={})", BUILD_TAG);
    std::panic::set_hook(Box::new(|info| {
        use std::io::Write;
        let loc = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_else(|| "?".into());
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic>".into()
        };
        let line = format!("PANIC at {loc}: {msg}");
        // best-effort: we are mid-crash (panic=abort); every write/sync below
        // may itself fail and there is nothing to recover - leave what we can.
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(config::CRASH_LOG)
        {
            let _ = writeln!(f, "{line}");
            let _ = f.sync_data();
        }
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(KLOG)
        {
            let _ = writeln!(f, "{line}");
            let _ = f.sync_data();
        }
    }));
}

fn detect_hardware() -> (hw::DeviceConfig, hw::InputDevices) {
    let mut hw_cfg = hw::detect_device().unwrap_or_else(|| {
        log::warn!("hw: no device detected, using Libra Colour defaults");
        crate::device::registry::DEVICES
            .iter()
            .find(|d| d.codename == "monza")
            .unwrap()
            .clone()
    });
    hw::automagic_battery(&mut hw_cfg);
    hw::automagic_frontlight(&mut hw_cfg);
    crate::device::set_bt_bus(hw_cfg.soc);
    crate::device::log_bt_diagnostics();
    let input_devs = hw::scan_input_devices().unwrap_or_else(|| hw::InputDevices {
        touch_dev: config::TOUCH_DEV.to_string(),
        power_dev: config::POWER_DEV.to_string(),
    });
    (hw_cfg, input_devs)
}

fn resolve_initial_book(cli_path: &Option<String>, all_books: &[EpubEntry]) -> Option<String> {
    cli_path
        .clone()
        .or_else(|| {
            last_book_path(std::path::Path::new(POSITIONS_FILE))
                .filter(|p| all_books.iter().any(|b| b.path == *p))
        })
        .or_else(|| all_books.first().map(|b| b.path.to_string()))
        .or_else(|| {
            if std::path::Path::new(library::DEVICE_BOOK).exists() {
                Some(library::DEVICE_BOOK.to_string())
            } else {
                None
            }
        })
}
