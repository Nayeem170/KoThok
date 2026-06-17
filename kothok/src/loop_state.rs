use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Instant;

use kobo_core::Chapter;
use slint::platform::software_renderer::{MinimalSoftwareWindow, Rgb565Pixel};

use crate::audio::{Cmd, Event};
use crate::callbacks::Callbacks;
use crate::capabilities::KoboCapabilities;
use crate::data::config::AppConfig;
use crate::data::library::EpubEntry;
use crate::device::touch::TouchConfig;
use crate::rendering::fb::Fb;
use crate::rendering::layout::{ChapterState, OffsetComputation};
use crate::rendering::render::{CoverCache, GridCell};
use crate::{Reader, SystemState};

pub struct LoopState {
    pub current_chapter: usize,
    pub current_page: usize,
    pub chapter_count: usize,
    pub chapters: Vec<Chapter>,
    pub chapter_offsets: Vec<usize>,
    pub state: ChapterState,

    pub body_px: f32,
    pub head_px: f32,
    pub line_h: i32,

    pub current_book_path: String,

    pub reading_ch: usize,
    pub reading_pg: usize,
    pub reading_off: usize,
    pub reading_end: usize,

    pub picker_active: bool,
    pub picker_scroll: i32,
    pub picker_cells: Vec<GridCell>,
    pub picker_cover_cache: CoverCache,
    pub picker_entered: Option<Instant>,
    pub picker_last_tap_idx: Option<usize>,
    pub picker_last_tap_time: Instant,

    pub panel_open: bool,
    pub prev_panel_open: bool,
    pub prev_chapter_overlay: bool,
    pub cover_page_visible: bool,
    pub chapter_scroll: i32,
    pub text_dirty: bool,

    pub system_state: SystemState,
    pub saved_brightness: u32,

    pub prev_down: bool,
    pub frame_down: bool,
    pub frame_x: i32,
    pub frame_y: i32,
    pub tap_xy: Option<(f32, f32)>,
    pub scrubbing: bool,
    pub pp_pressed: bool,
    pub press_dispatched: bool,
    pub press_x: i32,
    pub press_y: i32,
    pub press_time: Instant,
    pub last_double_tap: Instant,
    pub last_tap_time: Instant,
    pub last_tap_y: i32,

    pub exit_armed: bool,
    pub exit_armed_time: Instant,

    pub offset_rx: Option<OffsetComputation>,

    pub last_activity: Instant,
    pub last_status_refresh: Instant,
    pub last_nav: Instant,
    pub last_font_count: usize,

    pub buffer: Vec<Rgb565Pixel>,
    pub prev_buffer: Vec<Rgb565Pixel>,
    pub text_cache: Vec<Rgb565Pixel>,

    pub voice_rx: Option<Receiver<Vec<kothok_edge_tts::VoiceInfo>>>,
    pub voice_fetch_attempted: bool,
    pub page_break_advanced: bool,
}

pub struct LoopContext<'a> {
    pub reader: &'a Reader,
    pub window: &'a Rc<MinimalSoftwareWindow>,
    pub fb: &'a Fb,
    pub cmd_tx: &'a Sender<Cmd>,
    pub evt_rx: &'a Receiver<Event>,
    pub cb: &'a Callbacks,
    pub cfg: &'a mut AppConfig,
    pub all_books: &'a mut Vec<EpubEntry>,
    pub caps: &'a KoboCapabilities,
    pub touch_dev: &'a mut std::fs::File,
    pub touch_fd: i32,
    pub touch_cfg: &'a TouchConfig,
    pub content_h: i32,
    pub w: usize,
    pub h: usize,
    pub power_pressed: &'a Arc<AtomicBool>,
    pub fl_path: &'a Option<PathBuf>,
}
