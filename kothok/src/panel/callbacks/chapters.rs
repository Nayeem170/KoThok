// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::sync::mpsc::Sender;

use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::loop_state::LoopState;
use crate::Reader;

pub(super) fn handle_chapter_overlay(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cb: &Callbacks,
) -> bool {
    let mut text_dirty = false;

    if cb.chapter_panel_cell.replace(false) && !st.picker_active {
        reader.set_current_chapter_idx(st.current_chapter as i32);
        reader.set_chapter_overlay_open(true);
    }

    if cb.word_open_cell.replace(false) && !st.picker_active && st.search_word_selected {
        st.search_results_active = true;
        st.search_results_scroll = 0;
        text_dirty = true;
    }

    // `nc` is a flattened TOC row index, not a spine chapter index (issue 11).
    // Resolve it to the chapter it names and, if it carries a `#fragment`, the
    // page within that chapter -- `jump_to_link_target` is the same
    // switch + page_for_offset + apply_page sequence a tapped hyperlink
    // follows (`loop_run::link_nav`); a TOC "open" and an in-text link both
    // resolve a `(chapter, Option<anchor>)` pair to a page identically, so the
    // sequence is shared there rather than duplicated here.
    if let Some(nc) = cb.chapter_select_cell.replace(None) {
        let (target_ch, anchor) = match st.toc_rows.get(nc) {
            Some(row) => (row.chapter, row.anchor.clone()),
            None => return text_dirty,
        };
        let Some(target_ch) = target_ch else {
            // A divider/grouping entry names no real chapter: shown, not jumped.
            return text_dirty;
        };
        if target_ch >= st.chapters.len() {
            return text_dirty;
        }
        let target = kobo_core::formats::epub::LinkTarget {
            chapter: target_ch,
            anchor,
        };
        crate::reader::jump_to_link_target(st, reader, cmd_tx, &target, true);
        text_dirty = true;
        let cn = crate::data::library::chapter_display_title(&st.chapters[target_ch], target_ch);
        crate::set_chapter_name(reader, &cn);
    }

    text_dirty
}
