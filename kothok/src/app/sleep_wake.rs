use std::os::unix::io::AsRawFd;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use log::{debug, info};

use slint::platform::software_renderer::Rgb565Pixel;

use crate::audio::Cmd;
use crate::device::power::{frontlight_get, frontlight_set};
use crate::device::{bt_toggle, wifi_status, wifi_toggle};
use crate::loop_state::{LoopContext, LoopState};
use crate::reader::apply_page;
use crate::rendering::common::rgb565_as_bytes_ref;
use crate::rendering::fb::{Fb, WAVE_GC16};
use crate::rendering::layout::PAD_TOP;
use crate::rendering::render::{composite_text, refresh_text_cache, render_book_cover_scaled};

use super::*;

pub fn enter_sleep(st: &mut LoopState, ctx: &LoopContext, from_picker: bool, bt_on: bool) -> u32 {
    let fb = ctx.fb;
    let buffer = &mut st.buffer;
    let prev_buffer = &mut st.prev_buffer;
    let book_path = &st.current_book_path;
    let fl_path = ctx.fl_path;
    let cmd_tx = ctx.cmd_tx;
    let brightness = fl_path
        .as_deref()
        .and_then(frontlight_get)
        .unwrap_or(ctx.reader.get_brightness_val() as u32);
    let w = ctx.w;
    let h = ctx.h;
    let plan = sleep_plan(from_picker, fl_path, wifi_status(), bt_on);
    // Sleep screen: the book cover when locking from a book, the KoThok logo
    // when locking from the library (no book is "open" to show a cover for).
    // Present the cover WHILE LIT (develop behaviour) so the user sees it
    // appear immediately on press; only then dim. Dimming first made the cover
    // render in the dark and pop in ~1s later via ambient light.
    if plan.show_cover {
        // best-effort: cover render is decorative; skip if it fails
        let _ = render_book_cover_scaled(book_path, buffer);
    } else {
        crate::rendering::render::paint_kothok_splash(buffer);
    }
    fb.present(rgb565_as_bytes_ref(buffer), w, h, true, 0, 0, WAVE_GC16);
    prev_buffer.copy_from_slice(buffer);
    // Hold the cover lit through the GC16 refresh so it is visible, then dim.
    std::thread::sleep(std::time::Duration::from_millis(SLEEP_COVER_SETTLE_MS));
    if plan.frontlight_off {
        if let Some(path) = fl_path {
            frontlight_set(path, 0);
        }
    }
    // best-effort: channel may be full
    let _ = cmd_tx.send(Cmd::Stop);
    if plan.wifi_off {
        wifi_toggle(false);
    }
    // Only power BT down when it was actually on. On devices without a BT
    // adapter (e.g. Clara Colour, has_bt=false) the dbus-send call hangs
    // forever - calling it unconditionally stalls enter_sleep and the whole
    // main loop, so power button sleep/wake appears dead. Wake mirrors this
    // with `if reader.get_bt_on()`.
    if plan.bt_off {
        bt_toggle(false);
    }
    brightness
}

pub fn wake_from_sleep(st: &mut LoopState, ctx: &LoopContext) {
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
    reader.set_page((chapter_offsets[current_chapter] + current_page) as i32);
    reader.set_page_count(*chapter_offsets.last().unwrap_or(&1) as i32);
    // Capture the pre-sleep reading marker (where you left off) - the Slint
    // properties survive sleep, but apply_page below resets them.
    let wake_cs = reader.get_cur_start();
    let wake_ce = reader.get_cur_end();
    debug!("pwr: wake marker captured cs={} ce={}", wake_cs, wake_ce);
    // Re-apply the current page so the render is consistent with current_page
    // after wake (no drift to chapter start).
    apply_page(
        reader,
        state,
        current_page,
        chapter_offsets,
        current_chapter,
    );
    // Restore the marker to the last-read line (mid-page), NOT the page default.
    if wake_ce > wake_cs {
        reader.set_cur_start(wake_cs);
        reader.set_cur_end(wake_ce);
    }
    debug!(
        "pwr: wake marker after restore: cs={} ce={}",
        reader.get_cur_start(),
        reader.get_cur_end()
    );
    let utts = crate::audio::glue::page_utterances(current_page, state);
    let target_idx = if wake_ce > wake_cs {
        crate::audio::glue::utterance_index_for_offset(&utts, wake_cs as usize)
    } else {
        0
    };
    let _ = ctx.cmd_tx.send(Cmd::Reload(utts));
    let _ = ctx.cmd_tx.send(Cmd::Seek(target_idx));
    debug!("pwr: wake audio reload + seek to utt {}", target_idx);
    // Swap the screen from the sleep cover to the book page WHILE THE
    // FRONTLIGHT IS STILL OFF (it was dimmed in enter_sleep). Doing the content
    // swap in the dark means the cover is never seen lit; then the light comes
    // up on the already-drawn book page - one clean transition instead of the
    // cover-lit "double blink" on wake.
    let content_end = (PAD_TOP + content_h as usize).min(h);
    buffer[PAD_TOP * w..content_end * w].fill(Rgb565Pixel(0xFFFF));
    window.request_redraw();
    // best-effort: Slint draw may be no-op if no redraw pending
    let _ = window.draw_if_needed(|r| {
        r.render(buffer, w);
    });
    refresh_text_cache(
        text_cache,
        w,
        h,
        &state.all_rows,
        current_page,
        &state.pages,
        PAD_TOP,
        &state.row_heights,
        &state.decoded_images,
        body_px,
        head_px,
        line_h,
    );
    composite_text(
        buffer,
        text_cache,
        w,
        h,
        state.all_rows.as_slice(),
        current_page,
        state.pages.as_slice(),
        PAD_TOP,
        state.row_heights.as_slice(),
        line_h,
        reader.get_cur_start(),
        reader.get_cur_end(),
    );
    let strip_start = (content_end + 70).min(h) * w;
    buffer[strip_start..h * w].fill(Rgb565Pixel(0xFFFF));
    fb.present(rgb565_as_bytes_ref(buffer), w, h, true, 0, 0, WAVE_GC16);
    // Let the full GC16 refresh finish on the panel before raising the
    // frontlight (SEND_UPDATE only schedules and returns at once). Without this
    // the slow cold-panel refresh is seen mid-flight with the light on (the
    // long-sleep wake flicker). The mxcfb WAIT_FOR_UPDATE_COMPLETE ioctl is
    // kernel-version-specific and a wrong number corrupts the refresh, so use a
    // fixed settle - the same proven duration the sleep side uses.
    std::thread::sleep(std::time::Duration::from_millis(SLEEP_COVER_SETTLE_MS));
    prev_buffer.copy_from_slice(buffer);
    // Now bring the frontlight up on the already-drawn book page. Do NOT toggle
    // fb0/bl_power - that file controls the EPD display panel, and its
    // powerdown/unblank cycle visibly blanks the screen (the wake flicker). The
    // frontlight (lm3630a_led/brightness) is a separate path.
    if let Some(ref path) = fl_path {
        crate::device::power::restore_frontlight(path, saved_brightness);
    }
    if reader.get_wifi_on() {
        wifi_toggle(true);
    }
}

pub fn teardown(fb: &Fb, exit_flag: &Arc<AtomicBool>, power_dev: &str, w: usize, h: usize) {
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
        let mut splash = vec![Rgb565Pixel(0); w * h];
        crate::rendering::render::paint_kothok_splash(&mut splash);
        fb.present(rgb565_as_bytes_ref(&splash), w, h, true, 0, 0, WAVE_GC16);
        let r = kobo_core::rendering::loader::spinner_rect(w as i32, h as i32);
        let y0 = (r.y as usize).saturating_sub(4);
        let y1 = ((r.y + r.h + 4) as usize).min(h);
        let mut angle = 0u32;
        for _ in 0..6 {
            angle = (angle + 60) % 360;
            kobo_core::rendering::loader::paint_spinner(
                crate::rendering::common::rgb565_as_bytes(&mut splash),
                w, h, angle,
            );
            fb.present(rgb565_as_bytes_ref(&splash), w, h, false, y0, y1, WAVE_GC16);
            std::thread::sleep(std::time::Duration::from_millis(110));
        }
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
            debug!("power key event sent");
        }
    }
    info!("teardown complete");
}
