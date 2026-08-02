# Definition of Done: feat-per-book-settings

## Build
- [ ] `cross build` succeeds with no warnings from new code
- [ ] `cross test` passes (ALL tests, not just new ones)

## Convention compliance
- [ ] ASCII-only in all source files (no em dash, smart quotes, unicode)
- [ ] LF line endings (no CRLF)
- [ ] No comments unless explaining non-obvious WHY
- [ ] No fallback implementations
- [ ] Conventional commit messages
- [ ] Branch named feat/per-book-settings

## Requirement coverage
- [ ] Per-book storage file created at `/mnt/onboard/.adds/booksettings` - `data/persistence/book_settings.rs:save_book_settings`
- [ ] Per-book settings loaded on book open (picker path) - `loop_run/picker/open_book.rs`
- [ ] Per-book settings loaded on startup (last-book path) - `setup.rs:init_reader_and_config`
- [ ] Per-book settings applied to UI on open - font_size_val, tts_speed, tts_voice, tts_voice_label pushed to reader
- [ ] Slider changes save per-book when book is open - `panel/callbacks/font.rs`, `sliders.rs` (tts_rate only), `voice.rs`
- [ ] Slider changes save global when in picker - same handlers, `picker_active` check
- [ ] Returning to picker restores global defaults - picker-return path reloads global config

## Test coverage
- [ ] book_settings_roundtrip - save and load all fields
- [ ] book_settings_missing_file_returns_all_none - no file -> all None
- [ ] book_settings_multiple_books - two books, each loads own values
- [ ] book_settings_save_overwrites_previous - same book saved twice
- [ ] book_settings_partial_fields - only some fields saved, rest None
- [ ] book_settings_unknown_keys_ignored - extra key=value on line skipped

## Scope
- [ ] No changes outside plan.md scope
- [ ] No unrelated refactoring
- [ ] No changes to PAD_LEFT, LINE_HEIGHT_SCALE, or layout.rs
- [ ] No changes to Slint UI files
- [ ] No changes to cache.rs (cache key unchanged)
