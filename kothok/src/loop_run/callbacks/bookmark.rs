// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use log::{info, warn};

use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::loop_state::LoopState;
use crate::Reader;

use super::super::{apply_page, best_effort_send, switch_chapter, ChapterSwitchOpts};

pub(super) fn handle_bookmark_set(st: &mut LoopState, reader: &Reader, cb: &Callbacks) -> bool {
    if !cb.bookmark_set_cell.replace(false) || st.picker_active {
        return false;
    }
    let cur = reader.get_cur_start().max(0) as usize;
    let cursor_on_page = page_for_offset(st, cur) == Some(st.current_page);
    // Anchor the bookmark to the live TTS line when playing, or to a cursor
    // that still sits on this page (paused mid-sentence). Otherwise the
    // cursor is stale -- manual page turns preserve the old cursor -- so fall
    // back to the first line of the current page.
    let off = if reader.get_playing() || cursor_on_page {
        cur
    } else {
        first_text_row_offset_on_page(st).unwrap_or(cur)
    };
    // Without active playback there is no sentence band to show the mark, so
    // place the visible reading cursor on the bookmarked line. Otherwise the
    // progress bar moves but the page gives no sign of where it landed.
    if !reader.get_playing() {
        restore_cursor_line(st, reader, off);
    }
    st.bookmark = Some(crate::Bookmark {
        chapter: st.current_chapter,
        page: st.current_page,
        offset: off,
    });
    reader.set_has_bookmark(true);
    let global_page = st
        .chapter_offsets
        .get(st.current_chapter)
        .copied()
        .unwrap_or(0)
        + st.current_page;
    reader.set_status(format!("Bookmarked page {}", global_page + 1).into());
    info!(
        "bookmark-set: ch={} pg={} off={} playing={} on_page={}",
        st.current_chapter + 1,
        st.current_page + 1,
        off,
        reader.get_playing(),
        cursor_on_page,
    );
    true
}

/// Byte offset of the first text-bearing row on the current page. Used as the
/// bookmark anchor when the reading cursor is stale or absent (not playing).
fn first_text_row_offset_on_page(st: &LoopState) -> Option<usize> {
    let (s, e) = st.state.pages.get(st.current_page)?;
    crate::reader::first_text_row(&st.state, *s, *e).map(|(start, _)| start as usize)
}

pub(super) fn handle_bookmark_jump(
    st: &mut LoopState,
    reader: &Reader,
    cb: &Callbacks,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
) -> bool {
    if !cb.bookmark_jump_cell.replace(false) || st.picker_active {
        return false;
    }
    if let Some(bm) = st.bookmark {
        if bm.chapter >= st.chapters.len() {
            st.bookmark = None;
            reader.set_has_bookmark(false);
            reader.set_status("That bookmark is no longer in this book".into());
            warn!(
                "bookmark-jump: ch {} out of range ({} chapters), cleared",
                bm.chapter,
                st.chapters.len()
            );
        } else {
            if bm.chapter != st.current_chapter {
                switch_chapter(
                    st,
                    reader,
                    cmd_tx,
                    bm.chapter,
                    ChapterSwitchOpts {
                        to_last_page: false,
                        update_cursor: false,
                        load_audio: true,
                    },
                );
            }
            st.current_page = page_for_bookmark(st, &bm);
            apply_page(
                reader,
                &st.state,
                st.current_page,
                &st.chapter_offsets,
                st.current_chapter,
            );
            let restored = restore_cursor_line(st, reader, bm.offset);
            let base = st
                .chapter_offsets
                .get(st.current_chapter)
                .copied()
                .unwrap_or(0);
            reader.set_saved_page((base + st.current_page) as i32);
            if matches!(st.view_mode, crate::ViewMode::Audio) {
                // Audio mode: the full chapter is already queued (by
                // switch_chapter or a prior load). Seek to the utterance
                // matching the bookmark offset within the chapter -- do NOT
                // reload, which would replace the chapter with one page.
                let chapter_utts = crate::audio::glue::chapter_utterances(&st.state);
                let target =
                    crate::audio::glue::utterance_index_for_offset(&chapter_utts, bm.offset);
                best_effort_send(cmd_tx, Cmd::Seek(target));
            } else {
                let utts = crate::audio::glue::page_utterances(st.current_page, &st.state);
                let target = crate::audio::glue::utterance_index_for_offset(&utts, bm.offset);
                best_effort_send(cmd_tx, Cmd::Reload(utts));
                best_effort_send(cmd_tx, Cmd::Seek(target));
            }
            st.text_dirty = true;
            reader.set_status(
                if restored {
                    format!("Back to page {}", base + st.current_page + 1)
                } else {
                    format!("Back to page {} (line moved)", base + st.current_page + 1)
                }
                .into(),
            );
            info!(
                "bookmark-jump: ch={} pg={} off={} line_restored={} audio_mode={}",
                bm.chapter + 1,
                st.current_page + 1,
                bm.offset,
                restored,
                matches!(st.view_mode, crate::ViewMode::Audio),
            );
        }
    } else {
        reader.set_status("No bookmark yet - tap the ribbon to set one".into());
    }
    true
}

/// Which page a bookmark now lives on.
///
/// A bookmark stores a chapter, a page and a character offset, but only the
/// offset is stable: changing the font size repaginates the chapter, so the
/// stored page number then points somewhere else entirely. Trusting it meant
/// that after a font change the jump landed on the wrong page, the cursor line
/// could not be found there, and playback fell back to the top of the page.
///
/// So the page is derived from the offset whenever the offset can still be
/// located, and the stored page is used only as a fallback -- for a bookmark
/// whose text has genuinely gone (an edited book), where landing near where it
/// used to be beats landing at the chapter start.
pub(super) fn page_for_bookmark(st: &LoopState, bm: &crate::Bookmark) -> usize {
    page_for_offset(st, bm.offset).unwrap_or_else(|| {
        let npages = st.state.pages.len();
        if npages > 0 {
            bm.page.min(npages - 1)
        } else {
            0
        }
    })
}

/// Index of the page whose rows cover `offset`, if any.
pub(super) fn page_for_offset(st: &LoopState, offset: usize) -> Option<usize> {
    st.state.page_for_offset(offset)
}

pub(super) fn restore_cursor_line(st: &LoopState, reader: &Reader, offset: usize) -> bool {
    let mut restored = false;
    if let Some((s, e)) = st.state.pages.get(st.current_page) {
        if let Some(rows) = st.state.all_rows.get(*s..*e) {
            for row in rows {
                if row.start < row.end && offset >= row.start as usize && offset < row.end as usize
                {
                    reader.set_cur_start(row.start);
                    reader.set_cur_end(row.end);
                    restored = true;
                    break;
                }
            }
            if !restored {
                for row in rows {
                    if row.start < row.end {
                        reader.set_cur_start(row.start);
                        reader.set_cur_end(row.end);
                        break;
                    }
                }
            }
        }
    }
    restored
}
