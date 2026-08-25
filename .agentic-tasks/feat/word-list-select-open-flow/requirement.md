# feat: Words tab select-then-open flow (match Chapters tab)

## User request

The Words tab interaction should match the Chapters tab:
- **Chapters tab**: single tap selects a chapter (highlights it), then an "Open" button opens it
- **Words tab should be**: single tap selects a word (highlights it), then an "Open" button shows word usages (search results). From search results, single tap on a result opens the book page at that occurrence.

## Current behavior

### Chapters tab (select + Open button)

1. **Single tap on a chapter row**: `touch_release.rs:168-173` calls `gesture::chapter_overlay_target()` which returns `ChapterOverlayAction::Select(idx)`. The release handler calls `reader.set_chapter_preview_idx(idx as i32)`. This highlights the row visually (chapter_list.rs:146 checks `i as i32 == selected`). No chapter is opened yet.
2. **"Open" button**: The Slint `ChapterOverlay` component (`chapter_overlay.slint:69-76`) has an `ActionButton` with label "Open". When clicked, it fires `root.chapter-selected(root.chapter-preview-idx >= 0 ? root.chapter-preview-idx : root.current-chapter-idx)`. This callback is wired in `callbacks.rs:66-78` which sets `chapter-overlay-open = false`, resets `chapter_preview_idx` to -1, and fires `select_cell` to trigger the actual chapter switch.
3. **State fields used**:
   - `reader.chapter_preview_idx` (Slint property, i32, -1 = none selected)
   - `reader.chapter_overlay_open` (Slint property, bool)
   - `reader.chapter_pending` (Slint property, i32)

### Words tab (direct-open on tap)

1. **Single tap on a word row**: `search.rs:71-86` calls `word_list_hit_test()`. On match, it immediately sets `st.search_selected_word = idx` AND sets `st.search_results_active = true`, switching directly to search results. There is no select-then-confirm step.
2. **Search results view**: `search.rs:26-46` handles taps on results. A tap on a result row calls `jump_to_occurrence()` which switches chapter, navigates to the page, and closes the overlay. A tap on the back arrow returns to the word list.
3. **State fields used**:
   - `st.search_selected_word` (usize, index into `word_index.words`)
   - `st.search_results_active` (bool)
   - `st.search_results_scroll` (i32)
   - `reader.chapter_overlay_open` (Slint property, bool)

### Key difference

Chapters: tap -> sets `chapter_preview_idx` (visual highlight) -> user taps "Open" button -> fires `chapter-selected` callback -> closes overlay + switches chapter.

Words: tap -> immediately sets `search_selected_word` AND `search_results_active = true` -> jumps straight to search results. No selection state, no "Open" confirmation.

## Desired behavior

Words tab should mirror the chapters tab's two-step flow:

1. **Single tap on a word row**: Sets `st.search_selected_word = idx` (highlights the row in the word list). Does NOT open search results. The row gets the same inverted highlight it already has when `i == selected_word` (word_list.rs:172-177).
2. **"Open" button**: When the Words tab has a selected word, the "Open" button becomes available. Tapping it sets `st.search_results_active = true` and `st.search_results_scroll = 0`, switching to search results.
3. **Search results remain unchanged**: Tapping a result row calls `jump_to_occurrence()` as before. The back arrow returns to the word list with the previously selected word still highlighted.
4. **Tab switching**: Switching from Words to Chapters (or vice versa) should clear the selection state, same as today (already resets scroll, and `search_selected_word` would stay but is harmless since it's tab-specific).

### State transitions

| Action | State changes |
|--------|--------------|
| Tap word row (Words tab) | `st.search_selected_word = idx`, repaint (highlight) |
| Tap "Open" (Words tab, word selected) | `st.search_results_active = true`, `st.search_results_scroll = 0` |
| Tap back arrow (search results) | `st.search_results_active = false`, `st.search_results_scroll = 0` |
| Tap result row (search results) | `jump_to_occurrence()` -> closes overlay |
| Switch to Chapters tab | `st.chapter_tab = ChapterTab::Chapters`, `st.chapter_scroll = 0` |
| Switch to Words tab | `st.chapter_tab = ChapterTab::Words`, `st.search_scroll = 0` |
| Close overlay (X button) | `reader.chapter_overlay_open = false` |

### Open button label

The Open button is currently a Slint `ActionButton` with hardcoded label "Open" in `chapter_overlay.slint:71`. It fires `root.chapter-selected(...)`. For the Words tab, this same button should instead activate search results. The button label can stay "Open" for both tabs (it opens the selected item -- a chapter or a word's usages).

### How the Open button currently works

The Slint `ChapterOverlay` component draws the Open button and the 110px header. However, the Rust code paints its own tab bar and list over the buffer. The Open button is Slint-rendered and sits below the Rust-painted list area (in the `CH_LIST_BOTTOM_PAD = 136` reserved strip). The Slint button's `clicked` callback fires `root.chapter-selected(...)` which closes the overlay.

The Slint overlay header also draws "Chapters" as a hardcoded title, but the Rust tab bar covers the 110-158px band. This means the Slint header text is hidden when the Rust tab bar is visible.

**Implementation implication**: The Open button is already rendered by Slint at the bottom of the overlay. For the Words tab, we need the Open button to either:
- (a) Be wired to a different callback that activates search results when on the Words tab, OR
- (b) Have the Rust touch handler intercept taps in the bottom strip and handle them directly (consistent with how the tab bar and list are already Rust-drawn).

Option (b) is simpler and more consistent: the Rust code already owns the tab bar and list rendering, and the touch dispatch already runs through `handle_search_release()`. Adding a hit test for the Open button area in the bottom strip keeps all the Words tab logic in Rust.

## Open questions

None. The codebase is clear about the interaction model. The only design choice is whether the Open button action is handled in Slint or Rust (recommendation: Rust, for consistency with the existing tab/list handling).
