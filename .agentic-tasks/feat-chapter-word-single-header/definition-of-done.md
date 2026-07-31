# Definition of Done: feat-chapter-word-single-header

## Build
- [ ] `cross build` succeeds with no warnings from new code
- [ ] `cross test` passes (ALL tests, not just new ones)

## Convention compliance
- [ ] ASCII-only in all source files (no em dash, smart quotes, unicode)
- [ ] LF line endings (no CRLF)
- [ ] No comments unless explaining non-obvious WHY
- [ ] No fallback implementations
- [ ] Conventional commit message
- [ ] Branch named feat/chapter-word-single-header

## Requirement coverage
- [ ] Single 110px header replaces two-band layout - chapter_overlay.slint
- [ ] Tab pills (Chapters/Words) in Slint header center - chapter_overlay.slint
- [ ] Close button (brand-red) on right of header - chapter_overlay.slint
- [ ] Back button (brand-red) on left of header - chapter_overlay.slint
- [ ] Search results mode: tabs hidden, context title shown - chapter_overlay.slint
- [ ] CH_LIST_TOP = 110 (was 158) - chapter_list.rs

## Dead code removal
- [ ] paint_tab_bar() removed from word_list.rs
- [ ] paint_close_button() removed from word_list.rs
- ] tab_btn_rects() removed from word_list.rs
- [ ] TabBtn, CloseBtn structs removed from word_list.rs
- [ ] paint_back_arrow() removed from search_results.rs
- [ ] back_arrow_hit_test() removed from search_results.rs
- [ ] tab_bar_hit_test() removed from gesture/mod.rs
- [ ] search_header_hit_test() removed from gesture/mod.rs
- [ ] Tab bar dispatch removed from loop_run/search.rs handle_search_release()
- [ ] paint_tab_bar() calls removed from app/render.rs
- [ ] word_list.rs tab bar tests removed (active_tab_is_inked_and_inactive_is_not, tab_bar_fits_left_of_close_button)

## Test coverage
- [ ] chapter_list.rs straddling_row test updated for 0px gap
- [ ] All existing gesture/search/scroll tests pass with updated CH_LIST_TOP

## Scope
- [ ] No changes outside plan.md scope
- [ ] No unrelated refactoring
