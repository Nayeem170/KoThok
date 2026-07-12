mod book_init;

use book_init::{init_book, init_picker, BookInit, PickerInit, ReaderSetup, ScreenCtx};
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
use crate::rendering::common::{rgb565_as_bytes, rgb565_as_bytes_ref};
use crate::rendering::fb::{Fb, WAVE_DU, WAVE_GC16};
use crate::rendering::layout::{self, init_layout, PAD_TOP};
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
    let content_h: i32 = (h - PAD_TOP - 92) as i32;

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
    );

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

fn scan_and_resolve(
    fb: &Fb,
    w: usize,
    h: usize,
    cli_path: &Option<String>,
) -> (Option<String>, Vec<EpubEntry>) {
    let scan_handle = std::thread::spawn(move || scan_epubs(BOOK_DIR).unwrap_or_default());

    let mut splash = vec![Rgb565Pixel(0); w * h];
    render::paint_kothok_splash(&mut splash);
    fb.present(rgb565_as_bytes_ref(&splash), w, h, true, 0, 0, WAVE_GC16);

    fonts::log_available_fonts();
    let font_handle = std::thread::spawn(|| {
        fonts::load_cached_fonts();
    });

    let r = kobo_core::rendering::loader::spinner_rect(w as i32, h as i32);
    let y0 = (r.y as usize).saturating_sub(4);
    let y1 = ((r.y + r.h + 4) as usize).min(h);
    let mut angle = 0u32;
    while !scan_handle.is_finished() || !font_handle.is_finished() {
        angle = (angle + 30) % 360;
        kobo_core::rendering::loader::paint_spinner(
            rgb565_as_bytes(&mut splash),
            w,
            h,
            angle,
        );
        fb.present(rgb565_as_bytes_ref(&splash), w, h, false, y0, y1, WAVE_DU);
        std::thread::sleep(std::time::Duration::from_millis(80));
    }
    render::paint_kothok_splash(&mut splash);
    fb.present(rgb565_as_bytes_ref(&splash), w, h, true, 0, 0, WAVE_GC16);

    font_handle.join().unwrap_or(());
    let all_books = scan_handle.join().unwrap_or_default();
    let initial_path = resolve_initial_book(cli_path, &all_books);
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
    init_layout(w, h);
    window.set_size(slint::PhysicalSize::new(w as u32, h as u32));
    window
}

fn init_reader_and_config(w: usize, hw_cfg: &hw::DeviceConfig) -> ReaderSetup {
    let reader = crate::Reader::new().expect("Reader::new");
    let device_default_font = (w as i32 / 30).clamp(20, 60);
    let cfg = config::load_config_from_base(config::CONFIG_FILE, device_default_font);
    let body_px: f32 = cfg.font_size as f32;
    let head_px: f32 = cfg.font_size as f32 * 0.78;
    let line_h: i32 = (cfg.font_size as f32 * 1.4) as i32;
    reader.set_tts_lang(SharedString::from(cfg.tts_lang.clone()));
    reader.set_tts_voice(SharedString::from(cfg.tts_voice.clone()));
    reader.set_tts_speed(cfg.tts_rate);
    reader.set_font_size_val(cfg.font_size);
    let caps = KoboCapabilities;
    reader.set_wifi_on(caps.network_available());
    reader.set_bt_on(caps.audio_sink_available());
    reader.set_play_enabled(caps.read_aloud_available());
    if let Some(n) = caps.wifi_name() {
        reader.set_wifi_name(SharedString::from(n));
    }
    if let Some(n) = caps.bt_name() {
        reader.set_bt_name(SharedString::from(n));
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
        )
    });
    let picker = picker_state
        .unwrap_or_else(|| (false, 0, Vec::new(), std::collections::HashMap::new(), None));
    (book, picker)
}

fn build_loop_state(
    book: BookInit,
    picker: PickerInit,
    body_px: f32,
    head_px: f32,
    line_h: i32,
    w: usize,
    h: usize,
) -> crate::loop_state::LoopState {
    let cached = crate::panel::load_voice_cache();
    if !cached.is_empty() {
        let count = cached.len();
        crate::panel::set_dynamic_voices(cached);
        info!("loaded {count} cached voices");
    }
    let buffer = vec![Rgb565Pixel(0); w * h];
    let prev_buffer = buffer.clone();
    let text_cache = vec![Rgb565Pixel(0); w * h];
    let now = std::time::Instant::now();
    crate::loop_state::LoopState {
        current_chapter: book.4,
        current_page: book.6,
        chapter_count: book.1,
        chapters: book.0,
        chapter_offsets: book.2,
        state: book.5,
        body_px,
        head_px,
        line_h,
        current_book_path: book.11,
        reading_ch: book.7,
        reading_pg: book.8,
        reading_off: book.9,
        reading_end: book.10,
        picker_active: picker.0,
        picker_scroll: picker.1,
        picker_cells: picker.2,
        picker_cover_cache: picker.3,
        picker_entered: picker.4,
        panel_open: false,
        prev_panel_open: false,
        prev_chapter_overlay: false,
        cover_page_visible: book.12,
        chapter_scroll: 0,
        text_dirty: book.13,
        system_state: crate::SystemState::Awake,
        saved_brightness: 0,
        prev_down: false,
        frame_down: false,
        frame_x: 0,
        frame_y: 0,
        tap_xy: None,
        scrubbing: false,
        pp_pressed: false,
        press_dispatched: false,
        press_x: 0,
        press_y: 0,
        press_time: now,
        last_double_tap: now,
        last_tap_time: now,
        last_tap_y: -1,
        picker_last_tap_idx: None,
        picker_last_tap_time: now,
        exit_armed: false,
        exit_armed_time: now,
        offset_rx: book.3,
        last_activity: now,
        last_status_refresh: now,
        last_nav: now,
        last_font_count: 0,
        buffer,
        prev_buffer,
        text_cache,
        voice_rx: None,
        voice_fetch_attempted: false,
    }
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
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(config::CRASH_LOG)
        {
            // best-effort: panic-hook write - nothing can be done if this fails
            let _ = writeln!(f, "PANIC: {}", info);
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
