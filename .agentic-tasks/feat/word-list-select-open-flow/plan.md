# Plan: feat/word-list-select-open-flow

## Summary

Three related changes to the chapter overlay interaction model:
1. Words tab switches from direct-open to select-then-open (matching chapters).
2. Both word list and chapter list gain a draggable scrollbar (replacing visual-only).
3. The Slint Open button routes conditionally based on the active tab.

## Design Decisions

| ID | Decision | Rationale |
|----|----------|-----------|
| DD-1 | Use existing Slint Open button for both tabs | The callback signature changes to `callback chapter-selected(int, int)` passing both `idx` and `tab` (0=Chapters, 1=Words). Rust handler routes based on the `tab` argument. No Slint property needed. |
| DD-2 | Tap word row selects only (no results open) | Matches the chapters tab's two-step pattern exactly. |
| DD-3 | Back from results preserves word selection | User can re-open the same word's results without re-tapping. Matches chapters (going back keeps the chapter selected). |
| DD-4 | Draggable scrollbar on right edge | Replaces visual-only scrollbar. Thumb drag = proportional scroll, rail tap = jump. |
| DD-5 | Full-height scrollbar rail (tap anywhere to jump) | Easier to target on e-ink. Both fast-jump and fine-control. |

## Architecture

### Interaction state machine (Words tab)

```
                       Word list
                      (no selection)
                           |
              tap word row |
          +----------------+
          |
          v
                       Word list
                      (word selected)
                     /       |        \
         tap another  drag    tap "Open"
         word row     SB      |
         |            / \         v
         v     drag /   \ release  Search results
       (re-select) /     \         /              \
                   v       v  tap result     tap < back
              scroll moves  list    row i           |
              with thumb   stays      |              v
                              at      v         Word list
                              new   jump_to_page() (same word
                              pos   close overlay  selected)
```

### Scrollbar touch handling

The scrollbar occupies the right edge of the list area (same column `paint_scrollbar` already draws to). Touch-down detection uses a new `scrollbar_hit_test` function that checks whether the finger lands on the scrollbar track region (right edge of list area, width = `SB_TRACK_PAD + SB_TRACK_W`).

State fields needed:
- `sb_dragging: bool` -- true when a scrollbar drag is in progress (set on touch-down in scrollbar zone, cleared on release).
- `sb_drag_active_tab: ChapterTab` -- which tab's scrollbar is being dragged (needed because both tabs share the scrollbar column).

On touch-down in the scrollbar zone:
- Compute scroll position from finger Y (proportional mapping from finger position to list scroll offset).
- Set `sb_dragging = true`, store the tab.

On touch-move while `sb_dragging`:
- Recompute scroll from current finger Y.
- Clamp to scroll_max.
- Set `text_dirty = true`.

On touch-release while `sb_dragging`:
- Clear `sb_dragging = false`.
- Do NOT treat as a tap on a list row.

Scrollbar visibility: `paint_scrollbar` already returns early when `item_count <= 1`. The threshold for hiding is when all items fit in the viewport: `item_count * CH_ROW_PITCH <= list_height`.

### Open button routing

The Slint `ChapterOverlay` button fires `root.chapter-selected(idx, tab)`. The existing `on_chapter_selected` callback in `callbacks.rs:66-78` closes the overlay and sets `chapter_select_cell`. A new callback is not needed; instead, the Rust-side handler checks the `tab` argument:

- If `chapter_tab == Chapters`: existing behavior (close overlay, fire `select_cell` for chapter switch).
- If `chapter_tab == Words`: set `search_results_active = true`, `search_results_scroll = 0`, keep overlay open.

Implementation: change the `chapter-selected` callback in both Slint files to `callback chapter-selected(int, int)` where the second argument is the active tab (0=Chapters, 1=Words). The Slint Open button's `clicked` handler already knows which tab is active (via `root.chapter-tab` property on `ChapterOverlay` or hard-coded per-tab wiring). It passes `(idx, tab)` to the callback. The Rust `on_chapter_selected` handler receives `(idx: i32, tab: i32)` and routes: `tab == 0` sets `chapter_select_cell`, `tab == 1` sets `word_open_cell`.

Slint `ChapterOverlay` already tracks which tab is active (the `active-tab` property or the tab bar's selected state). The `clicked` handler for the Open button reads this and passes it as the second callback argument. No new Slint property is needed beyond what already tracks tab state.

## Files to Modify

### 1. `kothok/src/loop_run/search.rs`

**R1 - Select-then-open**: Change `handle_search_release` lines 71-86. Remove the immediate `st.search_results_active = true` on word tap. Only set `st.search_selected_word = idx`, set `st.search_word_selected = true`, and repaint.

**R1 - Open button activation**: Add handling for the new `word_open_cell` in the release path (or via a new callback check). When the word open signal fires, set `st.search_results_active = true` and `st.search_results_scroll = 0`.

**R1 - Sentinel update**: The `search_word_selected: bool` field (new in LoopState) gates highlighting. `jump_to_occurrence` (line 98) reads `st.search_selected_word` unconditionally -- it is only reached when results are active, so no guard change needed there.

### 2. `kothok/src/loop_state.rs`

**R2 - Scrollbar drag state**: Add fields:
- `sb_dragging: bool` (default false)
- `sb_drag_tab: ChapterTab` (which tab's scrollbar is active)

**R1 - Selection sentinel**: Add field:
- `search_word_selected: bool` (default false) -- true when a word has been tapped in the Words tab. `paint_word_list` uses this to decide whether to highlight any row.

### 3. `kothok/src/loop_run/touch_dispatch.rs`

**R2 - Scrollbar drag**: In the `st.frame_down` move branch (lines 239-278), add a check before the swipe-scroll logic. If `st.sb_dragging`, compute scroll from finger Y position using the scrollbar proportion formula and update the appropriate scroll field (`search_scroll`, `chapter_scroll`, or `search_results_scroll`).

**R2 - Scrollbar touch-down**: In the press branch (lines 106-205), after the existing gesture classification, add a `scrollbar_hit_test` check when the overlay is open. If hit, set `sb_dragging = true`, compute initial scroll, and set `press_dispatched = false` to prevent `PointerPressed` from reaching Slint (which would cause the Open button to activate on release).

### 4. `kothok/src/loop_run/touch_release.rs`

**R2 - Scrollbar release**: In `on_release`, early-return when `st.sb_dragging` to prevent the scrollbar release from being interpreted as a list-row tap.

### 5. `kothok/src/rendering/chapter_list.rs`

**R2 - Scrollbar hit test**: Add `pub fn scrollbar_hit_test(dx: f32, dy: f32, w: usize) -> bool` that checks whether the touch is within the scrollbar track region (right edge of list area: `x >= w - SB_TRACK_PAD - SB_TRACK_W`, `y` within list bounds).

**R2 - Scrollbar visibility helper**: Add `pub fn scrollbar_visible(item_count: usize) -> bool` that returns true when `item_count > visible_rows`.

**R2 - Scroll position computation**: Add `pub fn scrollbar_y_to_scroll(finger_y: i32, item_count: usize) -> i32` that maps a finger Y position to a scroll offset. The formula inverts `paint_scrollbar`'s thumb position calculation:
- `max_travel = track_h - thumb_h` (same as `paint_scrollbar` line 53)
- `finger_y_fraction = ((finger_y - list_top) - thumb_h/2) / max_travel` (centers the thumb on the finger), clamped to `[0.0, 1.0]`
- `scroll = (finger_y_fraction * scroll_max).round()` where `scroll_max = total_h - visible_h`
- Result clamped to `[0, scroll_max]`

This matches the inverse of `paint_scrollbar`'s `thumb_top = list_top + scroll_frac * max_travel`.

### 6. `kothok/src/rendering/word_list.rs`

**R1 - Selected word sentinel**: Change `paint_word_list` signature to accept a `word_selected: bool` parameter (or pass the pair `(usize, bool)`). At line 172, change `let selected = i == selected_word` to `let selected = word_selected && i == selected_word`. When `word_selected` is false, no row is highlighted regardless of the `selected_word` index value.

### 7. `kothok/src/app/render.rs`

**R1 - Pass selection state**: At line 161-167, when calling `paint_word_list`, pass the new `st.search_word_selected` bool so the painter can gate highlighting. `st.search_selected_word` (the index) is still passed as-is.

### 7a. `kothok/src/loop_run/search.rs` (sentinel update for `jump_to_occurrence`)

**MEDIUM 2**: `jump_to_occurrence` (line 92) reads `st.search_selected_word` to look up occurrences. This function is only called from the search results hit-test path (line 42), which is gated by `st.search_results_active == true`. No sentinel guard needed here -- results can only be active when a word is selected. No changes to `jump_to_occurrence` signature or body.

### 8. `kothok/src/callbacks.rs`

**R3 - Open button routing**: Add `word_open_cell: Rc<Cell<bool>>` to `Callbacks`. In `register_chapter`, modify the `on_chapter_selected` handler to accept two arguments `(idx: i32, tab: i32)`. When `tab == 0` (Chapters): existing behavior. When `tab == 1` (Words): set `word_open_cell` to true, do NOT close overlay.

### 9. `kothok/ui/components/chapter_overlay.slint` and `kothok/ui/reader.slint`

**R3 - Callback signature change**: Change `callback chapter-selected(int)` to `callback chapter-selected(int, int)` in both `chapter_overlay.slint` (line 13) and `reader.slint` (line 102). The Open button's `clicked` handler in `chapter_overlay.slint` (line 74) passes both the chapter index and a tab identifier: `root.chapter-selected(idx, tab_value)`. The `tab_value` can come from a local `active-tab` property (add `in-out property <int> active-tab: 0` to `ChapterOverlay`) that Rust sets when switching tabs. In `reader.slint` (line 266), forward both arguments: `root.chapter-selected(i, tab)`.

### 10. `kothok/src/loop_run/callbacks.rs` (or wherever `process_loop_callbacks` lives)

**R3 - Handle word open**: Process the `word_open_cell` signal. When true, set `st.search_results_active = true`, `st.search_results_scroll = 0`, `st.text_dirty = true`.

### 11. `kothok/src/gesture/mod.rs`

No changes needed. The `sb_dragging` flag in `LoopState` is checked in the release path (`touch_release.rs`) before hit tests run, so scrollbar touches are excluded without modifying gesture hit-test functions.

### 12. `kothok/src/setup/loop_init.rs`

**R2 + R1 - Initialize new LoopState fields**: Add initialization in the `LoopState` struct literal (around line 31-147):
- `sb_dragging: false`
- `sb_drag_tab: ChapterTab::default()` (Chapters)
- `search_word_selected: false`

These must appear in the struct construction block alongside the existing `search_selected_word: 0`.

## Dependencies to Add

None.

## Out of Scope

- Scrollbar for search results (already uses `paint_scrollbar` but no draggable interaction in this pass).
- Scrollbar auto-hide timer (fade after inactivity). The scrollbar hides when content fits; no additional hiding.
- Haptic feedback on scrollbar interaction (not available on this device).
- Chapter overlay Slint title text ("Chapters"/"Words" label in the header). The Rust tab bar covers it and it's not visible.
- Changing the Open button label per tab.

## Risk Assessment

| Risk | Likelihood | Mitigation |
|------|-----------|------------|
| Scrollbar drag conflicts with list swipe-down | Medium | Check scrollbar zone first in touch-down. `sb_dragging` flag gates the move handler, so a list-area swipe never triggers scrollbar logic. |
| Selected-word sentinel change (`search_word_selected: bool`) breaks callers | Low | `paint_word_list` gains one `bool` parameter. Called from one place (`app/render.rs:161`). `jump_to_occurrence` does not need the sentinel -- it is gated by `search_results_active`. |
| Open button Slint callback routing changes break chapters | Low | Chapters path is unchanged: same `on_chapter_selected` fires, same `chapter_select_cell` is set. The Words path is additive. |
| Scrollbar proportion formula diverges from paint_scrollbar | Low | Extract the formula into a shared helper (`scrollbar_y_to_scroll`) used by both painter and hit-test. |

## Best Practices Reference

- `docs/CODE_CONVENTIONS.md` -- module size limits, naming, types-over-tuples, error handling
- `docs/REFACTOR_PLAN.md` -- ongoing restructuring context
- AGENTS.md -- no comments unless asked, ASCII-only, LF line endings, build before commit
