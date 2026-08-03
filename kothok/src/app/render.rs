// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use log::info;

use crate::loop_state::{ChapterTab, LoopContext, LoopState};
use crate::rendering::common::{rgb565_as_bytes, rgb565_as_bytes_ref};
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
    let overlay_transition = overlay_open != st.prev_chapter_overlay;
    if had_event || ui_changed || panel_transition || overlay_transition {
        ctx.window.request_redraw();
    }
    let will_draw = had_event || ui_changed || panel_transition || overlay_transition;
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
        if panel_transition || overlay_transition {
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
                if st.zoom_active {
                    crate::rendering::render::apply_zoom(
                        &mut st.buffer,
                        ctx.w,
                        PAD_TOP,
                        content_end - PAD_TOP,
                        st.zoom_center,
                    );
                }
                // Clear anything below the footer. The footer box is FOOTER_H tall
                // and content_end == h - FOOTER_H, so this normally clears nothing;
                // it must not be hardcoded shorter than the footer or it wipes a
                // strip through the bottom of the play button.
                let strip_start = (content_end + layout::FOOTER_H as usize).min(ctx.h) * ctx.w;
                st.buffer[strip_start..ctx.h * ctx.w].fill(Rgb565Pixel(0xFFFF));
            }
        } else if chapter_overlay {
            if chapter_overlay != st.prev_chapter_overlay {
                info!(
                    "overlay: paint tab={:?} results={} words={}",
                    st.chapter_tab,
                    st.search_results_active,
                    st.word_index.words.len(),
                );
            }
            if st.search_results_active {
                let word = st
                    .word_index
                    .words
                    .get(st.search_selected_word)
                    .map(|s| s.as_str())
                    .unwrap_or("");
                let hits = st
                    .word_index
                    .occurrences
                    .get(st.search_selected_word)
                    .map(|h| h.as_slice())
                    .unwrap_or(&[]);
                let total = hits.len();
                crate::rendering::search_results::paint_search_results(
                    &mut st.buffer,
                    word,
                    hits,
                    &st.chapters,
                    st.search_results_scroll,
                    st.body_px,
                    total,
                    if st.search_result_selected {
                        st.search_selected_result
                    } else {
                        usize::MAX
                    },
                    st.sb_dragging && st.search_results_active,
                );
            } else if st.chapter_tab == ChapterTab::Words {
                crate::rendering::word_list::paint_word_list(
                    &mut st.buffer,
                    &st.word_index.words,
                    st.search_scroll,
                    st.body_px,
                    if st.search_word_selected {
                        st.search_selected_word
                    } else {
                        usize::MAX
                    },
                    st.sb_dragging && st.sb_drag_tab == ChapterTab::Words,
                );
            } else {
                let current_row =
                    crate::rendering::render::current_toc_row(&st.toc_rows, st.current_chapter)
                        .unwrap_or(0) as i32;
                crate::rendering::render::paint_chapter_list(
                    &mut st.buffer,
                    &st.toc_rows,
                    st.chapter_scroll,
                    ctx.reader.get_chapter_preview_idx(),
                    current_row,
                );
                let buf_bytes = rgb565_as_bytes(&mut st.buffer);
                crate::rendering::chapter_list::paint_scrollbar(
                    buf_bytes,
                    ctx.w,
                    ctx.h,
                    st.toc_rows.len(),
                    st.chapter_scroll,
                    st.sb_dragging && st.sb_drag_tab == ChapterTab::Chapters,
                );
            }
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
                    if settle { WAVE_GC16 } else { WAVE_A2 },
                    false,
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
        // under the new one. That is tolerable for reading mode's panel and the
        // chapter list (both mostly text over text, plus thin lines -- see
        // chapter_list.rs's card fill, which was changed to match) but not for
        // anything involving the audio player, where the swap is near-total
        // (disk + ring <-> sliders). Pay the GC16 flash there;
        // AUDIO_PLAYER_MODE.md specifies GC16 for mode switches anyway.
        //
        // GC16 was forced on the chapter overlay for a while to fight
        // ghosting from a solid grey card fill (removed -- see
        // chapter_list.rs). Chasing that with waveform swaps (GC16 -> GL16 ->
        // A2, none of which fixed the ghosting because none of them address
        // the actual amount of grey being refreshed) is why this kept
        // recurring. With the large grey fill gone, the chapter list is
        // content-wise the same shape as every other quiet screen, so every
        // transition present now goes through the same policy
        // (`PanelTransition`) instead of a per-case waveform guess.
        //
        // The PPM/nonwhite diagnostic that used to run here has served its
        // purpose: the buffer is white-filled and clean, so the residue is on
        // the glass. What is left to settle is the waveform, which the policy
        // logs below.
        let content_wf = waveform_for(RenderScenario::Content);
        if panel_transition || overlay_transition {
            present_transition(st, ctx);
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

/// Present a whole-screen transition (panel open/close, mode switch, chapter
/// overlay) under the configured waveform policy.
///
/// The white clearing pass reuses `prev_buffer` rather than allocating one:
/// it is overwritten with the new frame immediately afterwards, and a
/// full-screen RGB565 buffer is ~4 MB to allocate on a transition.
fn present_transition(st: &mut LoopState, ctx: &LoopContext) {
    let mode = ctx.cfg.panel_transition;
    let wf = mode.waveform();
    let full = mode.full();
    info!(
        "transition: mode={} wf={} um={} panel_open={}",
        mode.as_key(),
        wf,
        u8::from(full),
        st.panel_open
    );
    if mode.needs_white_pass() {
        st.prev_buffer.fill(Rgb565Pixel(0xFFFF));
        ctx.fb.present(
            rgb565_as_bytes_ref(&st.prev_buffer),
            ctx.w,
            ctx.h,
            false,
            0,
            ctx.h,
            wf,
        );
        // The second pass must not be merged with the white one by the
        // driver's update queue, or the clear never reaches the glass.
        ctx.fb.wait_for_update_complete();
    }
    ctx.fb.present(
        rgb565_as_bytes_ref(&st.buffer),
        ctx.w,
        ctx.h,
        full,
        0,
        ctx.h,
        wf,
    );
    st.prev_buffer.copy_from_slice(&st.buffer);
}
