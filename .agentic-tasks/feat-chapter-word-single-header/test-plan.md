# Test Plan: feat-chapter-word-single-header

## Existing tests affected

### Tests to remove (functionality deleted)
- `word_list.rs::active_tab_is_inked_and_inactive_is_not` - tab bar painting removed
- `word_list.rs::tab_bar_fits_left_of_close_button` - tab bar layout removed

### Tests to update
- `chapter_list.rs::straddling_row_does_not_paint_into_the_header_gap` - the 48px gap no longer exists. The header gap is now 0px. Test must verify no straddling row paints into the Slint header area (0..CH_LIST_TOP).

### Tests unaffected (pass via CH_LIST_TOP constant change)
- All 5 `chapter_list_hit_test_*` tests (use CH_LIST_TOP directly)
- All 4 `word_list_hit_test_*` tests (use CH_LIST_TOP directly)
- All 3 `results_hit_test_*` tests (use CH_LIST_TOP directly)
- `chapter_list.rs::straddling_row_does_not_paint_over_the_open_button_strip` (uses list_bottom, unaffected)
- All scrollbar tests (use CH_LIST_TOP, unaffected)
- All gesture tests (no tab_bar references)
- All app tests (no overlay-specific tests)

## New test scenarios

### TS-01: Tab pill rendering via Slint (manual device test)
- Open chapter overlay
- Verify: two tab pills visible ("Chapters" and "Words")
- Tap "Words" tab
- Verify: active pill inks, inactive pill shows border
- Tap "Chapters" tab
- Verify: active pill switches back

### TS-02: Close button closes overlay
- Open chapter overlay (any tab)
- Tap close button (X, right side)
- Verify: overlay closes, reading view resumes

### TS-03: Back button in normal mode closes overlay
- Open chapter overlay
- Tap back button (left side, red circle)
- Verify: overlay closes, reading view resumes

### TS-04: Back button in results mode returns to word list
- Open chapter overlay -> tap Words -> tap a word with search results
- Verify: search results view appears with context title
- Tap back button (left side)
- Verify: returns to word list (not reading view), overlay stays open

### TS-05: Close button in results mode exits overlay
- In search results view
- Tap close button (X, right side)
- Verify: overlay closes entirely, reading view resumes

### TS-06: Search results header (no Rust painting)
- In search results view
- Verify: no 110-158px Rust-painted band visible
- Verify: back button (Slint) visible on left
- Verify: context title ("<word>" - N matches) visible in header
- Verify: close button (X) visible on right

### TS-07: List content starts at 110px
- Open chapter overlay with many chapters
- Scroll list
- Verify: first row starts immediately below header, no 48px gap

### TS-08: Chapter list hit test with CH_LIST_TOP=110
- Automated: `chapter_list_hit_test_first_row` passes (tapping at y=110 returns row 0)
- Automated: `chapter_list_hit_test_above_list_returns_none` passes (tapping at y=109 returns None)

### TS-09: Word list hit test with CH_LIST_TOP=110
- Automated: `word_list_hit_test_first_row` passes (tapping at y=110 returns word 0)
- Automated: `word_list_hit_test_above_returns_none` passes (tapping at y=109 returns None)

### TS-10: Search results hit test with CH_LIST_TOP=110
- Automated: `results_hit_test_first_row` passes
- Automated: `results_hit_test_above_returns_none` passes
- Automated: `results_hit_test_respects_scroll` passes

### TS-11: Straddling row does not paint into header
- Automated: updated `straddling_row_does_not_paint_into_the_header_gap` verifies 0px gap
- Row that straddles y=110 must not paint above CH_LIST_TOP

### TS-12: Scrollbar calculation with reduced list height
- Automated: scrollbar tests pass with CH_LIST_TOP=110 (taller list area = more visible rows)
- `scrollbar_y_to_scroll_bottom_equals_scroll_max` passes

### TS-13: Dead code compilation
- Automated: `cross build` succeeds with no warnings about unused imports
- No references to removed functions (`paint_tab_bar`, `paint_close_button`, `paint_back_arrow`, `back_arrow_hit_test`, `tab_bar_hit_test`, `search_header_hit_test`)
- No references to removed constants (`TAB_BAR_TOP`, `TAB_BTN_H`, `TAB_BTN_Y`, `BACK_W`, `BACK_X`, `HEADER_PX`, `CLOSE_BTN_PX`)
- No references to removed enum (`TabBarAction`)

### TS-14: Brand red color on close button
- Device test: close button circle is brand-red (#F42A41), matching back button

### TS-15: Empty word list state
- Open chapter overlay -> Words tab
- With no words indexed, verify empty state renders correctly (no tab bar artifacts)

### TS-16: Single chapter book
- Open chapter overlay with a book having only 1 chapter
- Verify: single row renders at y=110, scrollbar hidden (existing test covers this)
