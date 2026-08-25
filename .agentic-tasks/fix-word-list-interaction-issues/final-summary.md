# Final Summary: fix-word-list-interaction-issues

## Branch
fix/word-list-interaction-issues -> develop (merged with --no-ff)

## Commit
bf0d418 feat: add word list scrollbar, selection visual, and fix search crashes

## Files changed
27 files, +1817 / -411

### New files
- kothok/src/data/word_index.rs (130 lines) - word index builder
- kothok/src/loop_run/search.rs (167 lines) - search/jump logic
- kothok/src/rendering/layout/state/body.rs (427 lines) - body build extracted from state.rs
- kothok/src/rendering/search_results.rs (251 lines) - search results painter
- kothok/src/rendering/word_list.rs (296 lines) - word list + tab bar painter

### Modified files (key)
- data/library.rs - pre-build body before word index, CACHE_FORMAT v6
- rendering/chapter_list.rs - shared paint_scrollbar function
- rendering/layout/state.rs - split into body.rs + rows.rs
- loop_run/touch_dispatch.rs - scroll max formula fix
- app/render.rs - passes selected_word to paint_word_list

## Features
1. Scrollbar for word list and search results (visual-only, matching chapter list)
2. Selection visibility (inverted pill: ink fill + white text)
3. Crash fix: body timing (pre-build before index) + UTF-8 char boundary safety
4. Cursor fix: jump_to_occurrence scans rows for hit offset instead of first_text_row

## Tests
323 passed, 0 failed

## Pipeline steps
S0 -> S1 -> S2 -> S3(1 iter) -> S3.5(2 iter) -> S4 -> S5(1 iter) -> S6 -> S7(confirmed) -> S8

## Caveats
- Diff exceeds 400-line PR soft limit (27 files) due to body.rs/rows.rs refactoring included in branch
- develop is 34 commits ahead of origin/develop (not pushed)
