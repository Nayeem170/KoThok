# Iteration 1 Review

## Feedback

1. **HIGH** - `meta.rs:59` calls `save_config(cfg)` and is not in the files-to-change table. When `apply_book_voice` runs during book open, it will write book-specific voice/lang to the global config, defeating the per-book feature. `meta.rs` must be added to the change list, and `apply_book_voice`'s `save_config` must be replaced with the same per-book save helper.

2. **MEDIUM** - `tts_lang` field in `BookSettings` contradicts the out-of-scope note. Either remove `tts_lang` from `BookSettings` (since `apply_book_voice` handles it per-book via book language detection), or remove the out-of-scope claim and clarify the interaction.

3. **MEDIUM** - `panel_transition` type mismatch. `BookSettings.panel_transition` is `Option<String>` but `AppConfig.panel_transition` is `PanelTransition`. `apply_book_settings` must convert using `PanelTransition::from_key()`. The plan doesn't mention this.

4. **MEDIUM** - Phase 2 item 8 (picker-return reload of global defaults) reads as if the path already exists. Neither `status.rs:158` nor `power.rs:235` currently reloads global config. Clarify that this is new code to be added.

5. **MEDIUM** - No DoD document at `dod.md`. Pipeline requires standalone DoD with machine-checkable pass/fail criteria.
