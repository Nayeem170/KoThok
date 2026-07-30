# Test Plan: fix-word-list-interaction-issues

## Scope

Tests for scrollbar rendering, selection visuals, crash fixes (UTF-8 safety + empty body), cursor positioning in `jump_to_occurrence`, and edge cases across `word_list.rs`, `search_results.rs`, `word_index.rs`, and `search.rs`.

## Test Scenarios

### TS-01: Scrollbar appears when word list exceeds viewport

- **Given**: a word list with more rows than fit between `CH_LIST_TOP` and screen bottom minus `CH_LIST_BOTTOM_PAD`
- **When**: `paint_word_list` is called with `scroll = 0`
- **Expected**: the rightmost column at `screen_w - SB_TRACK_PAD - SB_TRACK_W` contains non-white pixels (track + thumb)
- **Notes**: `paint_scrollbar` is called internally by `paint_word_list`; the track color is `SB_TRACK_COLOR` (0xD6BA) and thumb is `SB_THUMB_COLOR` (0x94B2)

### TS-02: Scrollbar thumb position reflects scroll offset

- **Given**: a word list with enough rows to require scrolling
- **When**: `paint_word_list` is called with `scroll = max_scroll / 2`
- **Expected**: the thumb is positioned approximately in the middle of the track (between `list_top` and `list_bottom`)
- **Notes**: verify by scanning the `SB_TRACK_W`-wide column for `SB_THUMB_COLOR` pixels and checking their vertical center

### TS-03: Scrollbar thumb size reflects visible-to-total ratio

- **Given**: 100 words in the list, viewport shows ~8 rows
- **When**: `paint_word_list` is called
- **Expected**: the thumb height is roughly 8% of the track height (visible/total fraction)
- **Notes**: `frac = visible_h / total_h`, `thumb_h = ceil(frac * track_h)`; check that the thumb occupies much less than the full track

### TS-04: Scrollbar absent when list fits in viewport

- **Given**: a word list with 3 words (all visible without scrolling)
- **When**: `paint_word_list` is called
- **Expected**: the scrollbar column at the right edge remains white (no track or thumb drawn)
- **Notes**: `paint_scrollbar` returns early when `item_count <= 1`; verify it also returns for `item_count` that fits in `visible_h`

### TS-05: Scrollbar appears on search results when results exceed viewport

- **Given**: search results with more rows than fit the viewport
- **When**: `paint_search_results` is called
- **Expected**: the scrollbar track and thumb are visible in the same right-edge column
- **Notes**: same `paint_scrollbar` function, different `item_count` (capped at `MAX_SEARCH_RESULTS`)

### TS-06: Selected word row renders with ink fill and white text

- **Given**: a word list with 5 words, `selected_word = 2`
- **When**: `paint_word_list` is called
- **Expected**: row index 2 has predominantly dark pixels (INK = 0x0000) as fill, and the text pixels are white (0xFFFF) or near-white
- **Notes**: use the `dark_frac` pattern from existing tab-bar tests to measure fill color

### TS-07: Unselected word rows render with white fill and dark text

- **Given**: a word list with 5 words, `selected_word = 2`
- **When**: `paint_word_list` is called
- **Expected**: rows 0, 1, 3, 4 have predominantly white fill and dark text pixels
- **Notes**: measure `dark_frac` on non-selected rows; it should be low (fill) but non-zero (text)

### TS-08: Tab switch clears selection visual

- **Given**: word "alpha" is at index 3, `selected_word = 3`
- **When**: the user switches to Chapters tab and back to Words tab
- **Expected**: `search_selected_word` is preserved (or reset); whichever value is used, the rendered selection matches the current `selected_word` parameter exactly
- **Notes**: this is a rendering correctness test -- the painted output must always match the `selected_word` argument

### TS-09: Out-of-bounds selected_word shows no selection

- **Given**: a word list with 5 words, `selected_word = 999`
- **When**: `paint_word_list` is called
- **Expected**: no row is rendered with ink fill (all rows are white fill); the comparison `i == selected_word` never matches a valid row index
- **Notes**: the `i == selected_word` check means an out-of-range index simply never matches, so all rows render as unselected

### TS-10: Bengali text (multi-byte) does not crash build_snippet

- **Given**: a chapter whose body is Bengali text (e.g., `"বাংলা ভাষা বাক্য"`, all multi-byte UTF-8)
- **When**: `build_snippet` is called with a `WordHit` whose `byte_offset` points into the Bengali body
- **Expected**: the function returns a non-panicking `String` result (may be empty or contain truncated Bengali)
- **Notes**: `is_char_boundary` + `floor_char_boundary` must not panic on mid-byte offsets

### TS-11: Empty chapter body returns empty snippet

- **Given**: a chapter with `body = ""`
- **When**: `build_snippet` is called with any `WordHit` referencing this chapter
- **Expected**: the function returns an empty `String` (`""`)
- **Notes**: the early return on `body.is_empty()` prevents any indexing

### TS-12: Stale offset beyond body length returns empty snippet

- **Given**: a chapter with `body = "hello"` (len 5)
- **When**: `build_snippet` is called with `byte_offset = 100` (well past end)
- **Expected**: the function returns an empty `String`; no panic on out-of-range indexing
- **Notes**: `floor_char_boundary` clamps to body length, then `start >= body.len()` returns empty

### TS-13: build_snippet with offset at a multi-byte char boundary

- **Given**: a chapter body `"a\xC3\xA9b"` where `\xC3\xA9` is e-acute (2 bytes)
- **When**: `build_snippet` is called with `byte_offset = 2` (boundary) and `byte_offset = 1` (mid-byte)
- **Expected**: offset 2 produces a snippet starting with the correct char; offset 1 floors to 2 and also succeeds
- **Notes**: validates the `is_char_boundary` then `floor_char_boundary` path

### TS-14: jump_to_occurrence sets cursor to row containing hit offset

- **Given**: a chapter with pages where `all_rows` has rows with `(start, end)` ranges, and a `WordHit` whose `byte_offset` falls within row 3's range
- **When**: `jump_to_occurrence` is called with that hit
- **Expected**: `reader.set_cur_start` and `reader.set_cur_end` are called with row 3's `(start, end)`
- **Notes**: the row scan `r.start <= offset < r.end` must find the enclosing row

### TS-15: jump_to_occurrence falls back when no row matches

- **Given**: a chapter where `all_rows` has gaps (no row covers the hit's byte_offset)
- **When**: `jump_to_occurrence` is called
- **Expected**: cursor falls back to `first_text_row` for the target page; no panic
- **Notes**: the `.unwrap_or_else(|| first_text_row(...))` fallback path

### TS-16: jump_to_occurrence switches to a different chapter

- **Given**: the current chapter is 0, the hit references chapter 3
- **When**: `jump_to_occurrence` is called
- **Expected**: `switch_chapter` is called with `target_ch = 3`; the page, cursor, and audio are all updated for chapter 3
- **Notes**: the `target_ch != st.current_chapter` branch must fire

### TS-17: Empty word list shows "No searchable text"

- **Given**: `words` is an empty slice
- **When**: `paint_word_list` is called
- **Expected**: the list area shows centered text "No searchable text"; no scrollbar is drawn; no rows are painted
- **Notes**: `paint_empty_message` is called and returns early before `paint_scrollbar`

### TS-18: Empty search results render without crash

- **Given**: `hits` is an empty slice, `total_hits = 0`
- **When**: `paint_search_results` is called
- **Expected**: the header shows `"word" - 0 matches`; no result rows are painted; scrollbar is absent (0 items)
- **Notes**: `paint_scrollbar` with `display_count = 0` returns early

### TS-19: Very long word list (1000+ words) scrolls correctly

- **Given**: a word list with 1500 words
- **When**: `paint_word_list` is called with various scroll values (0, max, middle)
- **Expected**: visible rows show the correct subset of words; scrollbar thumb is proportionally tiny; no out-of-bounds panic
- **Notes**: verifies integer overflow safety in scroll calculations and hit-test

### TS-20: Chapter with only images produces empty word index

- **Given**: a chapter whose body is empty (images stripped during `build_chapter_body`)
- **When**: `build_word_index` is called
- **Expected**: the index contains no words from that chapter; the word list renders correctly (possibly showing words from other chapters only)
- **Notes**: existing test `image_only_chapter_produces_empty_index` covers the index; this extends to the rendering path

### TS-21: jump_to_occurrence reloads audio for the target page

- **Given**: chapter with pages containing utterances, a `WordHit` on page 2
- **When**: `jump_to_occurrence` is called
- **Expected**: `Cmd::Reload` is sent with utterances from page 2; `Cmd::Seek` is sent with an utterance index matching the hit offset (via `utterance_index_for_offset`)

### TS-22: paint_scrollbar not drawn for single-item list

- **Given**: exactly 1 word in the list that fits in the viewport
- **When**: `paint_scrollbar` is called with `item_count=1`
- **Expected**: no scrollbar is rendered (early return in `paint_scrollbar`)

### TS-23: word_list_hit_test returns None when scroll exceeds max

- **Given**: 5 words, `scroll = 1000`
- **When**: tapping at `CH_LIST_TOP`
- **Expected**: returns `None` (index out of range)

### TS-24: search results capped at MAX_SEARCH_RESULTS

- **Given**: 250 hits for a word
- **When**: `paint_search_results` is called
- **Expected**: only 200 rows are painted; the `"and 50 more..."` message is shown

### TS-25: build_snippet with out-of-range chapter returns empty

- **Given**: 3 chapters, `hit.chapter = 99`
- **When**: `build_snippet` is called
- **Expected**: returns empty string
