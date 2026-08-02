# Plan: feat-per-book-settings

All existing settings remembered per book. Global values become defaults for new books. No new UI.

## Current state (verified in source)

| Setting | Storage | Slider handler |
|---|---|---|
| Font size | `AppConfig.font_size` | `panel/callbacks/font.rs:33` calls `save_config(cfg)` |
| Brightness | `AppConfig.brightness` | `panel/callbacks/sliders.rs:28` calls `save_config(cfg)` |
| Volume | `AppConfig.volume` | `panel/callbacks/sliders.rs:43` calls `save_config(cfg)` |
| TTS rate | `AppConfig.tts_rate` | `panel/callbacks/sliders.rs:58` calls `save_config(cfg)` |
| TTS voice | `AppConfig.voices[lang]` | `panel/callbacks/voice.rs:64` calls `save_config(cfg)` |
| TTS lang | `AppConfig.tts_lang` | `meta.rs:58-60` calls `save_config(cfg)` via `apply_book_voice` |
| Natural scroll | `AppConfig.natural_scroll` | stored in config |
| Panel transition | `AppConfig.panel_transition` | stored in config |
| Reading auto sleep | `AppConfig.reading_auto_sleep_secs` | `panel/callbacks/sleep.rs:35` calls `save_config(cfg)` |

Every slider handler calls `save_config(cfg)` which writes the single global file. `apply_book_voice` at `meta.rs:58-60` also writes global config when voice/lang changes during book open. Per-book positions are already separate (`positions` file at `config.rs:20`).

## Design decisions

- **D1**: Global values become defaults; slider writes per-book only while book open.
- **D2**: Separate `booksettings` file, not fields on `positions`.
- **D3**: Store adjustable settings as pipe-delimited key=value, same keys as config.
- **D4**: Cache key unchanged (font_size already in `{hash}_{font}.bin`).
- **D5**: Save per-book when book open, save global when in picker.
- **D6**: `tts_lang` is NOT stored per-book. `apply_book_voice` already sets it per-book based on book language detection. Storing it per-book would fight with that auto-detection.

## Data model

New `kothok/src/data/persistence/book_settings.rs`:

```rust
/// Per-book settings override. Only the adjustable fields -- not
/// onboarding_version (app state), voices (language-level map, not book-level),
/// or tts_lang (auto-detected from book language by apply_book_voice).
pub struct BookSettings {
    pub font_size: Option<i32>,
    pub brightness: Option<i32>,
    pub volume: Option<i32>,
    pub tts_rate: Option<i32>,
    pub tts_voice: Option<String>,
    pub natural_scroll: Option<bool>,
    pub reading_auto_sleep_secs: Option<u32>,
    pub panel_transition: Option<String>, // stored as key string, converted via PanelTransition::from_key()
}
```

Using `Option` so missing keys fall back to the global default. A book with no entry at all gets `None` for everything -> global config as-is.

File `/mnt/onboard/.adds/booksettings`, one line per book:

```
/mnt/onboard/books/Foo.epub|font_size=42|brightness=60|volume=80|tts_rate=55|tts_voice=Emma (English)|natural_scroll=1|reading_auto_sleep=300|panel_transition=reagl
```

API mirroring `position.rs`:

```rust
pub fn load_book_settings(file: &Path, book_path: &str) -> BookSettings;
pub fn save_book_settings(file: &Path, book_path: &str, cfg: &AppConfig);
```

`load_book_settings` returns a `BookSettings` with `None` for missing keys or missing file.
`save_book_settings` rewrites the book's line (same filter-append pattern as `save_position`), writing all adjustable fields from the current `AppConfig`.

`apply_book_settings(cfg: &mut AppConfig, settings: &BookSettings)` overwrites each `cfg.field` with `settings.field` when `Some`. For `panel_transition`, converts the stored key string via `PanelTransition::from_key()` before assigning.

`BOOK_SETTINGS_FILE: &str = "/mnt/onboard/.adds/booksettings"` added to `config.rs`.

## Implementation phases

### Phase 1 - storage (no behaviour change)

1. `data/persistence/book_settings.rs` + `BookSettings` struct with `Option` fields.
2. `load_book_settings`, `save_book_settings`, `apply_book_settings` functions.
3. Re-export from `data/persistence.rs`.
4. Add `BOOK_SETTINGS_FILE` constant to `config.rs`.
5. Tests: roundtrip, missing file returns all-None, multiple books, overwrite, unknown keys ignored, partial fields (only font_size saved, rest None).

### Phase 2 - load on book open

6. `open_book_from_picker` (`loop_run/picker/open_book.rs:98`): after `load_position`, call `load_book_settings` + `apply_book_settings` to overlay per-book values on `cfg`. Push the updated values to the UI (font_size_val, brightness_val, tts_speed, volume_val, etc.).
7. `init_reader_and_config` (`setup.rs:243`): for the startup path (last book auto-open), same load + apply before the initial `body_px`/`head_px`/`line_h` computation at `setup.rs:247-249`.
8. **NEW CODE** in picker-return path (`loop_run/status.rs:158` and `loop_run/power.rs:235`): after `st.picker_active = true`, reload global config via `load_config_from_base(CONFIG_FILE, device_default_font)` and apply the restored values to the UI properties (font_size_val, brightness_val, volume_val, tts_speed, etc.). This is the only way to restore the "new book defaults" when returning from a book that had different settings.

### Phase 3 - save on change

9. Each slider handler that currently calls `save_config(cfg)` instead calls a new `save_settings_for_current_book(st, cfg)` that writes per-book when `!st.picker_active`, or global when in picker.
10. Font slider (`panel/callbacks/font.rs:33`): replace `save_config(cfg)` with `save_settings_for_current_book`.
11. Brightness slider (`panel/callbacks/sliders.rs:28`): same.
12. Volume slider (`panel/callbacks/sliders.rs:43`): same.
13. TTS rate slider (`panel/callbacks/sliders.rs:58`): same.
14. Voice cycle (`panel/callbacks/voice.rs:64`): same.
15. Sleep cycle (`panel/callbacks/sleep.rs:35`): same.
16. `apply_book_voice` (`meta.rs:58-60`): replace `save_config(cfg)` with per-book save. This fires during book open (`open_book.rs:90`) and would otherwise write book-specific voice to global config.
17. Connectivity toggles (WiFi, BT): these are hardware state, not book content. They stay global-only. `save_config(cfg)` for those stays unchanged.

### Helper function

```rust
fn save_settings_for_current_book(st: &LoopState, cfg: &AppConfig) {
    if st.picker_active {
        save_config(cfg);
    } else {
        save_book_settings(
            Path::new(BOOK_SETTINGS_FILE),
            &st.current_book_path,
            cfg,
        );
    }
}
```

Placed in `data/config.rs` alongside `save_config`, accessible by all slider handlers.

## Files to change

| File | Change |
|---|---|
| `data/persistence/book_settings.rs` | **NEW** - BookSettings struct, load, save, apply |
| `data/persistence.rs` | Re-export book_settings module |
| `data/config.rs` | Add `BOOK_SETTINGS_FILE` constant, add `save_settings_for_current_book` helper |
| `meta.rs` | Replace `save_config(cfg)` at line 59 with per-book save in `apply_book_voice` |
| `loop_run/picker/open_book.rs` | Load per-book settings after position load, push to UI |
| `loop_run/status.rs` | Reload global config on picker return (new code at line 158) |
| `setup.rs` | Load per-book settings in startup path |
| `panel/callbacks/font.rs` | Replace `save_config(cfg)` with per-book save |
| `panel/callbacks/sliders.rs` | Replace `save_config(cfg)` with per-book save |
| `panel/callbacks/voice.rs` | Replace `save_config(cfg)` with per-book save |
| `panel/callbacks/sleep.rs` | Replace `save_config(cfg)` with per-book save |

## What stays unchanged

- `PAD_LEFT` stays const (no dynamic margins)
- `LINE_HEIGHT_SCALE` stays const (no line spacing setting)
- Cache key stays `{hash}_{font_size}.bin`
- All Slint UI files (no new sliders, no new properties)
- `positions` file format
- Connectivity handlers (WiFi, BT) -- global only

## Risks

- **Brightness jumping on book switch.** If book A has brightness 30 and book B has brightness 80, switching books changes frontlight visibly. This matches the requirement (each book has its own settings). If it proves annoying, brightness can be made global-only in a follow-up.
- **File growth.** One line per book opened, same as positions. Negligible.
- **Crash during save.** Same filter-append-rewrite pattern as positions. Worst case: one book's settings lost, global config untouched.

## Out of scope

- Per-book font face, justification, hyphenation
- Per-book line spacing, margins (no UI to set them)
- Dynamic margins (PAD_LEFT stays const)
- "Apply to all books" action
- Per-book TTS language (`apply_book_voice` already handles this via book language detection)
