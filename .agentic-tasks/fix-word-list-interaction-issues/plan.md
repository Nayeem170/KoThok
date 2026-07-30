# Plan: fix-word-list-interaction-issues

## Summary

Fix three interaction issues with the Words tab of the chapter overlay:
add scrollbars to the word list and search results, make the selected word
row visually distinct with an inverted pill style, and fix crashes caused by
empty chapter body at index-build time and unsafe UTF-8 slicing in snippet
extraction.

## Design decisions (user-approved)

| Decision | Options considered | User's choice | Rationale |
|----------|-------------------|---------------|-----------|
| Scrollbar interaction | drag-to-seek vs visual-only | visual-only (matching chapter list) | chapter list precedent; swipe already works; drag adds touch complexity |
| Scrollbar location | shared function vs per-caller | shared `paint_scrollbar` in `chapter_list.rs` | both lists share layout constants and colors |
| Selection style | border-only vs inverted pill vs accent fill | inverted pill (ink fill + white text) | matches tab bar visual language; border-only invisible at this row size |
| Crash fix: body timing | defer index build vs pre-build body | pre-build body before index | offsets must reference real body text; cache stores built body |
| Crash fix: UTF-8 safety | panic on mid-byte vs floor_char_boundary | `is_char_boundary` + `floor_char_boundary` | safe fallback for non-ASCII text |
| Cursor positioning | `first_text_row` vs row containing hit | scan rows for `start <= offset < end` | cursor must point at the actual hit location |

## Architecture

The Words tab lives inside the chapter overlay alongside the Chapters tab.
Both tabs share the same overlay layout (`CH_LIST_TOP`, `CH_LIST_BOTTOM_PAD`,
row dimensions). The rendering pipeline:

1. `app/render.rs` checks `st.chapter_tab` and `st.search_results_active` to
   decide which painter to call.
2. For the Words tab: `paint_tab_bar` + `paint_word_list` (with scrollbar).
3. Tapping a word sets `search_selected_word`, opens `search_results_active`,
   which routes to `paint_search_results` (with scrollbar).
4. Tapping a result calls `jump_to_occurrence`: switch chapter if needed,
   find the page containing the byte offset, apply page, set cursor to the
   row containing the hit, sync audio.

Data flow: `open_book` builds chapter bodies, then `build_word_index`, then
caches both. The cache (`CACHE_FORMAT` v6) stores chapters (with body),
`toc_tree`, and `word_index` together so cache hits skip all build work.

## Files to create

| Path | Purpose |
|------|---------|
| (none) | All changes are to existing files |

## Files to modify

| Path | Change |
|------|--------|
| `rendering/chapter_list.rs` | Extract `paint_scrollbar` as `pub` shared function; add scrollbar color constants (`SB_TRACK_COLOR`, `SB_THUMB_COLOR`, etc.) |
| `rendering/word_list.rs` | Add selected-row inverted pill rendering in `paint_word_list`; add `selected_word` parameter; call `paint_scrollbar`; add tab bar pill constants (`TAB_BORDER`, `INK`) |
| `rendering/search_results.rs` | Call `paint_scrollbar` after painting result rows; fix `build_snippet` UTF-8 char boundary with `is_char_boundary` + `floor_char_boundary` |
| `rendering/mod.rs` | Add `pub mod search_results;` and `pub mod word_list;` |
| `data/library.rs` | Pre-build `chapter.body` via `build_chapter_body` before `build_word_index` in cold path of `open_book`; bump `CACHE_FORMAT` to 6; pre-build body in `save_cached_book`; add `word_index` field to `CachedBook` |
| `data/word_index.rs` | No changes needed (already uses `chapter.body`) |
| `loop_run/search.rs` | Fix `jump_to_occurrence` cursor: scan `all_rows[s..e]` for row containing `hit.byte_offset` instead of using `first_text_row` |
| `app/render.rs` | Pass `st.search_selected_word` to `paint_word_list` call |

## Dependencies to add

none

## Out of scope

- Draggable scrollbar interaction (touch-down/move/up on the scrollbar rail)
- Scrollbar in the chapter list itself (already absent; not requested)
- Changes to the chapter list row selection visual (border-only is adequate there)
- Word frequency counts, fuzzy search, or search-as-you-type
- Touch-to-wake as a fallback for any crash scenario (fix root cause instead)
- Audio/layout sync changes (this task does not call `build_state()`)

## Risk assessment

| Risk | Mitigation |
|------|-----------|
| `build_chapter_body` adds cost to cold open | Only runs when body is empty (cache miss); cached body skips this entirely |
| `CACHE_FORMAT` bump invalidates all existing caches | Expected and correct; stale cache would lack `word_index` field and crash on deserialize |
| Non-ASCII body text could still produce mid-boundary offsets | `floor_char_boundary` always returns a valid boundary; `unicode_segmentation` splits on grapheme clusters which are always boundary-aligned in Rust strings |
| Scrollbar draw overflows | `paint_scrollbar` uses `saturating_sub`, `clamp`, and buffer length guards |
| Row scan in `jump_to_occurrence` finds no match | Falls back to `first_text_row` |

## Best practices reference

No external references. Design follows existing patterns in the chapter list
scrollbar (color constants, visual-only approach) and the tab bar (inverted
pill visual language from `paint_tab_button`).
