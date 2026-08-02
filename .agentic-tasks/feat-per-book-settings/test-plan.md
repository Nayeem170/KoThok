# Test Plan: feat-per-book-settings

## Unit tests (desktop, `cross test -p kothok-app`)

### Storage layer

- `book_settings_roundtrip`: save all fields for one book, load, verify every field matches
- `book_settings_missing_file_returns_all_none`: load from nonexistent path, all fields None
- `book_settings_multiple_books`: save for books A and B with different values, load A gets A's values, load B gets B's values
- `book_settings_save_overwrites_previous`: save book A with font=30, then font=48, load returns 48
- `book_settings_partial_fields`: save with only font_size set, load returns Some for font_size and None for everything else
- `book_settings_unknown_keys_ignored`: line has `future_key=foo` after known keys, ignored without error

### apply_book_settings

- `apply_overrides_matching_fields`: settings with Some values override cfg; None fields leave cfg unchanged
- `apply_panel_transition_converts_from_key`: `panel_transition=Some("reagl")` sets cfg.panel_transition via `from_key()`

## Device tests (per deploy-usb workflow)

### Per-book isolation

- Open book A, set font 48 / brightness 60 / volume 80. Return to picker. Open book B -- shows global defaults, not A's values. Return to picker. Reopen A -- A's values restored.

### Picker restores defaults

- Open a book with font 48. Return to picker. Font slider shows global default (36 or whatever). Open a new book (no saved settings) -- also shows global default.

### Settings survive reboot

- Set per-book settings, sync, reboot. Reopen book -- values persisted.

### No regression on existing behaviour

- Open a book that has never had per-book settings saved. Opens exactly as before (global defaults, same pagination, same reading position).

### UI updates on load

- Open a book with saved brightness 30. Frontlight should be at 30, slider should show 30%.

### Startup path

- Last-opened book has per-book settings. App restarts, reopens that book with per-book values applied.

### Audio sync after book switch

- Open book A with font 36, start TTS playback, note the auto page-turn timing. Open book B with font 48 (larger font = fewer words per page). Verify auto page-turn fires at the correct sentence boundary for B's layout, not A's. No stale TTS markers from the previous book.

### apply_book_voice and per-book voice interaction

- Open book A (English), manually set voice to "Guy". Return to picker, reopen A. Voice should still be "Guy" (the saved per-book value), not the auto-detected default for English. The `apply_book_voice` auto-detection runs on open but must not clobber the user's explicit voice choice that was saved per-book.
