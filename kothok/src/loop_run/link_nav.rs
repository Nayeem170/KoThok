// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
//! Tap-to-follow for in-book hyperlinks.
//!
//! A tap in the text column resolves to a `body` byte offset, that offset to a
//! link run, and the run's `href` to a spine chapter plus an optional anchor.
//! Links that leave the book (`http:`, `mailto:`) and hrefs naming a file
//! outside the spine resolve to nothing and fall through to the normal tap
//! handling, so tapping ordinary prose still behaves as it always did.

use crate::audio::Cmd;
use crate::loop_state::LoopState;
use crate::reader::jump_to_link_target;
use crate::rendering::layout::PAD_TOP;
use crate::rendering::text_overlay::{offset_at_point, PageView};
use crate::Reader;

use super::LoopContext;

/// Follow a link if the tap landed on one. Returns true when the tap was
/// consumed, so the caller must not also treat it as a page turn or a
/// panel toggle.
pub(super) fn try_follow_link(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    ctx: &mut LoopContext,
    tap_x: f32,
    tap_y: f32,
) -> bool {
    if st.picker_active || st.panel_open || st.chapters.is_empty() {
        return false;
    }
    if tap_x < 0.0 || tap_y < 0.0 {
        return false;
    }
    let pv = PageView {
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
    let offset = match offset_at_point(&pv, tap_x as usize, tap_y as usize) {
        Some(o) => o,
        None => return false,
    };
    let href = match st.state.link_at(offset) {
        Some(l) => l.href.clone(),
        None => return false,
    };
    let Some(target) =
        kobo_core::formats::epub::resolve_link(&st.chapters, st.current_chapter, &href)
    else {
        log::info!("link: {href} does not resolve inside this book");
        return false;
    };
    log::info!("link: {href} -> chapter {}", target.chapter);
    // A footnote-style in-text link keeps the current audio queue rather than
    // reloading it -- unlike an explicit chapter-overlay open (see
    // `panel::callbacks::chapters`), which does.
    jump_to_link_target(st, reader, cmd_tx, &target, false);
    st.text_dirty = true;
    ctx.window.request_redraw();
    true
}
