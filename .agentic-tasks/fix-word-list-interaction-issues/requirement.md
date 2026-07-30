# Requirement: fix-word-list-interaction-issues

## User request

Three issues with the word list / dictionary feature on the Kobo Libra Colour:

1. **Scrollbar needed**: On the book page, there is a long list of words. To make navigation easier, add a draggable scrollbar at the bottom of the word list (before the Open button), similar to how a book reader has a scrollbar for page navigation.

2. **Selection visibility**: It is hard to understand whether a word is currently selected or not. The selected state needs to be visually clearer.

3. **Crash on click**: When single-clicking, double-clicking, or clicking the Open button on a word, the Kothok application crashes. Not clear which specific action triggers the crash - it may be any of them.

## Clarifications
<!-- Filled in during Phase 1. Each Q/A pair: -->

### Phase 1 exploration findings (developer)

The "word list" was located in the codebase. It is the **Chapters / table-of-contents
overlay** (the `ChapterOverlay` Slint component in `ui/components/chapter_overlay.slint`,
paired with the Rust painter `rendering::chapter_list.rs::paint_chapter_list`). Evidence:

- It is the only list UI that has an **"Open" button** (`chapter_overlay.slint:69-76`),
  matching the user's "Open button on a word".
- It is a long, **vertically scrollable** list (scroll state `LoopState::chapter_scroll`,
  swipe-scroll in `loop_run/touch_dispatch.rs:260-276`, row hit-test in
  `chapter_list.rs::chapter_list_hit_test`), matching "long list" + the scrollbar request.
- Selection state is `chapter_preview_idx` (tap a row -> `ChapterOverlayAction::Select` in
  `gesture/mod.rs:229-244`, consumed in `loop_run/touch_release.rs:148-167`), matching the
  "is a word selected?" visibility issue.

How the three issues map:

1. **Scrollbar**: the list scrolls only by swipe today; there is no on-screen scrubber.
   The user wants a draggable bar at the bottom of the list (above the "Open" button),
   like a book reader's page progress bar, to jump-scroll.
2. **Selection visibility**: a selected row is currently distinguished only by a thin
   border-color change (selected border `0x0000` black vs unselected `0x94B2` grey, both on
   a white card fill - `chapter_list.rs:94-99`). This is too subtle to read as "selected".
3. **Crash on click**: tapping a row (single/double) or pressing "Open" crashes the app
   (panic = SIGABRT = reboot on this device). I traced every obvious index/panic path in
   the chapter-overlay tap, render, and jump paths; all are bounds-checked
   (`st.toc_rows.get(nc)`, `blit_rgb565` pixel guards, `clamp_page`, etc.). The root cause
   is not obvious from static reading and likely needs either the on-device panic trace or
   a build+test cycle to surface.

### Questions for the user (requirement-level only)

<!-- Q1 -->
**Q1: Feature confirmation.** ANSWERED. The "list of words" is the **Words tab**
inside the chapter (TOC) overlay - the overlay has two tabs, "Chapters" and
"Words". Tapping a word shows a search-results list of its occurrences, and a
result can be opened (jump to that location). This feature is currently
**uncommitted WIP** (see Q3 below), not yet in any branch.

### Phase 1 findings (developer) - corrected after locating the feature

The feature is the **Words tab** of the chapter overlay. Files (all uncommitted,
on disk in the main working tree at `D:\Programming\BitOps\EReader\kothok\src`):

- `data/word_index.rs` - `build_word_index(chapters)`: a sorted `Vec<String>` of
  unique lowercased words + parallel `Vec<Vec<WordHit>>` occurrences
  (`chapter: u16`, `byte_offset: u32` into `chapter.body`).
- `rendering/word_list.rs` - paints the Words-tab list (`paint_word_list`),
  the tab bar (`paint_tab_bar`: Chapters/Words/close), `word_list_hit_test`.
- `rendering/search_results.rs` - paints the occurrences list
  (`paint_search_results`), `build_snippet`, `results_hit_test`.
- `loop_run/search.rs` - `handle_search_release` (tap a word -> show results;
  tap a result -> `jump_to_occurrence` switches chapter + page + audio),
  `search_scroll_max` / `results_scroll_max`.
- Integration (in stash): `loop_state.rs` (new fields `word_index`,
  `chapter_tab`, `search_scroll`, `search_results_active`, `search_results_scroll`,
  `search_selected_word`, ...), `touch_dispatch.rs` (swipe-scroll per tab),
  `touch_release.rs` (calls `handle_search_release`), `gesture/mod.rs`
  (`tab_bar_hit_test`, `search_header_hit_test`), `app/render.rs` (paints
  tab bar + word list / search results), `data/library.rs` + `open_book`
  (build + cache the index), new dep `unicode-segmentation`.

How the three issues map:

1. **Scrollbar (confirmed absent)**: neither `paint_word_list` nor
   `paint_search_results` draws any scrollbar/scrubber. The lists scroll only by
   swipe (`search_scroll` / `search_results_scroll`). Issue #1 = add a draggable
   scrollbar at the bottom of the list, above the "Open" button.
2. **Selection visibility (confirmed absent - worse than expected)**:
   `paint_word_list` draws every row identically (white card, `0x94B2` border)
   - there is **no selected-state visual at all** (unlike the chapter list, which
   has a selected border). The selected word is held in `search_selected_word`
   but never reflected visually. Issue #2 = add a clear selected-row treatment.
3. **Crash**: the on-disk feature code is defensively written (all index/optional
   access guarded; `chapter.body` is stable between indexing and rendering, so
   `build_snippet`'s slice does not go out of range). No panic is reachable by
   static analysis on the tap / render / jump paths. Root-causing needs a
   build+test of the actual WIP (Phase 4) - and first the worktree must actually
   contain the feature (see Q3).

<!-- Q3 -->
**Q3: Worktree setup blocker (NEW - discovered during Phase 1).**
The Words-tab feature is **uncommitted WIP**: its four new source files exist
only as untracked files in the main working tree (`D:\Programming\BitOps\EReader`,
on branch `fix/word-list-interaction-issues` @ `3584945`), and the integration
edits live in `stash@{0}` ("WIP on feat/book-search"). The task branch
`fix-word-list-interaction-issues-2` (this worktree) was cut from an older
`develop` (`109056c`) that is **34 commits behind** and contains **none** of the
feature. I cannot build, test, or fix the feature here. Before Phase 4 the task
branch must be brought to the feature's base (`feat/book-search` tip `3584945`)
with the WIP files + stash applied - this needs an orchestrator decision.

### Out of scope for Phase 1 (deferred to Phase 2 design decisions)
- Scrollbar orientation / behaviour (horizontal scrubber vs vertical rail, drag-to-seek vs
  page-step tap), thumb sizing, and e-ink waveform for the scrubber.
- The exact "selected" visual treatment (filled accent fill, inverted text, thicker
  border, checkmark, etc.) and its e-ink ghosting cost.

## Final requirement
<!-- Consolidated after clarifications. This is what the developer implements. -->
<!-- TODO: finalize once Q1/Q2 are answered. Draft (pending confirmation): -->
<!-- Fix the chapter list (TOC) overlay: -->
<!--  (1) add a draggable scrollbar at the bottom of the list, above the "Open" button, -->
<!--      for fast scroll navigation; -->
<!--  (2) make the selected-row state clearly distinguishable from unselected rows; -->
<!--  (3) diagnose and fix the crash triggered by tapping a row (single/double click) or -->
<!--      pressing "Open". -->
<!-- Root-cause the crash; do not add a fallback that masks it. -->
