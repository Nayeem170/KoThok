# Design Decisions: fix-word-list-interaction-issues

## DD-1: Scrollbar -- shared visual indicator, drag-only matching chapter list

Both `paint_word_list` and `paint_search_results` now call `paint_scrollbar`
(extracted into `chapter_list.rs` as a shared function). The scrollbar is a
vertical rail on the right edge of the list area, drawn in the same colors as
the existing chapter list scrollbar (`SB_TRACK_COLOR` 0xD6BA, `SB_THUMB_COLOR`
0x94B2, 6px wide, 10px screen-edge padding).

No drag-to-seek interaction was added. The scrollbar is a visual indicator
only, identical in behavior to the chapter list scrollbar. Drag interaction
was considered but deferred: it adds significant touch-dispatch complexity
(touch-down on the scrollbar, move tracking, touch-up) for a list that is
already swipe-scrollable, and the chapter list established the precedent of
scrollbar-as-indicator-only.

Thumb size is proportional: `frac = visible_height / total_content_height`,
clamped to `[0, 1]`. Thumb travel maps linearly to scroll fraction.

## DD-2: Selection visibility -- inverted pill matching tab bar language

The selected word row uses an inverted pill style that matches the tab bar
buttons (`paint_tab_button` in `word_list.rs`). This was chosen because:

- The tab bar buttons already established the visual language for this
  overlay: active = ink fill (`INK` 0x0000) + white text (`WHITE` 0xFFFF),
  inactive = white fill + ink text, with `TAB_BORDER` (0x2104) as the
  rounded-rect border color.
- The previous state (no visual distinction at all -- every row was a white
  card with a 0x94B2 border) was invisible as a selection signal.
- A thin border-only approach (as used by the chapter list rows) was
  rejected for the word list because the rows are smaller and denser; a
  border-only change does not read as "selected" at this size on the
  Kaleido panel.

`paint_word_list` now takes a `selected_word: usize` parameter. Row `i`
compares `i == selected_word` to decide fill/border/foreground colors.

## DD-3: Crash fix -- body-before-index and char boundary safety

Two distinct crash causes were identified and fixed:

### (a) body built before word index in open_book

`build_word_index` reads `chapter.body` to extract words and record byte
offsets. If `chapter.body` was empty at index-build time (it is populated
lazily by `build_chapter_body` during layout), the index would record no
words -- or worse, if body was populated after indexing from a different
source, offsets would not match.

In `library.rs::open_book`, the cold path now builds `chapter.body` via
`build_chapter_body` for every chapter whose body is empty, **before**
calling `build_word_index`. The hot path (cache hit) does not need this
because the cache already stores the built body.

`CACHE_FORMAT` was bumped from 5 to 6 to invalidate stale caches that lack
the `word_index` field. `save_cached_book` also pre-builds body for any
chapter that has segments but no body, so the cache always stores usable
body text.

### (b) UTF-8 char boundary safety in build_snippet

`build_snippet` in `search_results.rs` slices `chapter.body` at
`hit.byte_offset`. If `byte_offset` lands mid-multibyte (valid for a word
boundary in non-ASCII text where `unicode_words` splits on grapheme
clusters), a direct slice would panic.

The fix: `body.is_char_boundary(start)` check first. If false, fall back to
`body.floor_char_boundary(start)` to find the nearest earlier boundary. An
additional `start >= body.len()` guard handles out-of-range offsets.

## DD-4: Cursor fix -- find row containing hit byte offset

`jump_to_occurrence` (in `loop_run/search.rs`) sets the reading cursor to
the text row that contains the search hit. The previous code used
`first_text_row` which always pointed to the first text row of the target
page, ignoring where on the page the hit actually was.

The fix: after navigating to the correct chapter and page, scan
`st.state.all_rows[s..e]` for the row where
`r.start <= hit.byte_offset && hit.byte_offset < r.end`. If no row matches
(should not happen in practice), fall back to `first_text_row`.
