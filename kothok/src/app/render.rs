// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use log::info;

use crate::loop_state::{LoopContext, LoopState};
use crate::rendering::common::rgb565_as_bytes_ref;
use crate::rendering::fb::{diff_rows, waveform_for, RenderScenario, WAVE_A2, WAVE_GC16};
use crate::rendering::layout::{self, PAD_TOP};
use crate::rendering::render::{composite_text, refresh_text_cache};

pub fn render_and_present(
    st: &mut LoopState,
    ctx: &LoopContext,
    had_event: bool,
    ui_changed: bool,
    _page_changed: bool,
) -> bool {
    if st.about_open {
        return false;
    }
    let mode_transition = st.view_mode != st.prev_view_mode;
    let panel_transition = st.panel_open != st.prev_panel_open || mode_transition;
    let overlay_open = ctx.reader.get_chapter_overlay_open();
    if had_event || ui_changed || panel_transition || overlay_open {
        ctx.window.request_redraw();
    }
    let will_draw = had_event || ui_changed || panel_transition || overlay_open;
    if will_draw && matches!(st.view_mode, crate::ViewMode::Audio) {
        info!(
            "render: audio-mode draw (panel={} overlay={} dirty={} loading={})",
            st.panel_open,
            ctx.reader.get_chapter_overlay_open(),
            st.text_dirty,
            ctx.reader.get_loading_visible(),
        );
    }
    if ctx.window.draw_if_needed(|renderer| {
        if panel_transition {
            st.buffer.fill(Rgb565Pixel(0xFFFF));
        }
        renderer.render(&mut st.buffer, ctx.w);
    }) {
        if matches!(st.view_mode, crate::ViewMode::Audio) {
            info!("render: slint render OK, post-render starting");
        }
        let content_end = (PAD_TOP + ctx.content_h as usize).min(ctx.h);
        if st.text_dirty {
            if !st.picker_active && !matches!(st.view_mode, crate::ViewMode::Audio) {
                let pv = crate::rendering::text_overlay::PageView {
                    w: ctx.w,
                    h: ctx.h,
                    rows: &st.state.all_rows,
                    page: st.current_page,
                    pages: &st.state.pages,
                    content_top: PAD_TOP,
                    row_heights: &st.state.row_heights,
                    decoded_images: &st.state.decoded_images,
                    body_px: st.body_px,
                    head_px: st.head_px,
                    line_h: st.line_h,
                    style_runs: &st.state.style_runs,
                };
                refresh_text_cache(&mut st.text_cache, &pv);
            }
            st.text_dirty = false;
        }
        let chapter_overlay = ctx.reader.get_chapter_overlay_open();
        let loading_vis = ctx.reader.get_loading_visible();
        // The picker is Rust-drawn and owns the whole screen, so it outranks the
        // view mode. `view_mode` survives a trip to the library (it is per-book
        // and must be restored on reopen), so without excluding the picker here
        // the audio branch would skip the picker blit and leave Slint's
        // AudioPlayer composited over the library.
        let audio = matches!(st.view_mode, crate::ViewMode::Audio) && !st.picker_active;
        if !st.panel_open && !chapter_overlay && !audio {
            if st.picker_active {
                st.buffer.copy_from_slice(&st.text_cache);
            } else {
                st.buffer[PAD_TOP * ctx.w..content_end * ctx.w].fill(Rgb565Pixel(0xFFFF));
                let pv = crate::rendering::text_overlay::PageView {
                    w: ctx.w,
                    h: ctx.h,
                    rows: &st.state.all_rows,
                    page: st.current_page,
                    pages: &st.state.pages,
                    content_top: PAD_TOP,
                    row_heights: &st.state.row_heights,
                    decoded_images: &st.state.decoded_images,
                    body_px: st.body_px,
                    head_px: st.head_px,
                    line_h: st.line_h,
                    style_runs: &st.state.style_runs,
                };
                composite_text(
                    &mut st.buffer,
                    &st.text_cache,
                    &pv,
                    ctx.reader.get_cur_start(),
                    ctx.reader.get_cur_end(),
                );
                // Clear anything below the footer. The footer box is FOOTER_H tall
                // and content_end == h - FOOTER_H, so this normally clears nothing;
                // it must not be hardcoded shorter than the footer or it wipes a
                // strip through the bottom of the play button.
                let strip_start = (content_end + layout::FOOTER_H as usize).min(ctx.h) * ctx.w;
                st.buffer[strip_start..ctx.h * ctx.w].fill(Rgb565Pixel(0xFFFF));
            }
        } else if chapter_overlay {
            crate::rendering::render::paint_chapter_list(
                &mut st.buffer,
                &st.chapters,
                st.chapter_scroll,
                ctx.reader.get_chapter_preview_idx(),
                st.current_chapter as i32,
            );
        }
        // Marker fast path: refresh ONLY the box the marker moved through, with A2.
        //
        // A2 is ~120ms and does no clearing pass, but it drives every pixel in
        // its region to pure black or white -- so the region must contain
        // neither the colour ring nor the cover art. `vinyl::marker_box` is
        // constrained to the black vinyl annulus for exactly that reason, and
        // `marker_box_stays_in_annulus` asserts it holds at every angle on every
        // panel. This also skips diff_rows entirely: the frame is a known rect.
        let spin_only = st.disk_spin_only
            && !panel_transition
            && !st.panel_open
            && !chapter_overlay
            && !st.picker_active
            && audio;
        st.disk_spin_only = false;
        // A panel/mode transition presents the whole screen on GC16, which
        // already clears the disk's A2 ghosting - so a pending `disk_settle`
        // (set when opening the panel stops the spinning disk) is redundant and
        // must NOT take the disk-only early-return path, or the close present is
        // skipped and the panel pixels are never cleared ("menu not closing").
        if panel_transition {
            st.disk_settle = false;
        }
        if spin_only
            || (st.disk_settle && !panel_transition && audio && !st.panel_open && !st.picker_active)
        {
            let settle = !spin_only;
            st.disk_settle = false;
            // The grains moved, so each rect moves with its own: it spans that
            // grain's old and new positions, erasing the previous dot in the
            // same pass that draws the next. They are presented separately
            // because one rect enclosing all five would span the whole disk and
            // hand the cover and the colour ring to a 2-level waveform.
            //
            // The settle pass instead covers the whole disk in one go -- it runs
            // on GC16, which reproduces colour, so it is free to include the ring
            // and clears ghosting anywhere on any orbit.
            let mut rects =
                [(0usize, 0usize, 0usize, 0usize); crate::rendering::vinyl::GRAIN_COUNT];
            let n = if settle {
                rects[0] = crate::rendering::vinyl::settle_box(ctx.w, ctx.h);
                1
            } else {
                rects = crate::rendering::vinyl::grain_boxes(
                    ctx.w,
                    ctx.h,
                    st.prev_cover_rotation,
                    st.cover_rotation,
                );
                crate::rendering::vinyl::GRAIN_COUNT
            };
            for &(rx, ry, rw, rh) in &rects[..n] {
                ctx.fb.present_rect(
                    rgb565_as_bytes_ref(&st.buffer),
                    ctx.w,
                    ctx.h,
                    &kobo_core::device::fb::UpdateRegion {
                        x: rx,
                        y: ry,
                        w: rw,
                        h: rh,
                    },
                    // One 16-level pass once the disk stops, to clear A2's ghosting.
                    if settle { WAVE_GC16 } else { WAVE_A2 },
                );
                for row in ry..(ry + rh).min(ctx.h) {
                    let s = row * ctx.w + rx;
                    let e = (s + rw).min(row * ctx.w + ctx.w);
                    st.prev_buffer[s..e].copy_from_slice(&st.buffer[s..e]);
                }
            }
            st.prev_panel_open = st.panel_open;
            st.prev_view_mode = st.view_mode;
            return false;
        }

        st.prev_panel_open = st.panel_open;
        st.prev_view_mode = st.view_mode;
        let overlay_transition = chapter_overlay != st.prev_chapter_overlay;
        if overlay_transition {
            st.prev_chapter_overlay = chapter_overlay;
        }
        // GL16 does not fully clear, which shows as the outgoing screen ghosting
        // under the new one. That is tolerable for reading mode's panel (mostly
        // text over text) but not for anything involving the audio player, where
        // the swap is near-total (disk + ring <-> sliders). Pay the GC16 flash
        // there; AUDIO_PLAYER_MODE.md specifies GC16 for mode switches anyway.
        let heavy_swap = mode_transition || matches!(st.view_mode, crate::ViewMode::Audio);
        let trans_wf = if heavy_swap {
            WAVE_GC16
        } else {
            waveform_for(RenderScenario::Transition)
        };
        let content_wf = waveform_for(RenderScenario::Content);
        if panel_transition || overlay_transition {
            ctx.fb.present(
                rgb565_as_bytes_ref(&st.buffer),
                ctx.w,
                ctx.h,
                true,
                0,
                ctx.h,
                trans_wf,
            );
            st.prev_buffer.copy_from_slice(&st.buffer);
        } else if let Some((top, rh)) = diff_rows(
            rgb565_as_bytes_ref(&st.prev_buffer),
            rgb565_as_bytes_ref(&st.buffer),
            ctx.w,
            ctx.h,
        ) {
            if matches!(st.view_mode, crate::ViewMode::Audio) {
                info!("render: diff_rows top={} rh={}", top, rh);
            }
            // While loading in audio mode, hold the disk still. The radios are
            // connecting and `refresh_status`/loading-pct churn the header,
            // footer and progress bar every tick; each produces a diff band that
            // spans the disk and would be repainted with GL16 (a flash) -- read
            // as the disk blinking. Skip these incidental presents and leave
            // prev_buffer untouched so the change is carried until the load
            // completes, which presents the settled screen once.
            let suppress = audio && loading_vis;
            if !suppress {
                ctx.fb.present(
                    rgb565_as_bytes_ref(&st.buffer),
                    ctx.w,
                    ctx.h,
                    false,
                    top,
                    rh,
                    content_wf,
                );
                let strip = ctx.w * rh;
                st.prev_buffer[top * ctx.w..top * ctx.w + strip]
                    .copy_from_slice(&st.buffer[top * ctx.w..top * ctx.w + strip]);
            }
        } else if loading_vis && !audio {
            ctx.fb.present(
                rgb565_as_bytes_ref(&st.buffer),
                ctx.w,
                ctx.h,
                false,
                0,
                ctx.h,
                content_wf,
            );
            st.prev_buffer.copy_from_slice(&st.buffer);
        }
        false
    } else {
        false
    }
}
