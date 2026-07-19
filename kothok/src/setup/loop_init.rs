// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use log::info;

use super::book_init::{BookInit, PickerInit};

pub(super) fn build_loop_state(
    book: BookInit,
    picker: PickerInit,
    body_px: f32,
    head_px: f32,
    line_h: i32,
    w: usize,
    h: usize,
    device_model: &'static str,
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
        library_filter: crate::rendering::render::LibraryFilter::default(),
        picker_cover_cache: picker.3,
        picker_entered: picker.4,
        panel_open: false,
        prev_panel_open: false,
        prev_chapter_overlay: false,
        cover_page_visible: book.12,
        chapter_scroll: 0,
        text_dirty: book.13,
        #[cfg(feature = "screenshot")]
        shot_armed: false,
        #[cfg(feature = "screenshot")]
        shot_done: false,
        system_state: crate::SystemState::Awake,
        view_mode: book.14,
        prev_view_mode: book.14,
        bookmark: book.15,
        lock_time: None,
        saved_brightness: 0,
        lock_radios_off: false,
        lock_wifi_off: false,
        lock_bt_off: false,
        disk_key: None,
        disk_cover: None,
        disk_cover_path: String::new(),
        cover_rotation: 0.0,
        prev_cover_rotation: 0.0,
        last_cover_rot: now,
        disk_spin_only: false,
        disk_settle: false,
        prev_playing: false,
        prev_down: false,
        frame_down: false,
        frame_x: 0,
        frame_y: 0,
        tap_xy: None,
        scrubbing: false,
        pp_pressed: false,
        lib_pressed: false,
        menu_pressed: false,
        mode_toggle_pressed: false,
        bookmark_set_pressed: false,
        bookmark_jump_pressed: false,
        sleep_pressed: false,
        chapter_pressed: false,
        header_visible: true,
        pending_tap_at: None,
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
        about_open: false,
        device_model: device_model.to_string(),
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
        wifi_bt_list_rx: None,
        wifi_list: Vec::new(),
        wifi_list_idx: 0,
        wifi_list_fetched: false,
        wifi_list_ids_valid: true,
        bt_list: Vec::new(),
        bt_list_idx: 0,
        bt_list_fetched: false,
        bt_list_ids_valid: true,
    }
}
