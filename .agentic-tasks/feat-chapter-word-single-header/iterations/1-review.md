# Iteration 1 Review (Plan - Step 3)

## Verdict: FEEDBACK

### BLOCKING

1. **search_results.rs:13 imports `TAB_BAR_TOP`, `TAB_BTN_H` from word_list** - These become dead after removing the 110-158px header band. `INK` and `TAB_BORDER` stay (used at line 76 in result row rendering).
   Fix: Remove `TAB_BAR_TOP`, `TAB_BTN_H` from the word_list import. Remove dead constants `BACK_W`, `BACK_X`, `HEADER_PX`.

2. **app/render.rs:127 references `word_list::TAB_BAR_TOP`** - This debug log will fail to compile after TAB_BAR_TOP is deleted.
   Fix: Remove or update this reference.

3. **word_list.rs:74 and search_results.rs:38 band-clearing loops** - `for y in TAB_BAR_TOP..CH_LIST_TOP` clearing. word_list.rs:74 is inside `paint_tab_bar()` (already listed for removal). search_results.rs:38 is inside `paint_search_results()` (NOT listed for removal - must be explicitly called out).
   Fix: Explicitly list the search_results.rs line 38-41 band-clearing loop and lines 46-68 header text rendering as dead code to remove.

### HIGH

4. **touch_release.rs listed as "remove tab_bar_hit_test dispatch"** - No tab_bar_hit_test references exist in touch_release.rs. The dispatch is in loop_run/search.rs (already listed).
   Fix: Remove touch_release.rs from modified files. Add to out-of-scope.

5. **gesture/mod.rs:3,7,8,9 dead imports** - `CH_LIST_TOP` (line 3), `back_arrow_hit_test` (line 7), `tab_btn_rects` (line 8), `TAB_BAR_TOP` (line 9) all become dead after removing functions. Not listed in plan.
   Fix: Add all four imports to dead code removal.

6. **Back button dual-purpose routing** - Back button in search results must call `back_from_results()` (return to word list), but in normal mode calls `close-to-book()` (exit overlay). The Slint back button callback needs to check `search_results_active` in Rust and route accordingly. Close button always exits overlay.
   Fix: Add "Back button dual-purpose routing" section to plan explaining the mode-dependent behavior.

7. **search_results.rs:38 band clearing** - Same as BLOCKING 3 above but for the explicit call-out.
   Fix: Add to dead code removal list.

### SUGGESTION

8. **TabBarAction enum (gesture/mod.rs:257-264)** becomes entirely unused after removing `tab_bar_hit_test` and `search_header_hit_test`. Should be explicitly listed for removal.
   Fix: Add to dead code removal.

9. **CH_LIST_TOP has ~64 references, not 25+**. The plan understates the blast radius. The build will catch any missed reference, so this is informational.
   Fix: Update count to ~64 for accuracy.

## Resolution

All BLOCKING and HIGH items addressed in updated plan.md:
- B1: Added search_results.rs dead imports and constants to dead code removal
- B2: Added app/render.rs TAB_BAR_TOP debug log reference removal
- B3/B4/B7: Added search_results.rs band-clearing loop (line 38) and header text rendering (lines 46-68) to dead code removal. Removed touch_release.rs from modified files (no references).
- B5: Added gesture/mod.rs dead imports (CH_LIST_TOP, back_arrow_hit_test, tab_btn_rects, TAB_BAR_TOP) to dead code removal
- B6: Added "Back button dual-purpose routing" section explaining mode-dependent behavior
- S8: Added TabBarAction enum to dead code removal
- S9: Updated CH_LIST_TOP reference count to ~64
