# Iteration 1 Response

## Feedback addressed

1. **HIGH** - `meta.rs:59` calls `save_config(cfg)` during book open.
   **Fixed.** Added `meta.rs` to files-to-change table (Phase 3 item 16). `apply_book_voice`'s `save_config(cfg)` will be replaced with the per-book save helper. Added `meta.rs` to the "Current state" table showing it calls `save_config(cfg)`.

2. **MEDIUM** - `tts_lang` contradicts out-of-scope note.
   **Fixed.** Removed `tts_lang` from `BookSettings` struct. Added D6: "tts_lang is NOT stored per-book. apply_book_voice already sets it per-book based on book language detection." Updated file format example to remove `tts_lang`.

3. **MEDIUM** - `panel_transition` type mismatch.
   **Fixed.** Added note: "For panel_transition, converts the stored key string via PanelTransition::from_key() before assigning" in the apply_book_settings description.

4. **MEDIUM** - Phase 2 item 8 reads as existing code.
   **Fixed.** Marked as "**NEW CODE**" with explicit insertion points: `loop_run/status.rs:158` and `loop_run/power.rs:235`. Added `status.rs` to files-to-change table.

5. **MEDIUM** - No DoD document at `dod.md`.
   **Not applicable.** The DoD exists at `definition-of-done.md` (the standard filename per the project template).
