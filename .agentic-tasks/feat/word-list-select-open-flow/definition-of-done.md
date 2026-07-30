# Definition of Done: feat/word-list-select-open-flow

## Build

- [ ] `cross build --target armv7-unknown-linux-musleabihf --release -p kothok-app` from `kothok/` compiles with zero errors
- [ ] `cross test -p kothok-app --target armv7-unknown-linux-musleabihf` passes all tests (kothok-app only)

## Convention Compliance

- [ ] No comments added (unless explicitly requested)
- [ ] ASCII-only: no em dashes, en dashes, smart quotes, unicode arrows in source
- [ ] LF line endings on all changed files
- [ ] No new dependencies added

---

## R1: Select-then-open flow

### R1.1: Word tap selects without opening results

- [ ] `loop_run/search.rs`: `handle_search_release` word-tap branch (around line 71-86) sets `st.search_selected_word = idx` but does NOT set `st.search_results_active = true`
- [ ] After tapping a word row, the word list repaints with the tapped row highlighted (inverted fill)
- [ ] `st.search_results_active` remains `false` after word tap

### R1.2: Open button activates search results

- [ ] When Words tab has a selected word and the Slint Open button is tapped, `st.search_results_active` becomes `true`
- [ ] `st.search_results_scroll` resets to 0
- [ ] The overlay stays open (does not close)
- [ ] Search results display for the selected word

### R1.3: Back from results preserves selection

- [ ] `loop_run/search.rs` back-arrow handler (lines 26-33): sets `st.search_results_active = false`, `st.search_results_scroll = 0` but does NOT clear `st.search_selected_word`
- [ ] After returning from results, the word list shows the previously selected word still highlighted
- [ ] Tapping "Open" again re-opens results for the same word

### R1.4: Selection sentinel

- [ ] `LoopState.search_word_selected: bool` field exists (default false) in `loop_state.rs`
- [ ] `loop_state.rs` also contains `sb_dragging: bool` (default false) and `sb_drag_tab: ChapterTab` (default Chapters)
- [ ] `setup/loop_init.rs` initializes all three new fields: `sb_dragging: false`, `sb_drag_tab: ChapterTab::default()`, `search_word_selected: false`
- [ ] `paint_word_list` in `rendering/word_list.rs` accepts a `word_selected: bool` parameter and only highlights when `word_selected && i == selected_word`
- [ ] `app/render.rs:161-167` passes `st.search_word_selected` to `paint_word_list`

---

## R2: Draggable scrollbar

### R2.1: Scrollbar hit test function exists

- [ ] `rendering/chapter_list.rs` exports `pub fn scrollbar_hit_test(dx: f32, dy: f32, w: usize) -> bool`
- [ ] Returns true when `dx` is within the scrollbar track column (`w - SB_TRACK_PAD - SB_TRACK_W .. w - SB_TRACK_PAD`) and `dy` is within the list bounds (`CH_LIST_TOP .. h - CH_LIST_BOTTOM_PAD`)

### R2.2: Scroll position from finger Y

- [ ] `rendering/chapter_list.rs` exports `pub fn scrollbar_y_to_scroll(finger_y: i32, item_count: usize) -> i32`
- [ ] The formula inverts `paint_scrollbar`'s thumb position: `max_travel = track_h - thumb_h`, `finger_y_fraction = ((finger_y - list_top) - thumb_h/2) / max_travel` clamped to `[0.0, 1.0]`, `scroll = (finger_y_fraction * scroll_max).round()` where `scroll_max = total_h - visible_h`
- [ ] Result is clamped to `[0, scroll_max]`

### R2.3: Scrollbar visibility

- [ ] `rendering/chapter_list.rs` exports `pub fn scrollbar_visible(item_count: usize) -> bool`
- [ ] Returns false when all items fit in the viewport: `item_count * CH_ROW_PITCH <= list_height`
- [ ] `paint_scrollbar` uses this (or equivalent inline check) to skip drawing when hidden

### R2.4: Touch-down on scrollbar starts drag

- [ ] `loop_run/touch_dispatch.rs`: In the press branch (around line 106-205), when overlay is open and `scrollbar_hit_test` returns true, sets `st.sb_dragging = true`, records `st.sb_drag_tab = st.chapter_tab`, and sets `press_dispatched = false` (preventing `PointerPressed` from being dispatched to Slint, which would cause the Open button to activate on release)
- [ ] On touch-down in scrollbar zone, the list immediately scrolls to the finger position

### R2.5: Touch-move while dragging updates scroll

- [ ] `loop_run/touch_dispatch.rs`: In the move branch (lines 239-278), when `st.sb_dragging` is true, computes scroll from current finger Y using `scrollbar_y_to_scroll`
- [ ] Updates the correct scroll field based on `st.sb_drag_tab`:
  - `ChapterTab::Chapters` -> `st.chapter_scroll`
  - `ChapterTab::Words` + `!search_results_active` -> `st.search_scroll`
  - `ChapterTab::Words` + `search_results_active` -> `st.search_results_scroll`
- [ ] Sets `st.text_dirty = true` and requests redraw

### R2.6: Touch-up ends drag

- [ ] `loop_run/touch_release.rs`: When `st.sb_dragging` is true on release, clears `st.sb_dragging = false` and returns early (does NOT pass through to list-row hit test or chapter hit test)
- [ ] List remains at the new scroll position after release

### R2.7: Scrollbar state fields exist in LoopState

- [ ] `loop_state.rs` contains `sb_dragging: bool` (default false)
- [ ] `loop_state.rs` contains `sb_drag_tab: ChapterTab` (default Chapters)
- [ ] `setup/loop_init.rs` initializes both fields in the `LoopState` struct literal

### R2.8: Scrollbar excluded from list hit tests

- [ ] When `st.sb_dragging` is true, `handle_search_release` and `chapter_overlay_target` do not process the release as a row tap
- [ ] Taps on the scrollbar zone that do not move (tap-down + tap-up without move) still jump the list to that position

### R2.9: Scrollbar hidden when content fits

- [ ] For a word list with <= ~19 words (fits viewport), no scrollbar is drawn and no scrollbar hit zone is active
- [ ] For a chapter list with <= ~19 rows, same behavior

---

## R3: Open button consistency

### R3.1: Slint callback signature change

- [ ] `chapter_overlay.slint` line 13: `callback chapter-selected(int)` changed to `callback chapter-selected(int, int)` -- second argument is tab (0=Chapters, 1=Words)
- [ ] `reader.slint` line 102: same signature change `callback chapter-selected(int, int)`
- [ ] `reader.slint` line 266: forwarding changed to `root.chapter-selected(i, tab_value)` passing both arguments
- [ ] `chapter_overlay.slint` line 74: Open button `clicked` handler passes `(idx, active-tab)` to the callback
- [ ] Rust code sets the `active-tab` property on `ChapterOverlay` when switching tabs (search.rs tab-switch paths)

### R3.2: Open button callback routes by tab

- [ ] `callbacks.rs` `on_chapter_selected` handler (line 66-78) accepts `(idx: i32, tab: i32)`:
  - `tab == 0` (Chapters): existing behavior (close overlay, set `chapter_select_cell`)
  - `tab == 1` (Words): set `word_open_cell` to true, do NOT close overlay
- [ ] `word_open_cell: Rc<Cell<bool>>` added to `Callbacks` struct

### R3.3: Word open signal processed in loop callbacks

- [ ] `loop_run/callbacks.rs` (or equivalent) processes `word_open_cell`: when true, sets `st.search_results_active = true`, `st.search_results_scroll = 0`, `st.text_dirty = true`
- [ ] Returns `true` for `ui_changed` so a repaint happens

### R3.4: Chapters tab behavior unchanged

- [ ] Tapping a chapter row still calls `reader.set_chapter_preview_idx(idx)` (touch_release.rs:168-173)
- [ ] Tapping Open on chapters tab still fires `chapter_select_cell` and closes overlay
- [ ] No regression in the existing chapters flow

### R3.5: Tab value passed on tab switch

- [ ] `loop_run/search.rs` tab-switch paths (lines 49-62): when switching to Chapters tab, set `active-tab` property to 0; when switching to Words tab, set `active-tab` property to 1
- [ ] The `active-tab` value on `ChapterOverlay` matches the Rust-side `st.chapter_tab` at all times

---

## MEDIUM 2: Sentinel propagation notes

- [ ] `loop_run/search.rs` `jump_to_occurrence` (line 92): no sentinel guard needed -- this function is only reachable when `search_results_active == true`, which implies a word is selected
- [ ] `app/render.rs` (line 135, 141): `st.search_selected_word` index reads for results word/hits lookup remain unchanged (gated by `st.search_results_active`)
- [ ] `loop_run/touch_dispatch.rs` (line 248): `st.search_selected_word` read for move-during-results remains unchanged

---

## Test Coverage

- [ ] `scrollbar_hit_test_inside_track_returns_true` -- hit test returns true for a point in the scrollbar track column within list bounds
- [ ] `scrollbar_hit_test_outside_track_returns_false` -- hit test returns false for a point left of the scrollbar column
- [ ] `scrollbar_hit_test_above_list_returns_false` -- hit test returns false above CH_LIST_TOP
- [ ] `scrollbar_hit_test_below_list_returns_false` -- hit test returns false below list_bottom
- [ ] `scrollbar_y_to_scroll_top_returns_zero` -- finger at list top -> scroll 0
- [ ] `scrollbar_y_to_scroll_bottom_returns_max` -- finger at list bottom -> scroll_max
- [ ] `scrollbar_y_to_scroll_midpoint_returns_half` -- finger at midpoint -> approximately half of scroll_max
- [ ] `scrollbar_visible_few_items_returns_false` -- item_count that fits viewport -> false
- [ ] `scrollbar_visible_many_items_returns_true` -- item_count exceeding viewport -> true
- [ ] `word_tap_selects_does_not_open_results` -- unit test verifying the search release sets selected_word but not results_active
- [ ] `back_from_results_preserves_selection` -- unit test verifying back clears results_active but not selected_word

---

## Scope Boundaries

- [ ] No changes to search results scrollbar interaction (visual-only remains for results in this pass)
- [ ] No changes to the Slint header text ("Chapters" title)
- [ ] No new Slint components created
- [ ] No changes to the close (X) button behavior
- [ ] No changes to the panel, audio mode, picker, or reading-mode interactions
- [ ] No new crates or external dependencies
