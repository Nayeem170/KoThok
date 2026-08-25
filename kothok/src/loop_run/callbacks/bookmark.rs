// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use log::info;

use crate::callbacks::Callbacks;
use crate::loop_state::LoopState;
use crate::Reader;

pub(super) fn handle_bookmark_set(st: &mut LoopState, reader: &Reader, cb: &Callbacks) -> bool {
    if !cb.bookmark_set_cell.replace(false) || st.picker_active {
        return false;
    }
    let cur = reader.get_cur_start().max(0) as usize;
    let cursor_on_page = page_for_offset(st, cur) == Some(st.current_page);
    let off = if reader.get_playing() || cursor_on_page {
        cur
    } else {
        first_text_row_offset_on_page(st).unwrap_or(cur)
    };
    if !reader.get_playing() {
        restore_cursor_line(st, reader, off);
    }
    let added = toggle_bookmark_at(
        &mut st.bookmarks,
        crate::Bookmark {
            chapter: st.current_chapter,
            page: st.current_page,
            offset: off,
        },
    );
    reader.set_has_bookmark(!st.bookmarks.is_empty());
    let global_page = st
        .chapter_offsets
        .get(st.current_chapter)
        .copied()
        .unwrap_or(0)
        + st.current_page;
    let msg = if added {
        format!("Bookmarked page {}", global_page + 1)
    } else {
        "Bookmark removed".to_string()
    };
    reader.set_status(msg.into());
    info!(
        "bookmark-set: ch={} pg={} off={} playing={} on_page={} added={} total={}",
        st.current_chapter + 1,
        st.current_page + 1,
        off,
        reader.get_playing(),
        cursor_on_page,
        added,
        st.bookmarks.len(),
    );
    true
}

/// Toggle a bookmark at `cur`'s spot. Identity is chapter + offset (the page
/// number is a repagination-volatile estimate), so re-setting a bookmark whose
/// page drifted after a font change still removes the original entry. Returns
/// true when the bookmark was added, false when an existing one was removed.
pub(crate) fn toggle_bookmark_at(bms: &mut Vec<crate::Bookmark>, cur: crate::Bookmark) -> bool {
    match bms
        .iter()
        .position(|b| b.chapter == cur.chapter && b.offset == cur.offset)
    {
        Some(i) => {
            bms.remove(i);
            false
        }
        None => {
            bms.push(cur);
            true
        }
    }
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
    cmd_tx: &std::sync::mpsc::Sender<crate::audio::Cmd>,
    ctx: &mut crate::loop_state::LoopContext,
) -> bool {
    if !cb.bookmark_jump_cell.replace(false) || st.picker_active {
        return false;
    }
    super::jump::jump_to_bookmark(st, reader, cmd_tx, ctx);
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
pub(crate) fn page_for_offset(st: &LoopState, offset: usize) -> Option<usize> {
    st.state.page_for_offset(offset)
}

pub(crate) fn restore_cursor_line(st: &LoopState, reader: &Reader, offset: usize) -> bool {
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

#[cfg(test)]
mod tests {
    use super::toggle_bookmark_at;
    use crate::Bookmark;

    fn bm(chapter: usize, page: usize, offset: usize) -> Bookmark {
        Bookmark {
            chapter,
            page,
            offset,
        }
    }

    #[test]
    fn toggle_adds_new_spot_and_removes_existing() {
        let mut bms = vec![bm(0, 5, 100), bm(2, 1, 900)];
        assert!(toggle_bookmark_at(&mut bms, bm(1, 2, 400)));
        assert_eq!(bms.len(), 3);
        // Same spot as an existing entry (page estimate differs): removes it.
        assert!(!toggle_bookmark_at(&mut bms, bm(2, 9, 900)));
        assert_eq!(bms, vec![bm(0, 5, 100), bm(1, 2, 400)]);
    }

    #[test]
    fn identity_is_chapter_and_offset_not_page() {
        let mut bms = vec![bm(1, 4, 500)];
        // Same text spot, repaginated page estimate: still the same bookmark.
        assert!(!toggle_bookmark_at(&mut bms, bm(1, 7, 500)));
        assert!(bms.is_empty());
    }

    #[test]
    fn same_offset_in_other_chapter_is_a_new_bookmark() {
        let mut bms = vec![bm(1, 4, 500)];
        assert!(toggle_bookmark_at(&mut bms, bm(3, 0, 500)));
        assert_eq!(bms.len(), 2);
    }
}
