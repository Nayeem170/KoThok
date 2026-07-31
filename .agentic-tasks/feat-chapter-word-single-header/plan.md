# Plan: feat-chapter-word-single-header

## Root cause

The chapter/word list overlay uses two stacked header bands: a 110px Slint-rendered header (back button + title) and a 48px Rust-painted band (tab pills + close button). Total header height: 158px (`CH_LIST_TOP`). This wastes vertical space and creates visual inconsistency.

## Fix approach

Merge both bands into a single 110px Slint header containing: back button (left, red), tab pills (center), close button (right, red). The Rust-painted tab bar and results header are eliminated. All header interaction moves to Slint callbacks.

### Slint changes

**chapter_overlay.slint**:
- Replace the static "Chapters" Text with tab pills ([Chapters] [Words]) and a close button (X)
- Add two new callbacks: `tab-switch(int)` for tab switching, `close-overlay()` for close button
- Add an `in-out property <bool> results-active: false` for search results mode
- In results mode: hide tabs, show context title ("<word>" - N matches), keep back + close
- Remove the separator line at y:107 (no longer at bottom of header area)
- Wire tab visibility to `active-tab`

**reader.slint**:
- Wire `tab-switch(int)` and `close-overlay()` callbacks to root
- Forward to appropriate state management in callbacks.rs

### Rust changes

**CH_LIST_TOP**: 158 -> 110. This impacts every list painter, hit-test, scrollbar, and scroll-max calculation. A comprehensive `CH_LIST_TOP` grep identified ~64 references (including tests) across the codebase. The build will catch any missed reference.

**Dead code removal**:

`word_list.rs`:
- Functions: `paint_tab_bar()`, `paint_tab_button()`, `paint_close_button()`, `tab_btn_rects()`
- Structs: `TabBtn`, `CloseBtn`
- Constants: `TAB_BAR_TOP`, `TAB_BTN_H`, `TAB_BTN_Y`, `TAB_BTN_PAD`, `TAB_GAP`, `TAB_FONT_PX`, `CLOSE_BTN_PX`
- Tests: `active_tab_is_inked_and_inactive_is_not`, `tab_bar_fits_left_of_close_button`

`search_results.rs`:
- Functions: `paint_back_arrow()`, `back_arrow_hit_test()`
- Dead import from word_list: `TAB_BAR_TOP`, `TAB_BTN_H` (keep `INK` and `TAB_BORDER` - both used at line 76 in result row rendering)
- Dead constants: `BACK_W` (line 17), `BACK_X` (line 18), `HEADER_PX` (line 21)
- Remove `TAB_BAR_TOP..CH_LIST_TOP` band-clearing loop at line 38 inside `paint_search_results()`
- Remove header text rendering block (lines 48-68: format header, text positioning, blit)

`gesture/mod.rs`:
- Functions: `tab_bar_hit_test()`, `search_header_hit_test()`
- Enum: `TabBarAction` (lines 257-264)
- Dead imports: `CH_LIST_TOP` (line 3), `back_arrow_hit_test` (line 7), `tab_btn_rects` (line 8), `TAB_BAR_TOP` (line 9)

`loop_run/search.rs`:
- Remove tab bar hit-test dispatch block (lines 49-74: `match gesture::tab_bar_hit_test(...)`)
- Remove search results back-arrow hit-test dispatch (lines 26-31: `gesture::search_header_hit_test(...)`)
- Keep the rest of `handle_search_release()` (result row hit-test, word list hit-test, scroll handling)

`app/render.rs`:
- Remove both `paint_tab_bar()` calls
- Remove debug log reference to `word_list::TAB_BAR_TOP` at line 127 (constant no longer exists)

`chapter_list.rs`:
- Update comment referencing 48px gap

**Modified code**:
- `chapter_list.rs`: CH_LIST_TOP = 110
- `word_list.rs`: `paint_word_list()` body area starts at 110 (automatic via CH_LIST_TOP). The `paint_tab_bar()` function (which contained the `TAB_BAR_TOP..CH_LIST_TOP` band-clearing at line 74) is entirely removed.
- `search_results.rs`: `paint_search_results()` body starts at 110. Remove the `TAB_BAR_TOP..CH_LIST_TOP` band-clearing loop (line 38-41) and the header text rendering (lines 46-68). The `CH_LIST_TOP..list_bottom` body fill remains.
- `loop_run/search.rs`: `search_scroll_max()` still works via updated CH_LIST_TOP
- `loop_run/callbacks.rs`: new `tab-switch(int)` and `close-overlay()` callback handling
- `loop_run/touch_dispatch.rs`: any CH_LIST_TOP references update automatically

**Back button dual-purpose routing**:
The existing Slint back button currently always calls `close-to-book()`. In the new design, it must serve two purposes:
- Normal mode (chapters/words): close overlay, return to reading
- Results mode (search results): back from results, return to word list (overlay stays open)

Implementation: the Rust handler for the back button callback checks `st.search_results_active`:
- If true: call `back_from_results(st)` (existing function in search.rs)
- If false: call `reader.set_chapter_overlay_open(false)` (close overlay)

The close button (X, right side) always calls `close-overlay()` which unconditionally closes the overlay and returns to reading.

### Side effects

- Net vertical space reclaimed: 48px more room for list content
- No changes to bottom-strip Open button behavior
- No changes to chapter row rendering or scrollbar
- No changes to reading mode header (content.slint, audio_player.slint)
- No changes to picker or control panel

## Out of scope

- No changes to tab pill visual design (keep existing picker-pill language)
- No changes to word list row rendering
- No changes to search result row rendering
- No changes to scrollbar behavior
- No changes to power/sleep handlers
- No changes to touch_release.rs (it has no CH_LIST_TOP or tab_bar references)

## Risks

- Medium: CH_LIST_TOP change touches ~64 references across the codebase (including tests). A missed reference causes layout breakage. The build will catch any missed reference.
- Low: Slint callback wiring for tabs/back requires careful coordination with Rust state. Back button dual-purpose routing must check `search_results_active`.

## DoD

- [ ] `chapter_overlay.slint` header has back button (red circle), tab pills (Chapters/Words), close button (red circle)
- [ ] Tab pills use the existing picker-pill language (active = ink/white, inactive = white/border)
- [ ] Close button uses brand-red (#F42A41) matching the app's primary action button color
- [ ] Search results mode: tabs hidden, context title visible, back + close visible
- [ ] Back button in normal mode closes overlay; back button in results mode calls back_from_results
- [ ] CH_LIST_TOP = 110 (was 158)
- [ ] No Rust-painted tab bar or results header in the 110-158px band
- [ ] paint_tab_bar() and paint_close_button() removed
- [ ] paint_back_arrow() and back_arrow_hit_test() removed
- [ ] tab_bar_hit_test() and search_header_hit_test() removed
- [ ] TabBarAction enum removed
- [ ] search_results.rs dead imports and constants removed (TAB_BAR_TOP, TAB_BTN_H, BACK_W, BACK_X, HEADER_PX)
- [ ] gesture/mod.rs dead imports removed (CH_LIST_TOP, back_arrow_hit_test, tab_btn_rects, TAB_BAR_TOP)
- [ ] app/render.rs TAB_BAR_TOP debug log reference removed
- [ ] `cross build` succeeds
- [ ] `cross test` passes (336 passed)
- [ ] Conventional commit message
- [ ] Branch from develop, not main
- [ ] Git clean after commit
- [ ] Dead Rust code removed (unused imports, dead functions)
- [ ] straddling_row header gap test updated for 0px gap
