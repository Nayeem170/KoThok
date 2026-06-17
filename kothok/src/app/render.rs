use slint::platform::software_renderer::Rgb565Pixel;

use crate::loop_state::{LoopContext, LoopState};
use crate::rendering::common::rgb565_as_bytes_ref;
use crate::rendering::fb::{diff_rows, waveform_for, RenderScenario};
use crate::rendering::layout::PAD_TOP;
use crate::rendering::render::{composite_text, refresh_text_cache};

pub fn render_and_present(
    st: &mut LoopState,
    ctx: &LoopContext,
    had_event: bool,
    ui_changed: bool,
    _page_changed: bool,
) -> bool {
    let panel_transition = st.panel_open != st.prev_panel_open;
    let overlay_open = ctx.reader.get_chapter_overlay_open();
    if had_event || ui_changed || panel_transition || overlay_open {
        ctx.window.request_redraw();
    }
    if ctx.window.draw_if_needed(|renderer| {
        if panel_transition {
            st.buffer.fill(Rgb565Pixel(0xFFFF));
        }
        renderer.render(&mut st.buffer, ctx.w);
    }) {
        let content_end = (PAD_TOP + ctx.content_h as usize).min(ctx.h);
        if st.text_dirty {
            if !st.picker_active {
                refresh_text_cache(
                    &mut st.text_cache,
                    ctx.w,
                    ctx.h,
                    &st.state.all_rows,
                    st.current_page,
                    &st.state.pages,
                    PAD_TOP,
                    &st.state.row_heights,
                    &st.state.decoded_images,
                    st.body_px,
                    st.head_px,
                    st.line_h,
                );
            }
            st.text_dirty = false;
        }
        let chapter_overlay = ctx.reader.get_chapter_overlay_open();
        let loading_vis = ctx.reader.get_loading_visible();
        if !st.panel_open && !chapter_overlay {
            if st.picker_active {
                st.buffer.copy_from_slice(&st.text_cache);
            } else {
                st.buffer[PAD_TOP * ctx.w..content_end * ctx.w].fill(Rgb565Pixel(0xFFFF));
                composite_text(
                    &mut st.buffer,
                    &st.text_cache,
                    ctx.w,
                    ctx.h,
                    st.state.all_rows.as_slice(),
                    st.current_page,
                    st.state.pages.as_slice(),
                    PAD_TOP,
                    st.state.row_heights.as_slice(),
                    st.line_h,
                    ctx.reader.get_cur_start(),
                    ctx.reader.get_cur_end(),
                );
                let strip_start = (content_end + 70).min(ctx.h) * ctx.w;
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
        st.prev_panel_open = st.panel_open;
        let overlay_transition = chapter_overlay != st.prev_chapter_overlay;
        if overlay_transition {
            st.prev_chapter_overlay = chapter_overlay;
        }
        let trans_wf = waveform_for(RenderScenario::Transition);
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
        } else if loading_vis {
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
