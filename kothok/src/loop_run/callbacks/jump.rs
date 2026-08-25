// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::loop_state::LoopState;
use crate::Reader;

use super::super::{
    apply_page, best_effort_send, build_state, load_page_audio, set_chapter_name, switch_chapter,
    ChapterSwitchOpts, LoopContext, ViewMode,
};
use super::bookmark::{page_for_bookmark, restore_cursor_line};

pub(super) fn handle_jump_to_reading(
    st: &mut LoopState,
    reader: &Reader,
    cb: &Callbacks,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    ctx: &mut LoopContext,
) {
    if !cb.jump_to_reading_cell.replace(false) || st.picker_active {
        return;
    }
    st.panel_open = false;
    cb.panel_open_cell.set(false);
    reader.set_panel_open(false);

    if matches!(st.view_mode, ViewMode::Audio) {
        jump_audio_bookmark(st, reader, cmd_tx, ctx);
    } else if st.reading_ch < st.chapters.len() {
        jump_reading_position(st, reader, cmd_tx, ctx);
    }
}

/// The most recently set bookmark that still points into this book, or None.
/// The Vec's last element is the newest by construction; invalid chapter
/// indices (an edited/shorter book) fall back to the next-newest.
pub(super) fn latest_valid_bookmark(
    bms: &[crate::Bookmark],
    chapter_count: usize,
) -> Option<crate::Bookmark> {
    bms.iter()
        .rev()
        .find(|b| b.chapter < chapter_count)
        .copied()
}

/// Jump straight to the most recently set bookmark from the bookmark-jump
/// header button. Switches chapter if needed, lands on the bookmark's page,
/// restores the reading cursor, and - in audio mode - reloads + seeks the
/// driver so the page-break markers match the new layout.
pub(super) fn jump_to_bookmark(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    ctx: &mut LoopContext,
) {
    let Some(bm) = latest_valid_bookmark(&st.bookmarks, st.chapters.len()) else {
        reader.set_status("No bookmark".into());
        return;
    };
    apply_bookmark_jump(st, reader, cmd_tx, ctx, bm);
}

/// Jump to the bookmark stored at `idx` (overlay Bookmarks-tab row or Open
/// button target). Falls back to the most recently set one when `idx` no
/// longer resolves (e.g. the row's bookmark was just deleted).
pub(in crate::loop_run) fn jump_to_bookmark_idx(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    ctx: &mut LoopContext,
    idx: usize,
) {
    let bm = match st.bookmarks.get(idx) {
        Some(bm) if bm.chapter < st.chapters.len() => *bm,
        _ => match latest_valid_bookmark(&st.bookmarks, st.chapters.len()) {
            Some(bm) => bm,
            None => return,
        },
    };
    apply_bookmark_jump(st, reader, cmd_tx, ctx, bm);
}

fn apply_bookmark_jump(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    ctx: &mut LoopContext,
    bm: crate::Bookmark,
) {
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
    restore_cursor_line(st, reader, bm.offset);
    let base = st
        .chapter_offsets
        .get(st.current_chapter)
        .copied()
        .unwrap_or(0);
    reader.set_saved_page((base + st.current_page) as i32);
    if matches!(st.view_mode, ViewMode::Audio) {
        let utts = crate::audio::glue::page_utterances(st.current_page, &st.state);
        let target = crate::audio::glue::utterance_index_for_offset(&utts, bm.offset);
        best_effort_send(cmd_tx, Cmd::Reload(utts));
        best_effort_send(cmd_tx, Cmd::Seek(target));
    }
    st.text_dirty = true;
    ctx.window.request_redraw();
}

fn jump_audio_bookmark(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    ctx: &mut LoopContext,
) {
    let Some(bm) = latest_valid_bookmark(&st.bookmarks, st.chapters.len()) else {
        return;
    };
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
    restore_cursor_line(st, reader, bm.offset);
    let base = st
        .chapter_offsets
        .get(st.current_chapter)
        .copied()
        .unwrap_or(0);
    reader.set_saved_page((base + st.current_page) as i32);
    let utts = crate::audio::glue::page_utterances(st.current_page, &st.state);
    let target = crate::audio::glue::utterance_index_for_offset(&utts, bm.offset);
    best_effort_send(cmd_tx, Cmd::Reload(utts));
    best_effort_send(cmd_tx, Cmd::Seek(target));
    st.text_dirty = true;
    ctx.window.request_redraw();
}

fn jump_reading_position(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &std::sync::mpsc::Sender<Cmd>,
    ctx: &mut LoopContext,
) {
    if st.reading_ch != st.current_chapter {
        st.current_chapter = st.reading_ch;
        st.state = build_state(
            &mut st.chapters[st.reading_ch],
            st.body_px,
            st.head_px,
            st.line_h,
            st.text_justify,
        );
        let cn =
            crate::data::library::chapter_display_title(&st.chapters[st.reading_ch], st.reading_ch);
        set_chapter_name(reader, &cn);
        best_effort_send(cmd_tx, Cmd::Reload(st.state.utterances.clone()));
    }
    if st.reading_off > 0 {
        st.current_page = st
            .state
            .pages
            .iter()
            .enumerate()
            .find(|(_, (rs, re))| {
                st.state
                    .all_rows
                    .get(*rs..*re)
                    .unwrap_or(&[])
                    .iter()
                    .any(|r| r.start as usize <= st.reading_off && r.end as usize > st.reading_off)
            })
            .map(|(i, _)| i)
            .unwrap_or(st.reading_pg);
    } else {
        st.current_page = st.reading_pg;
    }
    st.current_page = st.current_page.min(st.state.pages.len().saturating_sub(1));
    apply_page(
        reader,
        &st.state,
        st.current_page,
        &st.chapter_offsets,
        st.current_chapter,
    );
    if st.reading_off > 0 {
        reader.set_cur_start(st.reading_off as i32);
        reader.set_cur_end(st.reading_end as i32);
    }
    let base = st
        .chapter_offsets
        .get(st.current_chapter)
        .copied()
        .unwrap_or(0);
    reader.set_saved_page((base + st.current_page) as i32);
    load_page_audio(st.current_page, &st.state, cmd_tx);
    st.text_dirty = true;
    ctx.window.request_redraw();
}

#[cfg(test)]
mod tests {
    use super::latest_valid_bookmark;
    use crate::Bookmark;

    fn bm(chapter: usize, offset: usize) -> Bookmark {
        Bookmark {
            chapter,
            page: 0,
            offset,
        }
    }

    #[test]
    fn latest_set_wins() {
        let bms = vec![bm(0, 10), bm(4, 90), bm(2, 50)];
        assert_eq!(latest_valid_bookmark(&bms, 10), Some(bm(2, 50)));
    }

    #[test]
    fn invalid_newest_falls_back_to_next_newest() {
        let bms = vec![bm(0, 10), bm(9, 90)];
        // 9 chapters exist; the newest bookmark points past the book.
        assert_eq!(latest_valid_bookmark(&bms, 9), Some(bm(0, 10)));
    }

    #[test]
    fn empty_list_has_no_target() {
        assert_eq!(latest_valid_bookmark(&[], 10), None);
    }
}
