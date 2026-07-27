// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::os::unix::io::AsRawFd;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use log::info;

use slint::platform::software_renderer::Rgb565Pixel;

use crate::audio::glue::best_effort_send;
use crate::audio::Cmd;
use crate::device::power::{frontlight_get, frontlight_set};
use crate::device::{bt_toggle, wifi_toggle};
use crate::loop_state::{LoopContext, LoopState};
use crate::reader::apply_page;
use crate::rendering::common::rgb565_as_bytes_ref;
use crate::rendering::fb::{Fb, WAVE_GC16};
use crate::rendering::layout::PAD_TOP;
use crate::rendering::render::{composite_text, refresh_text_cache};

use kobo_core::Capabilities;

use super::*;

pub fn enter_sleep(st: &mut LoopState, ctx: &LoopContext, from_picker: bool) -> u32 {
    crate::debug_log::log(&format!(
        "enter_sleep: from_picker={} wifi_user={} bt_user={}",
        from_picker, st.wifi_user_on, st.bt_user_on
    ));
    if !from_picker {
        crate::loop_run::save_position_now(st, ctx.reader);
    }
    let fb = ctx.fb;
    let buffer = &mut st.buffer;
    let prev_buffer = &mut st.prev_buffer;
    let fl_path = ctx.fl_path;
    let cmd_tx = ctx.cmd_tx;
    let brightness = fl_path
        .as_deref()
        .and_then(frontlight_get)
        .unwrap_or(ctx.reader.get_brightness_val() as u32);
    let w = ctx.w;
    let h = ctx.h;
    let plan = sleep_plan(fl_path, st.wifi_user_on, st.bt_user_on);
    let status = if !from_picker && !st.current_book_path.is_empty() {
        let title = ctx.reader.get_book_title().to_string();
        let abs_page = st
            .chapter_offsets
            .get(st.current_chapter)
            .copied()
            .unwrap_or(0)
            + st.current_page;
        let total = st.chapter_offsets.last().copied().unwrap_or(1).max(1);
        let pct = (abs_page * 100 / total).min(100);
        format!("{} - {}%", title, pct)
    } else {
        String::new()
    };
    crate::rendering::render::paint_splash(
        buffer,
        crate::rendering::render::SPLASH_STAGES,
        &status,
    );
    fb.present(rgb565_as_bytes_ref(buffer), w, h, false, 0, h, WAVE_GC16);
    prev_buffer.copy_from_slice(buffer);
    // Hold lit through the refresh, then dim -- dimming first made it pop in later.
    fb.wait_for_update_complete();
    if plan.frontlight_off {
        if let Some(path) = fl_path {
            frontlight_set(path, 0);
        }
    }
    best_effort_send(cmd_tx, Cmd::Stop);
    if plan.wifi_off {
        wifi_toggle(false);
    }
    // BT toggle hangs on adapter-less devices (Clara Colour); gated on user intent.
    if plan.bt_off {
        bt_toggle(false);
    }
    brightness
}

pub fn wake_from_sleep(st: &mut LoopState, ctx: &LoopContext) {
    crate::debug_log::log("wake_from_sleep: entering");
    st.about_open = false;
    st.picker_last_tap_idx = None;
    st.exit_armed = false;

    if st.picker_active {
        wake_from_sleep_picker(st, ctx);
        return;
    }

    let reader = ctx.reader;
    let window = ctx.window;
    let fb = ctx.fb;
    let buffer = &mut st.buffer;
    let prev_buffer = &mut st.prev_buffer;
    let text_cache = &mut st.text_cache;
    let state = &st.state;
    let chapter_offsets = &st.chapter_offsets;
    let current_chapter = st.current_chapter;
    let current_page = st.current_page;
    let fl_path = ctx.fl_path;
    let saved_brightness = st.saved_brightness;
    let body_px = st.body_px;
    let head_px = st.head_px;
    let line_h = st.line_h;
    let w = ctx.w;
    let h = ctx.h;
    let content_h = ctx.content_h;
    reader.set_page((*chapter_offsets.get(current_chapter).unwrap_or(&0) + current_page) as i32);
    reader.set_page_count(*chapter_offsets.last().unwrap_or(&1) as i32);
    // Capture pre-sleep marker before apply_page resets it.
    let wake_cs = reader.get_cur_start();
    let wake_ce = reader.get_cur_end();
    apply_page(
        reader,
        state,
        current_page,
        chapter_offsets,
        current_chapter,
    );
    // Restore mid-page marker (apply_page resets to page default).
    if wake_ce > wake_cs {
        reader.set_cur_start(wake_cs);
        reader.set_cur_end(wake_ce);
    }
    let utts = crate::audio::glue::page_utterances(current_page, state);
    let target_idx = if wake_ce > wake_cs {
        crate::audio::glue::utterance_index_for_offset(&utts, wake_cs as usize)
    } else {
        0
    };
    best_effort_send(&ctx.cmd_tx, Cmd::Reload(utts));
    best_effort_send(&ctx.cmd_tx, Cmd::Seek(target_idx));
    // Swap cover->page while frontlight is still off to avoid double blink.
    let content_end = (PAD_TOP + content_h as usize).min(h);
    buffer[PAD_TOP * w..content_end * w].fill(Rgb565Pixel(0xFFFF));
    window.request_redraw();
    let _ = window.draw_if_needed(|r| {
        r.render(buffer, w);
    });
    let pv = crate::rendering::text_overlay::PageView {
        w,
        h,
        rows: &state.all_rows,
        page: current_page,
        pages: &state.pages,
        content_top: PAD_TOP,
        row_heights: &state.row_heights,
        decoded_images: &state.decoded_images,
        body_px,
        head_px,
        line_h,
        style_runs: &state.style_runs,
    };
    refresh_text_cache(text_cache, &pv);
    composite_text(
        buffer,
        text_cache,
        &pv,
        reader.get_cur_start(),
        reader.get_cur_end(),
    );
    let strip_start = (content_end + 70).min(h) * w;
    buffer[strip_start..h * w].fill(Rgb565Pixel(0xFFFF));
    fb.present(rgb565_as_bytes_ref(buffer), w, h, false, 0, h, WAVE_GC16);
    // Wait for GC16 to finish before raising light (avoids mid-refresh flicker).
    fb.wait_for_update_complete();
    prev_buffer.copy_from_slice(buffer);
    // Do NOT toggle fb0/bl_power - that controls the EPD panel and blanks the
    // screen. The frontlight (lm3630a_led/brightness) is a separate path.
    if let Some(ref path) = fl_path {
        crate::device::power::restore_frontlight(path, saved_brightness);
    }
    if st.wifi_user_on {
        wifi_toggle(true);
    }
}

pub fn teardown(
    fb: &Fb,
    exit_flag: &Arc<AtomicBool>,
    power_dev: &str,
    w: usize,
    h: usize,
    session: &str,
) {
    exit_flag.store(true, Ordering::SeqCst);
    {
        if let Ok(dev) = std::fs::OpenOptions::new().write(true).open(power_dev) {
            let fd = dev.as_raw_fd();
            let dummy: [u8; 24] = [0; 24];
            // SAFETY: fd is the valid POWER_DEV descriptor (dev alive on this block). write()
            // reads 24 bytes from the &[u8;24] (valid pointer + length); a failing write
            // returns <0 and is non-fatal (best-effort power-state poke on teardown).
            unsafe {
                libc::write(fd, dummy.as_ptr() as *const _, 24);
            }
            drop(dev);
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(200));
    // Show the KoThok splash during the reboot window so the screen isn't a
    // frozen nickel frame. Exit always reboots (AGENTS.md), so we never restore
    // nickel's framebuffer - the logo makes the shutdown look intentional.
    {
        // The exit splash is the entry splash, already fully inked, with the
        // status line reporting the session instead of the boot. Same layout and
        // the same paint path, so the two screens cannot drift apart -- and the
        // reboot wait becomes something worth looking at rather than dead time.
        //
        // Nothing animates here: shutdown has no progress to report, and a still
        // screen is the calmest thing e-ink can show.
        let mut splash = vec![Rgb565Pixel(0); w * h];
        crate::rendering::render::paint_splash(
            &mut splash,
            crate::rendering::render::SPLASH_STAGES,
            session,
        );
        fb.present(rgb565_as_bytes_ref(&splash), w, h, true, 0, 0, WAVE_GC16);
        fb.wait_for_update_complete();
    }
    {
        const EV_KEY: u16 = 1;
        const EV_SYN: u16 = 0;
        const KEY_POWER: u16 = 116;
        const SYN_REPORT: u16 = 0;
        fn ie(typ: u16, code: u16, val: i32) -> [u8; 24] {
            let mut b = [0u8; 24];
            b[8..10].copy_from_slice(&typ.to_le_bytes());
            b[10..12].copy_from_slice(&code.to_le_bytes());
            b[12..16].copy_from_slice(&val.to_le_bytes());
            b
        }
        if let Ok(dev) = std::fs::OpenOptions::new().write(true).open(power_dev) {
            let fd = dev.as_raw_fd();
            // SAFETY: fd is the valid power_dev descriptor (dev alive on this block). write()
            // reads 24 bytes from each ie(..) [u8;24] (valid pointer + length). The two writes
            // synthesize a power-key press event to hand control back to nickel; failures
            // return <0 and are non-fatal (best-effort).
            unsafe {
                libc::write(fd, ie(EV_KEY, KEY_POWER, 1).as_ptr() as *const _, 24);
                libc::write(fd, ie(EV_SYN, SYN_REPORT, 0).as_ptr() as *const _, 24);
            }
            drop(dev);
            std::thread::sleep(std::time::Duration::from_millis(100));
            if let Ok(dev) = std::fs::OpenOptions::new().write(true).open(power_dev) {
                let fd = dev.as_raw_fd();
                // SAFETY: same as above - valid fd, [u8;24] source, best-effort power-key
                // release event.
                unsafe {
                    libc::write(fd, ie(EV_KEY, KEY_POWER, 0).as_ptr() as *const _, 24);
                    libc::write(fd, ie(EV_SYN, SYN_REPORT, 0).as_ptr() as *const _, 24);
                }
                drop(dev);
            }
        }
    }
    info!("teardown complete");
}

fn wake_from_sleep_picker(st: &mut LoopState, ctx: &LoopContext) {
    let fl_path = ctx.fl_path;
    let saved_brightness = st.saved_brightness;

    crate::rendering::picker::show_book_picker(
        ctx.reader,
        ctx.fb,
        ctx.window,
        &mut st.buffer,
        &mut st.text_cache,
        &mut st.picker_cover_cache,
        ctx.all_books,
        st.picker_scroll,
        st.library_filter,
        &ctx.caps.current_clock(),
        ctx.caps.battery_pct(),
        "",
        crate::rendering::picker::PickerRefresh::Full,
    );
    st.prev_buffer.copy_from_slice(&st.buffer);

    ctx.fb.wait_for_update_complete();

    if let Some(ref path) = fl_path {
        crate::device::power::restore_frontlight(path, saved_brightness);
    }
}
