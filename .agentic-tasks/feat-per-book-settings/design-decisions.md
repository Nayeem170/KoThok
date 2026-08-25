# Design Decisions: feat-per-book-settings

## D1 - Global values become defaults, per-book values are overrides

A book with no saved entry opens at the current global `AppConfig` values (which stay as the "new book" default). Adjusting a setting while a book is open writes the per-book entry *only* -- it no longer touches `AppConfig`. This keeps existing installs behaving identically on first open of each book, and matches KOReader/Kobo convention.

**Rejected: per-book-only with no global** -- first open of every book would jump to a hardcoded default and lose the user's calibrated preferences.

## D2 - Separate `booksettings` file, not extra fields on `positions`

`positions` is rewritten on every page turn / cursor commit; settings are written on slider release or toggle. One responsibility per file. Format tolerates unknown keys so future settings append without a migration.

**Rejected: extra fields on positions line** -- positions rewrites on every page turn; settings changes are rare. Mixing write frequencies risks data loss on crash during position commit.

## D3 - Store the full adjustable AppConfig subset, not individual key-value pairs

Rather than storing each setting as a separate `key=value` on the book line, store a mini-config block after the book path delimiter. This mirrors `AppConfig`'s own format and keeps the loader simple: parse the same keys into the same struct.

Format: one line per book, book path as prefix, then pipe-delimited key=value pairs:
```
/mnt/onboard/books/Foo.epub|font_size=42|brightness=60|volume=80|tts_rate=55|tts_lang=auto|tts_voice=Emma (English)|natural_scroll=1|reading_auto_sleep=300|panel_transition=reagl
```

**Rejected: JSON per book** -- heavier to parse, harder to grep by hand on device.
**Rejected: binary format** -- impossible to debug over telnet.

## D4 - Cache key unchanged

The offset cache is already keyed `{hash}_{font_size}.bin`. Font size is still part of the cache key and still global in the per-book sense (each book has its own font size, and the cache uses that). No change needed.

## D5 - Slider callbacks write per-book when book is open, global when in picker

Each slider handler (font, brightness, volume, tts_rate, voice, sleep, connectivity) checks whether a book is currently open. If yes, writes to the per-book file. If no (picker mode), writes to the global `AppConfig` as today.

**Rejected: always write both** -- that would overwrite the global default every time any book's setting changed, defeating the purpose of having a default.
