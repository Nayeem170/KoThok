# Definition of Done: fix-word-list-interaction-issues

## Build
- [ ] `cross build --target armv7-unknown-linux-musleabihf --release -p kothok-app` succeeds with no warnings from new code
- [ ] `cross test -p kothok-app --target armv7-unknown-linux-musleabihf` passes (all tests)

## Convention compliance
- [ ] ASCII-only in all source files (no em dash, smart quotes, unicode arrows)
- [ ] LF line endings (no CRLF) -- verify with `git diff --check`
- [ ] No comments unless explaining non-obvious WHY
- [ ] No fallback implementations (no touch-to-wake, no generic error suppression)
- [ ] Branch named `fix/word-list-interaction-issues`

## Requirement coverage

### R1: Scrollbar for word list and search results
- [ ] `paint_word_list` in `rendering/word_list.rs` calls `paint_scrollbar` -- verify `paint_scrollbar` call present at end of function
- [ ] `paint_search_results` in `rendering/search_results.rs` calls `paint_scrollbar` -- verify call present at end of function
- [ ] `paint_scrollbar` is `pub` in `rendering/chapter_list.rs` and accepts `(buf_bytes, screen_w, screen_h, item_count, scroll)` -- verify signature
- [ ] Scrollbar uses `SB_TRACK_COLOR` 0xD6BA and `SB_THUMB_COLOR` 0x94B2 -- verify constants defined in `chapter_list.rs`

### R2: Selection visibility (inverted pill)
- [ ] `paint_word_list` signature includes `selected_word: usize` parameter -- verify in `word_list.rs`
- [ ] Selected row renders with `INK` fill (0x0000) and `WHITE` text (0xFFFF) -- verify fill/border/fg logic in paint loop
- [ ] Unselected row renders with `WHITE` fill and `INK` text -- verify else branch
- [ ] `app/render.rs` passes `st.search_selected_word` to `paint_word_list` -- verify call site

### R3: Crash fix (UTF-8 + body timing)
- [ ] `open_book` in `data/library.rs` builds `chapter.body` via `build_chapter_body` before calling `build_word_index` in cold path -- verify ordering: body build loop before `build_word_index` call
- [ ] `CACHE_FORMAT` == 6 in `data/library.rs` -- verify constant value
- [ ] `CachedBook` struct in `data/library.rs` has `word_index: WordIndex` field with `#[serde(default)]` -- verify struct definition
- [ ] `save_cached_book` in `data/library.rs` pre-builds body for chapters with empty body -- verify body build logic before serialization
- [ ] `build_snippet` in `rendering/search_results.rs` checks `body.is_char_boundary(start)` and falls back to `body.floor_char_boundary(start)` -- verify char boundary logic
- [ ] `build_snippet` guards `start >= body.len()` -- verify early return

### R4: Cursor fix (jump_to_occurrence)
- [ ] `jump_to_occurrence` in `loop_run/search.rs` scans `all_rows[s..e]` for row where `r.start <= hit.byte_offset && hit.byte_offset < r.end` -- verify row scan logic
- [ ] Fallback to `first_text_row` when no row matches -- verify `unwrap_or_else` branch

## Test coverage
- [ ] `word_list.rs::tests::active_tab_is_inked_and_inactive_is_not` passes -- verifies tab bar pill rendering
- [ ] `word_list.rs::tests::tab_bar_fits_left_of_close_button` passes -- verifies layout bounds
- [ ] `word_list.rs::tests::word_list_hit_test_*` tests pass (4 tests) -- verifies hit testing
- [ ] `search_results.rs::tests::results_hit_test_*` tests pass (3 tests) -- verifies result hit testing
- [ ] `word_index.rs::tests::byte_offsets_match_body_positions` passes -- verifies offset correctness
- [ ] `word_index.rs::tests::nth_occurrence_matches_nth_appearance_in_body` passes -- verifies all offsets

## Scope
- [ ] No changes outside plan.md scope
- [ ] No draggable scrollbar interaction added
- [ ] No changes to chapter list row selection visual
- [ ] No touch-to-wake or fallback mechanisms added
