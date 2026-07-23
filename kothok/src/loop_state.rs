// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
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
use crate::rendering::render::{CoverCache, GridCell, LibraryFilter};
use crate::{Bookmark, Reader, SystemState, ViewMode};

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
    /// Active filter pill. Not persisted: the library opens on All every time,
    /// so a filter set once can never hide books in a later session.
    pub library_filter: LibraryFilter,
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

    /// A press landed in the capture zone, so the normal tap path is held back
    /// until release. Cleared on release.
    #[cfg(feature = "screenshot")]
    pub shot_armed: bool,
    /// This press already captured; stops one hold writing a frame per poll.
    #[cfg(feature = "screenshot")]
    pub shot_done: bool,

    pub system_state: SystemState,
    pub view_mode: ViewMode,
    /// View mode the last presented frame was drawn for. A mode switch replaces
    /// the whole screen, so it needs a full GC16 present; a partial refresh
    /// leaves the outgoing mode's pixels ghosted on the panel.
    pub prev_view_mode: ViewMode,
    pub bookmark: Option<Bookmark>,
    pub lock_time: Option<Instant>,
    pub saved_brightness: u32,
    /// True when the lock disconnected BT and/or WiFi (entered while paused), so
    /// unlock knows to reconnect. Locking while playing leaves radios on and this
    /// stays false. The per-radio flags below record which ones were actually
    /// turned off, so unlock restores exactly those -- gating reconnect on the
    /// live `wifi_on`/`bt_on` fails, because `refresh_status` flips them false
    /// once the radio drops, and BT then never comes back.
    pub lock_radios_off: bool,
    pub lock_wifi_off: bool,
    pub lock_bt_off: bool,

    /// (book path, chapter, page, rotation step) the current disk image was
    /// rendered for. The disk is costly to rasterise, so it is rebuilt only when
    /// this key changes: the ring advances per page turn, the cover one rotation
    /// step per tick.
    pub disk_key: Option<(String, usize, usize, i32)>,
    /// Decoded cover art shown in the middle of the audio disk, and the book it
    /// was loaded for. Decoding opens the EPUB, so it is reloaded only when the
    /// book actually changes rather than on every disk refresh.
    pub disk_cover: Option<crate::rendering::text_render::DecodedImage>,
    pub disk_cover_path: String,
    /// Marker angle in degrees, advanced while playing in audio mode.
    pub cover_rotation: f32,
    /// The marker's angle on the previous tick. The A2 rect has to span both
    /// positions so the old dot is erased in the same pass that draws the new
    /// one; without this the marker leaves a trail around the annulus.
    pub prev_cover_rotation: f32,
    /// Last time the cover advanced a rotation step.
    pub last_cover_rot: Instant,
    /// Set when the only thing that changed is the cover angle, so the renderer
    /// can refresh just the cover box with A2 instead of a GL16 band.
    pub disk_spin_only: bool,
    /// Set when playback stops: A2 leaves ghosting, so the cover box gets one
    /// GL16 pass to settle once it is no longer moving.
    pub disk_settle: bool,
    pub prev_playing: bool,

    pub prev_down: bool,
    pub frame_down: bool,
    pub frame_x: i32,
    pub frame_y: i32,
    pub tap_xy: Option<(f32, f32)>,
    pub scrubbing: bool,
    pub pp_pressed: bool,
    /// Release time of the last footer play-button tap, held for the
    /// double-click window so a second tap can promote it to a bookmark
    /// instead of toggling playback.
    pub pp_pending_release: Option<Instant>,
    /// Reading position as last written to disk, and when. Used to autosave on
    /// cursor movement without writing the positions file on every TTS
    /// sentence: the tuple detects real movement, the instant rate-limits the
    /// write. `None` means nothing has been saved for this book yet.
    pub saved_pos: Option<(usize, usize, usize)>,
    pub saved_pos_at: Option<Instant>,
    pub lib_pressed: bool,
    pub menu_pressed: bool,
    pub mode_toggle_pressed: bool,
    pub bookmark_set_pressed: bool,
    pub bookmark_jump_pressed: bool,
    pub sleep_pressed: bool,
    pub chapter_pressed: bool,
    pub header_visible: bool,
    pub pending_tap_at: Option<Instant>,
    pub press_dispatched: bool,
    pub press_x: i32,
    pub press_y: i32,
    pub press_time: Instant,
    pub last_double_tap: Instant,
    pub last_tap_time: Instant,
    pub last_tap_y: i32,

    pub exit_armed: bool,
    pub exit_armed_time: Instant,
    pub about_open: bool,
    pub device_model: String,

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

    pub wifi_bt_list_rx: Option<Receiver<crate::panel::WifiBtListResult>>,

    pub font_download_rx: Option<Receiver<crate::device::font_download::FontDownloadResult>>,

    pub wifi_list: Vec<(String, u32)>,
    pub wifi_list_idx: usize,
    pub wifi_list_fetched: bool,
    pub wifi_list_ids_valid: bool,
    pub bt_list: Vec<(String, String)>,
    pub bt_list_idx: usize,
    pub bt_list_fetched: bool,
    pub bt_list_ids_valid: bool,
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
    pub media_signals: &'a crate::device::media_keys::MediaSignals,
    pub fl_path: &'a Option<PathBuf>,
}
