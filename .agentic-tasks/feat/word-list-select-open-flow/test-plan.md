# Test Plan: feat/word-list-select-open-flow

## Test Location

New unit tests go in existing `#[cfg(test)] mod tests` blocks:
- `rendering/chapter_list.rs` -- `scrollbar_hit_test`, `scrollbar_y_to_scroll`, `scrollbar_visible`
- `rendering/word_list.rs` -- `paint_word_list` selection gating
- `loop_run/search.rs` (integration-style, mocked) -- select-then-open flow, back preserves selection

Integration/EDL tests: manual on-device verification after build+deploy.

---

## R1: Select-then-open flow

### TS-01: Tap word selects without opening results

- **Given:** Words tab active, word list visible with >0 words, `search_word_selected = false`, `search_results_active = false`
- **When:** User taps a word row (finger lands in list area, within `CH_LIST_TOP..list_bottom`, `swipe_dy.abs() <= 40.0`)
- **Expected:**
  - `st.search_selected_word = idx` (the tapped row index)
  - `st.search_word_selected = true`
  - `st.search_results_active` remains `false`
  - `st.text_dirty = true`, redraw requested
  - Word list repaints with tapped row highlighted (inverted fill: `INK` background, `TAB_BORDER` border, white text)
- **Notes:** Existing `word_list_hit_test` at `word_list.rs:208` resolves the index. `handle_search_release` at `search.rs:77-86` must NOT set `search_results_active` on the tap path.
- **Behavior change:** The line at `search.rs:81` that sets `search_results_active = true` must be removed. Currently, any word tap activates results immediately; the new flow requires the separate Open button press (TS-03).

### TS-01b: Default index 0 is NOT highlighted without explicit selection

- **Given:** `search_word_selected = false`, `search_selected_word = 0` (default/initial state)
- **When:** Rendering the word list
- **Expected:**
  - Row 0 is NOT highlighted (no inverted fill)
  - Selection gating in `paint_word_list` checks `search_word_selected` sentinel, not just the index value
- **Notes:** Prevents false highlight on the first row when no user interaction has occurred. The sentinel gate (`search_word_selected`) must be the sole authority for whether a row is highlighted.

### TS-02: Tap different word clears previous selection

- **Given:** Words tab active, word at index 3 selected (`search_selected_word = 3`, `search_word_selected = true`)
- **When:** User taps word at index 7
- **Expected:**
  - `st.search_selected_word = 7`
  - `st.search_word_selected = true`
  - Row 3 no longer highlighted, row 7 highlighted
- **Notes:** No `search_results_active` change at any point.

### TS-03: Open button activates search results when word selected

- **Given:** Words tab active, word selected (`search_word_selected = true`, `search_results_active = false`)
- **When:** User taps the Slint Open button (fires `chapter-selected(idx, 1)` callback)
- **Expected:**
  - `word_open_cell` set to `true`
  - Loop callback processes signal: `st.search_results_active = true`, `st.search_results_scroll = 0`, `st.text_dirty = true`
  - Overlay stays open (`chapter_overlay_open` remains `true`)
  - Search results view renders for the selected word
- **Notes:** `callbacks.rs` `on_chapter_selected` handler routes `tab == 1` to `word_open_cell` instead of closing overlay. Slint passes `(idx, 1)` from the Open button `clicked` handler. Activating search results via the Open button does NOT go through `jump_to_occurrence` and does NOT trigger audio reload -- it only shows the results list; the reading position is unchanged until the user taps a specific result.

### TS-04: Open button disabled when no word selected

- **Given:** Words tab active, no word selected (`search_word_selected = false`)
- **When:** User taps the Slint Open button
- **Expected:**
  - No state change (no results opened, no overlay close)
  - Word list remains visible with no selection
- **Notes:** The Rust handler checks `search_word_selected` before processing `word_open_cell`. Alternatively, the Slint button can be visually disabled via property binding, but the Rust guard is the safety net.

### TS-05: Back from search results preserves word selection

- **Given:** Words tab active, word selected, `search_results_active = true`, user viewing search results
- **When:** User taps the back arrow (`gesture::search_header_hit_test` returns `TabBarAction::Back`)
- **Expected:**
  - `st.search_results_active = false`
  - `st.search_results_scroll = 0`
  - `st.search_selected_word` unchanged (still points to the selected word)
  - `st.search_word_selected` remains `true`
  - Word list repaints with the previously selected word still highlighted
- **Notes:** `search.rs:26-33` back-arrow handler must NOT clear `search_selected_word` or `search_word_selected`.

### TS-06: Re-open results after back preserves same word

- **Given:** Back from results completed (TS-05 state), word list visible with selection
- **When:** User taps Open button
- **Expected:** Search results re-appear for the same word, no need to re-tap the row
- **Notes:** Validates the full round-trip: select -> open -> back -> open again.

### TS-07: Tab switch to Chapters clears word selection

- **Given:** Words tab active, word selected (`search_word_selected = true`)
- **When:** User taps Chapters tab
- **Expected:**
  - `st.chapter_tab = ChapterTab::Chapters`
  - `st.chapter_scroll = 0`
  - `st.search_word_selected = false` (cleared)
  - `st.search_selected_word` may remain but is irrelevant since sentinel gates rendering
  - Chapters list renders with no word highlighting artifacts
- **Notes:** Tab switch path at `search.rs:49-55` must set `search_word_selected = false`.

### TS-07b: Tab switch back to Words after clearing selection

- **Given:** Chapters tab active (after word selection was cleared by TS-07)
- **When:** User switches back to Words tab
- **Expected:**
  - `search_word_selected` remains `false`
  - No word is highlighted in the word list
  - `search_selected_word` may hold a stale index but the sentinel gate prevents any highlight
- **Notes:** Validates that clearing on tab-away (TS-07) is sticky -- returning to Words does not resurrect a selection.

### TS-08: Close overlay resets all selection state

- **Given:** Words tab active, word selected
- **When:** User taps close (X) button (`TabBarAction::Close`)
- **Expected:**
  - `reader.chapter_overlay_open = false`
  - `st.search_word_selected = false`
  - `st.search_selected_word = 0` (or whatever default)
  - `st.search_results_active = false`
- **Notes:** `search.rs:63-67` close handler must clear selection state. On next overlay open, no stale selection appears.

---

## R2: Draggable scrollbar

### TS-09: Scrollbar visible when list overflows

- **Given:** Word list with 25 words (exceeds viewport of ~19 visible rows: `(h - CH_LIST_TOP - CH_LIST_BOTTOM_PAD) / CH_ROW_PITCH`)
- **When:** Rendering the word list
- **Expected:** `paint_scrollbar` draws track + thumb. `scrollbar_visible(25)` returns `true`.
- **Notes:** `scrollbar_visible` threshold: `item_count * CH_ROW_PITCH > list_height` where `list_height = h - CH_LIST_TOP - CH_LIST_BOTTOM_PAD`.

### TS-10: Scrollbar hidden when list fits

- **Given:** Word list with 10 words (fits viewport)
- **When:** Rendering the word list
- **Expected:** `paint_scrollbar` returns early. `scrollbar_visible(10)` returns `false`.
- **Notes:** Existing early-return at `paint_scrollbar` line 39: `item_count <= 1`. New function must also handle the "all items fit" case (`item_count * CH_ROW_PITCH <= list_height`).

### TS-10b: Exact-fit boundary -- no scrollbar when content fills viewport exactly

- **Given:** Word list with exactly N items where `N * CH_ROW_PITCH == list_height` (content fills the viewport with zero remaining space)
- **When:** Rendering the word list
- **Expected:** `paint_scrollbar` returns early. `scrollbar_visible(N)` returns `false`. No scrollbar drawn.
- **Notes:** Tests the `<=` vs `<` boundary in `paint_scrollbar` / `scrollbar_visible`. When content exactly fits, there is nothing to scroll so the scrollbar must not appear. The condition must be `item_count * CH_ROW_PITCH <= list_height` (not strictly less than).

### TS-11: Touch-down on scrollbar rail jumps thumb to finger

- **Given:** Words tab active, list overflows, scrollbar visible, `sb_dragging = false`
- **When:** User touches down on the scrollbar track region (`dx >= w - SB_TRACK_PAD - SB_TRACK_W`, `dy` within list bounds), not on the thumb
- **Expected:**
  - `st.sb_dragging = true`
  - `st.sb_drag_tab = ChapterTab::Words`
  - List scroll jumps to position proportional to finger Y (thumb centers on finger)
  - `press_dispatched = false` (Slint does not receive `PointerPressed`)
  - `st.text_dirty = true`
- **Notes:** `scrollbar_hit_test` at `chapter_list.rs` detects the zone. `scrollbar_y_to_scroll` computes the scroll offset. `press_dispatched` lives in `touch_dispatch.rs`.

### TS-12: Touch-down on scrollbar thumb starts drag

- **Given:** Same as TS-11, but finger lands on the thumb itself
- **Expected:** Same as TS-11 (rail and thumb use same entry point: `sb_dragging = true`, scroll jumps). The thumb is within the track region so `scrollbar_hit_test` returns true regardless of thumb vs rail distinction.
- **Notes:** The implementation does not distinguish thumb vs rail on touch-down. Both jump+start drag.

### TS-13: Touch-move during drag updates scroll proportionally

- **Given:** `sb_dragging = true`, `sb_drag_tab = ChapterTab::Words`
- **When:** User moves finger up/down
- **Expected:**
  - `st.search_scroll` updated via `scrollbar_y_to_scroll(current_finger_y, item_count)`
  - `st.text_dirty = true`
  - Scroll clamped to `[0, scroll_max]`
  - Word list repaints at new scroll position
- **Notes:** `touch_dispatch.rs` move branch checks `st.sb_dragging` before swipe-scroll logic.

### TS-14: Touch-up ends drag, list stays at new position

- **Given:** `sb_dragging = true`, list scrolled to some mid-position
- **When:** User lifts finger
- **Expected:**
  - `st.sb_dragging = false`
  - `touch_release.rs` returns early (no row tap processed)
  - List remains at the scroll position set during the drag
- **Notes:** Critical: without the early return, the release would be interpreted as a word-row tap at whatever Y coordinate the finger lifted.

### TS-14b: Tap-without-move on scrollbar track jumps and holds

- **Given:** Scrollbar visible, `sb_dragging = false`
- **When:** User taps (down+up without move, `swipe_dy.abs() <= 40.0`) on scrollbar track
- **Expected:**
  - On touch-down: `sb_dragging = true`, list scroll jumps to tapped position (same as TS-11)
  - On touch-up: `sb_dragging = false`, list remains at the jumped position
  - No row selection occurs (release early-returns when `sb_dragging` was true)
- **Notes:** A tap (no drag) on the scrollbar still triggers the jump on down, then immediately ends the drag on up. The list stays at the jumped-to position. This is the no-drag degenerate case of TS-11 + TS-14.

### TS-15: Scrollbar touch does not dispatch to Slint

- **Given:** Overlay open, scrollbar visible, finger touches down in scrollbar zone
- **When:** Touch-down processed
- **Expected:** `press_dispatched = false`, so `PointerPressed` event is NOT forwarded to Slint. The Open button does not receive a press event.
- **Notes:** Without this guard, the Open button's `clicked` signal would fire on release, causing unintended chapter switch or search-results activation. `press_dispatched` lives in `touch_dispatch.rs`.

### TS-16: Scrollbar touch does not trigger row selection

- **Given:** `sb_dragging = true` (finger was on scrollbar zone)
- **When:** Finger lifts (touch release)
- **Expected:** `touch_release.rs` early-returns when `sb_dragging`. Neither `word_list_hit_test` nor `chapter_list_hit_test` is called. No row selection change.
- **Notes:** This is the release-side guard complementing TS-15.

### TS-17: Scroll max formula correct

- **Given:** `item_count = 25`, screen dimensions from `crate::w()/h()`
- **When:** Computing `scroll_max`
- **Expected:** `scroll_max = (item_count - 1) * CH_ROW_PITCH + CH_ROW_H - (h - CH_LIST_TOP - CH_LIST_BOTTOM_PAD)`, clamped to >= 0. Matches `search_scroll_max` at `search.rs:158-163`.
- **Notes:** `scrollbar_y_to_scroll` must use the same formula as `paint_scrollbar` to ensure thumb position corresponds to actual scroll. The formula is `scroll_max = (item_count - 1) * CH_ROW_PITCH + CH_ROW_H - list_h` where `list_h = h - CH_LIST_TOP - CH_LIST_BOTTOM_PAD`. This is NOT `total_h - visible_h` -- it accounts for the last row being fully visible at max scroll.

### TS-18: Y-to-scroll inverse formula correct (using max_travel)

- **Given:** `item_count = 25`, `finger_y` at list top
- **When:** `scrollbar_y_to_scroll(list_top, 25)`
- **Expected:** Returns `0` (top of list)
- **Notes:** `max_travel = track_h - thumb_h`. At `finger_y = list_top`: `finger_y_fraction = ((list_top - list_top) - thumb_h/2) / max_travel` -> clamped to `0.0` -> scroll = 0. Uses the `search_scroll_max` formula: `scroll_max = (item_count-1)*CH_ROW_PITCH + CH_ROW_H - list_h`.

### TS-18b: scrollbar_y_to_scroll and paint_scrollbar are inverse-consistent

- **Given:** `item_count = 25`, arbitrary scroll values `s` in `[0, search_scroll_max]`
- **When:** For each `s`, compute `thumb_y = paint_scrollbar_thumb_y(s, 25, list_h)` then `s2 = scrollbar_y_to_scroll(thumb_y + thumb_h/2, 25)`
- **Expected:** `s2 == s` for all tested values (within rounding tolerance of 1). The round-trip `scroll -> thumb_y -> scroll` is lossless.
- **Notes:** Both functions must use `search_scroll_max`'s formula: `scroll_max = (item_count-1)*CH_ROW_PITCH + CH_ROW_H - list_h`. If `scrollbar_y_to_scroll` uses a different `total_h - visible_h` derivation, the round-trip breaks and the thumb drifts under drag.

### TS-19: Y-to-scroll at list bottom returns scroll_max

- **Given:** `item_count = 25`, `finger_y` at list bottom
- **When:** `scrollbar_y_to_scroll(list_bottom, 25)`
- **Expected:** Returns `scroll_max` (bottom of list, last item visible)
- **Notes:** `finger_y_fraction` = 1.0 (clamped) -> scroll = scroll_max.

### TS-20: Y-to-scroll at midpoint returns approximately half

- **Given:** `item_count = 25`, `finger_y` at list midpoint
- **When:** `scrollbar_y_to_scroll(midpoint, 25)`
- **Expected:** Returns approximately `scroll_max / 2` (within rounding tolerance of 1)
- **Notes:** Tests proportional mapping is linear.

### TS-21: Scrollbar during drag paints thumb at finger position

- **Given:** `sb_dragging = true`, finger at Y = 400 (mid-list)
- **When:** Frame renders during drag
- **Expected:** `paint_scrollbar` draws thumb centered near Y = 400. The thumb position corresponds to `scroll_frac * max_travel + list_top`.
- **Notes:** Validates the paint formula and the y_to_scroll formula are inverses.

### TS-22: Scrollbar after drag paints thumb at final scroll position

- **Given:** Drag ended (`sb_dragging = false`), `search_scroll = 350`
- **When:** Frame renders after release
- **Expected:** `paint_scrollbar` draws thumb at position corresponding to scroll = 350 (same as if scroll was set by swipe or any other means).
- **Notes:** Thumb position depends only on `scroll` value, not on `sb_dragging` state.

### TS-23: Scrollbar hit test inside track returns true

- **Given:** `dx = w - SB_TRACK_PAD - SB_TRACK_W/2` (center of track column), `dy` within `[CH_LIST_TOP, list_bottom)`
- **When:** `scrollbar_hit_test(dx, dy, w)`
- **Expected:** Returns `true`
- **Notes:** Unit test. Straightforward coordinate check.

### TS-24: Scrollbar hit test left of track returns false

- **Given:** `dx = w - SB_TRACK_PAD - SB_TRACK_W - 20` (left of track)
- **When:** `scrollbar_hit_test(dx, dy, w)`
- **Expected:** Returns `false`

### TS-25: Scrollbar hit test above list returns false

- **Given:** `dy = CH_LIST_TOP - 1`
- **When:** `scrollbar_hit_test(dx, dy, w)` with dx inside track column
- **Expected:** Returns `false`

### TS-26: Scrollbar hit test below list returns false

- **Given:** `dy = h - CH_LIST_BOTTOM_PAD`
- **When:** `scrollbar_hit_test(dx, dy, w)` with dx inside track column
- **Expected:** Returns `false`

### TS-27: Scrollbar works on Chapters tab

- **Given:** Chapters tab active, chapter list with 25 rows, scrollbar visible
- **When:** User drags the scrollbar
- **Expected:** `st.chapter_scroll` updates (not `st.search_scroll`). `sb_drag_tab = ChapterTab::Chapters` recorded on touch-down.
- **Notes:** Move handler dispatches to the correct scroll field based on `st.sb_drag_tab`.

## Deferred (out of scope)

### TS-28: Scrollbar works during search results view [DEFERRED]

- **Scope boundary:** Search results scrollbar dragging is out of scope for this pass (plan.md says "visual-only remains for results").
- **Given:** Words tab active, `search_results_active = true`, 200 results
- **When:** User drags the scrollbar
- **Expected:** `st.search_results_scroll` updates. `sb_drag_tab = ChapterTab::Words`.
- **Notes:** Move handler checks `search_results_active` in addition to `sb_drag_tab` to pick the correct field. This scenario is deferred to a future pass.

---

## R3: Open button consistency

### TS-29: Open button on Chapters tab fires chapter switch

- **Given:** Chapters tab active, chapter 3 selected (`chapter_preview_idx = 3`)
- **When:** User taps Open button
- **Expected:**
  - Slint fires `chapter-selected(3, 0)`
  - Rust handler: `tab == 0` -> sets `chapter_select_cell`, closes overlay (`chapter_overlay_open = false`)
  - Chapter 3 opens
- **Notes:** Existing behavior, must not regress.

### TS-30: Open button on Words tab activates search results

- **Given:** Words tab active, word selected
- **When:** User taps Open button
- **Expected:**
  - Slint fires `chapter-selected(idx, 1)`
  - Rust handler: `tab == 1` -> sets `word_open_cell`, overlay stays open
- **Notes:** See TS-03 for full detail. Same caveat: activating search results does NOT go through `jump_to_occurrence` and does NOT trigger audio reload.

### TS-31: Callback receives correct (idx, tab) pair for chapters

- **Given:** Chapters tab, chapter 5 selected
- **When:** Open button clicked
- **Expected:** `on_chapter_selected(5, 0)` called
- **Notes:** The `idx` comes from `chapter-preview-idx` Slint property. The `tab` comes from `active-tab` property on `ChapterOverlay`.

### TS-32: Callback receives correct (idx, tab) pair for words

- **Given:** Words tab, word at index 12 selected
- **When:** Open button clicked
- **Expected:** `on_chapter_selected(12, 1)` called
- **Notes:** `idx` is `search_selected_word` passed to Slint. `tab` is `active-tab = 1`.

### TS-33: Tab switch updates Slint active-tab property

- **Given:** User switches from Chapters to Words tab
- **When:** Tab bar tap processed (`TabBarAction::WordsTab` at `search.rs:56-62`)
- **Expected:** Rust sets `ChapterOverlay.active-tab = 1` (or equivalent property)
- **Notes:** `search.rs` tab-switch paths must update the Slint property.

### TS-34: Chapters select + open regression (existing flow unchanged)

- **Given:** Overlay open, Chapters tab active
- **When:** User taps chapter row 2, then taps Open
- **Expected:**
  - Row 2 highlighted after tap (no chapter opened yet)
  - Open button fires `chapter-selected(2, 0)`
  - Overlay closes, chapter 2 opens
  - No regression from pre-change behavior
- **Notes:** Full end-to-end regression for the chapters path. Existing audio sync in `jump_to_occurrence` (called when tapping a specific search result row) remains intact and is outside this flow -- this scenario tests only the chapter select + open path, not search result activation.

---

## Edge cases

### TS-35: Empty word list

- **Given:** Word index has 0 words
- **When:** Words tab displayed
- **Expected:**
  - No scrollbar drawn (`scrollbar_visible(0)` returns `false`)
  - `paint_word_list` shows "No searchable text" message
  - No selection possible (taps in list area hit nothing)
  - Open button does nothing
- **Notes:** `paint_word_list` at `word_list.rs:159-161` already handles empty list.

### TS-36: Single word in list

- **Given:** Word list has exactly 1 word
- **When:** User taps the word row, then taps Open
- **Expected:**
  - No scrollbar (fits viewport)
  - Tap selects the word (highlighted)
  - Open activates search results for that word
  - Back returns to list with word still selected
- **Notes:** `paint_scrollbar` returns early at `item_count <= 1`. Selection flow works normally.

### TS-37: Very long word list (500+ items)

- **Given:** Word list with 600 words
- **When:** Rendering and scrolling
- **Expected:**
  - Scrollbar visible
  - Thumb height proportional to `visible_h / total_h` (small thumb)
  - Drag scrollbar: scroll updates proportionally across full range
  - `scrollbar_y_to_scroll` clamps correctly at extremes
  - `search_scroll_max(600)` returns correct max
- **Notes:** Tests that proportional mapping scales to large lists without overflow or precision issues.

### TS-38: Search results with 1 match

- **Given:** Selected word has exactly 1 occurrence
- **When:** Search results displayed
- **Expected:** No scrollbar drawn (`item_count = 1`, `paint_scrollbar` returns early)
- **Notes:** Single result row fits in viewport.

### TS-39: Search results with 200 matches

- **Given:** Selected word has 200 occurrences (= `MAX_SEARCH_RESULTS`)
- **When:** Search results displayed
- **Expected:** Scrollbar visible. Thumb is proportional. Scrolling works normally.
- **Notes:** 200 matches overflow the viewport. Scrollbar track + thumb rendered.

### TS-40: Drag scrollbar to exact end

- **Given:** List with 25 items, scrollbar visible
- **When:** Drag thumb to bottom of track
- **Expected:** Last item is fully visible (not partially clipped). Scroll = `scroll_max`.
- **Notes:** The scroll formula should ensure the last row's top + `CH_ROW_H` = `list_bottom` when at max scroll.

### TS-41: Drag scrollbar to exact start

- **Given:** List with 25 items, scrollbar visible
- **When:** Drag thumb to top of track
- **Expected:** First item visible at `CH_LIST_TOP`. Scroll = 0.
- **Notes:** Validates the top boundary.

---

## Unit test naming map

Tests listed in `definition-of-done.md` Test Coverage section map to:

| DoD test name | This plan |
|---------------|-----------|
| `scrollbar_hit_test_inside_track_returns_true` | TS-23 |
| `scrollbar_hit_test_outside_track_returns_false` | TS-24 |
| `scrollbar_hit_test_above_list_returns_false` | TS-25 |
| `scrollbar_hit_test_below_list_returns_false` | TS-26 |
| `scrollbar_y_to_scroll_top_returns_zero` | TS-18 |
| `scrollbar_y_to_scroll_bottom_returns_max` | TS-19 |
| `scrollbar_y_to_scroll_midpoint_returns_half` | TS-20 |
| `scrollbar_visible_few_items_returns_false` | TS-10 |
| `scrollbar_visible_many_items_returns_true` | TS-09 |
| `word_tap_selects_does_not_open_results` | TS-01 |
| `back_from_results_preserves_selection` | TS-05 |

Additional tests beyond DoD: TS-01b through TS-08, TS-10b, TS-11 through TS-17, TS-18b, TS-21, TS-22, TS-27 through TS-41. TS-28 is deferred.
