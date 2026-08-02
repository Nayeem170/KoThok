# Requirement: feat-per-book-settings

## User request

All existing settings remembered per book instead of globally. Global values become defaults for new books.

## Current behaviour

All settings are global: font size, brightness, volume, TTS rate, TTS voice, natural scroll, panel transition, reading auto sleep. Stored in a single `AppConfig` at `/mnt/onboard/.adds/config`. Every book opens with the same values. Changing any setting affects all books.

## Desired behaviour

- Every book gets its own copy of all settings.
- A book with no saved entry opens at the current global `AppConfig` values (identical to today).
- Adjusting any setting while a book is open writes the per-book entry only -- the global `AppConfig` values stay as the "new book" default.
- Opening a different book loads that book's saved values; the UI updates to reflect them.
- Returning to the picker restores global defaults.

## Settings to remember per book

- font_size (20..60 step 2)
- brightness (0..100)
- volume (0..100)
- tts_rate (0..100)
- tts_lang (string)
- tts_voice (string)
- natural_scroll (bool)
- reading_auto_sleep_secs (u32)
- panel_transition (string key)

Not per-book (truly global, not book-dependent):
- onboarding_version (app state, not a setting)
- voices map (voice assignments per language -- tied to language, not book)

## Clarification log

- User clarified: no new sliders for line spacing or margin. Only existing settings get per-book memory.
- User clarified: "rest settings wants per book" = all existing adjustable settings.

## Scope

In scope: per-book storage file, load-on-open, save-on-change, cache key unchanged (font size already in key).
Out of scope: new settings, new sliders, dynamic margins, line spacing, per-book font face.
