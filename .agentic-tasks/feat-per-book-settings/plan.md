# Plan: feat-per-book-settings

Remember font size, line spacing and margins **per book** instead of one global value.

## Current state (verified in source)

| Setting | Today | Storage |
|---|---|---|
| Font size | global slider, 20..60 step 2 | `AppConfig.font_size` in `/mnt/onboard/.adds/config` ([config.rs:28](kothok/src/data/config.rs#L28)) |
| Line spacing | **not a setting** - hardcoded `LINE_HEIGHT_SCALE = 1.4` ([layout.rs:35](kothok/src/rendering/layout.rs#L35)) | - |
| Margins | **not a setting** - hardcoded `PAD_LEFT = 24`, baked into `text_w()` at `init_layout()` in a `OnceLock` ([layout.rs:114-126](kothok/src/rendering/layout.rs#L114-L126)) | - |

Per-book data already exists: `positions` file keyed by book path
([position.rs](kothok/src/data/persistence/position.rs)), and the offset cache is keyed
`{book_hash}_{font_size}.bin` ([cache.rs:23](kothok/src/data/persistence/cache.rs#L23)).

So this feature is two jobs, not one:

1. **Make line spacing and margins settings at all** (new sliders + dynamic layout).
2. **Key all three per book** (new store + load on open + save on change).

Job 2 alone is small. Job 1 is where the real work is, because `text_w()` is currently a
write-once global.

## Design decisions

**D1 - Global values become defaults, per-book values are overrides.**
A book with no saved entry opens at the current global `AppConfig` values (which stay as
the "new book" default). Adjusting a slider while a book is open writes the per-book
entry *only* - it no longer touches `AppConfig`. This keeps existing installs behaving
identically on first open of each book, and matches KOReader/Kobo convention.
Rejected: per-book-only with no global (first open of every book would jump to a
hardcoded default and lose the user's calibrated font size).

**D2 - Separate `booksettings` file, not extra fields on the `positions` line.**
`positions` is rewritten on every page turn / cursor commit; style is written on slider
release. One responsibility per file (§1). Format tolerates unknown keys so future
settings append without a migration.

**D3 - Side margins only.** `PAD_TOP = 110` is the header band height shared with
`content.slint`; making it settable means moving Slint geometry too. Out of scope -
noted in "Out of scope" below.

**D4 - Style is part of the offset-cache key.** Line spacing and margins repaginate just
like font size does, so `cache_path` must include them or page numbers go wrong after a
change. Filename becomes `{hash}_{font:04}_{line:03}_{margin:03}.bin`.

## Data model

New `kothok/src/data/persistence/book_style.rs`:

```rust
/// Per-book layout style. `line_pct` is line height as a percentage of the body
/// font size (140 == the old `LINE_HEIGHT_SCALE` of 1.4); `margin_px` is the
/// outer side pad (24 == the old `PAD_LEFT`). Both defaults reproduce the
/// pre-feature layout exactly, so an untouched book paginates identically.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BookStyle {
    pub font_size: i32, // 20..=60, even
    pub line_pct: i32,  // 110..=200, step 5
    pub margin_px: i32, // 8..=96, step 8
}
```

File `/mnt/onboard/.adds/bookstyles`, one line per book, same
read-filter-append-rewrite shape as `save_position`:

```
/mnt/onboard/books/Foo.epub|font=42|line=150|margin=32
```

API mirroring `position.rs`:

```rust
pub fn load_book_style(file: &Path, book_path: &str) -> Option<BookStyle>;
pub fn save_book_style(file: &Path, book_path: &str, style: &BookStyle);
```

Unknown `key=` pairs are skipped, missing keys fall back to the passed defaults - the
same forward/backward tolerance `load_position` already has for field 8.

## Making margins dynamic

The blocker: `PAD_LEFT` is a `const` and `text_w()` reads a write-once `OnceLock`.

Change [layout.rs](kothok/src/rendering/layout.rs):

```rust
/// Default side margin. Also the fixed margin for non-book screens (splash,
/// picker) - only the reading column follows the per-book value.
pub const MARGIN_DEFAULT_PX: usize = 24;
static MARGIN_PX: AtomicUsize = AtomicUsize::new(MARGIN_DEFAULT_PX);

pub fn pad_left() -> usize { MARGIN_PX.load(Ordering::Relaxed) }
pub fn set_margin_px(px: usize);          // clamped 8..=96
pub fn text_w() -> usize { fb_w() - 2 * (pad_left() + GUTTER_W + GUTTER_PAD) }
```

`init_layout` keeps `fb_w`/`fb_h` in the `OnceLock` instead of the derived `text_w`.
`content_h()` is unchanged.

Call sites of the `PAD_LEFT` const to convert to `pad_left()` - all reading-column
geometry:

- [text_overlay.rs:38](kothok/src/rendering/text_overlay.rs#L38), `:70`, `:160`, `:543`, `:579`
- [touch_dispatch.rs:20](kothok/src/loop_run/touch_dispatch.rs#L20) (progress-bar x)

Call sites that must stay pinned to `MARGIN_DEFAULT_PX` (not book content):

- [splash.rs:68](kothok/src/rendering/splash.rs#L68) `SPLASH_MARGIN` (const-evaluated, and
  `splash/tests.rs` asserts against it)

`text_w()` is read on the main thread only. The offset worker takes a `ScreenLayout`
snapshot ([offsets.rs:33](kothok/src/rendering/layout/state/offsets.rs#L33)) and calls
`count_chapter_pages`, never the global - so no locking is needed. **`set_margin_px` MUST
be called before `screen_layout()` is snapshotted** for the worker, otherwise the worker
paginates at the old width.

## Implementation phases

### Phase 1 - storage (no behaviour change)

1. `data/persistence/book_style.rs` + `BookStyle` with `Default` = `{36, 140, 24}` and a
   `clamped()` constructor; re-export from `data/persistence.rs`.
2. `BookStyle::from_config(&AppConfig)` - the D1 default path.
3. `cache_path(book_path, &BookStyle)` - new filename. Update the two callers
   ([book_session.rs:114](kothok/src/book_session.rs#L114),
   [offsets.rs:39](kothok/src/rendering/layout/state/offsets.rs#L39)) and
   `spawn_offset_computation`'s `font_size: i32` param -> `style: BookStyle`.
   `load_any_offset_cache` already matches on the `{hash}_` prefix and picks the newest
   file, so old-format caches stay harmless (they simply never win once a new one exists).
4. Tests: roundtrip, unknown-key tolerance, multiple books, missing file, clamping.

### Phase 2 - dynamic layout globals

5. `MARGIN_PX` atomic + `pad_left()` + `text_w()` rework, `set_margin_px`.
6. Convert the `PAD_LEFT` call sites listed above; leave splash pinned.
7. Derive `line_h` from `line_pct` everywhere it is currently
   `font_size * LINE_HEIGHT_SCALE`: [setup.rs:249](kothok/src/setup.rs#L249),
   [font.rs:68](kothok/src/panel/callbacks/font.rs#L68). Add
   `fn line_h_for(style: &BookStyle) -> i32` in `layout` so there is one home for the
   formula (§4). Keep `LINE_HEIGHT_SCALE` as the documented default only.
8. Test: `build_state` at margin 8 / 24 / 96 and line_pct 110 / 140 / 200 all produce
   non-empty pages and monotonically wider/narrower rows (extends the existing
   `line_h` loop at [layout/tests.rs:203](kothok/src/rendering/layout/tests.rs#L203)).

### Phase 3 - load per book on open

9. `LoopState` gains `pub style: BookStyle` (replacing the implicit
   `body_px`/`head_px`/`line_h` source of truth; those three stay as derived cache
   fields, recomputed by one `fn apply_style(st: &mut LoopState)`).
10. `open_book_from_picker` ([picker/open_book.rs:98](kothok/src/loop_run/picker/open_book.rs#L98)):
    next to `load_position`, load the style, fall back to `BookStyle::from_config(cfg)`,
    then `set_margin_px` + `apply_style` **before** `open_book_session`.
11. Same in the startup path: [setup/book_init.rs](kothok/src/setup/book_init.rs) /
    `init_reader_and_config` - the initial `body_px`/`head_px`/`line_h` at
    [setup.rs:247-249](kothok/src/setup.rs#L247-L249) must come from the resolved book
    style, not `cfg.font_size`, or the first paint uses the global and then reflows.
12. Push the values to the UI on open: `reader.set_font_size_val`, plus the two new
    properties, so the panel sliders show the book's values.
13. Returning to the picker / opening another book re-resolves; closing a book restores
    `MARGIN_DEFAULT_PX` before picker paint.

### Phase 4 - UI + write on change

14. `control_panel.slint`: two `CompactSlider`s under "Font" -
    `Line` (`value-text: line-pct + "%"`, `panel-frac(4, f)`) and
    `Margin` (`value-text: margin-px + "px"`, `panel-frac(5, f)`).
    New `in property <int>`s on `ControlPanel` + `reader.slint` root, forwarded like
    `font-size-val` ([reader.slint:212](kothok/ui/reader.slint#L212)).
15. `callbacks.rs` `on_panel_frac`: route `4`/`5` into new cells alongside
    `font_frac_in` ([callbacks.rs:157-164](kothok/src/callbacks.rs#L157-L164)).
    `SLIDER_LINE: i32 = 4`, `SLIDER_MARGIN: i32 = 5` (0/1/2/3 are taken by
    brightness/tts-rate/font/volume).
16. Generalise [panel/callbacks/font.rs](kothok/src/panel/callbacks/font.rs) into
    `handle_style_sliders`: all three sliders share one debounce
    (`FONT_DEBOUNCE_MS`), one pending value, and one `apply_style_reflow` - which is
    today's `apply_font_reflow` with the anchor logic unchanged, plus
    `set_margin_px` before `build_state`.
17. On slider commit: `save_book_style(...)` for the open book. **Do not** call
    `save_config` for these three any more; `AppConfig.font_size` is now only the
    new-book default. Keep writing `cfg.font_size` when no book is open (picker).
18. Audio reload contract (AGENTS.md): `apply_style_reflow` must keep the existing
    `page_utterances` -> `Cmd::Reload` -> `Cmd::Seek` tail. Margin/line changes
    repaginate exactly like font, so this is non-negotiable.

## Test plan

Unit (desktop, `cargo test -p kothok-app`):

- `book_style_roundtrip`, `book_style_unknown_keys_ignored`,
  `book_style_missing_file_returns_none`, `book_style_multiple_books`,
  `book_style_clamps_out_of_range`
- `book_style_save_overwrites_previous`
- `cache_path_differs_per_style` - three styles, three distinct paths
- `default_style_reproduces_legacy_layout` - `build_state` with
  `{36,140,24}` equals the pre-feature `build_state(ch, 36.0, 28.08, 50)` pagination
- `text_w_tracks_margin` - `set_margin_px(8)` widens `text_w()` by exactly 32 vs 24
- `line_h_for` boundaries at 110 / 200

Device (per `device-test-workflow`):

- Open book A, set font 48 / line 180 / margin 48. Back to picker. Open book B - shows
  the global default, not A's values. Reopen A - A's values restored, same page.
- Change margin mid-chapter: page count updates, reading position anchor holds (the
  `apply_font_reflow` anchor path), TTS resumes on the right sentence.
- Reboot after a change (`sync` gotcha - see `deploy-sync-gotcha`), reopen: values persist.

## Risks

- **Repagination cost.** Margin/line changes rerun `spawn_offset_computation` exactly
  like a font change, so a long book shows the loading percentage again. Same cost as
  today's font slider - acceptable, but the debounce must cover all three sliders or a
  drag spawns a worker per tick.
- **Cache dir growth.** The style-keyed name multiplies possible cache files per book.
  Each is `4 * (chapters+1)` bytes - tens of KB worst case. Not pruning for now; if it
  matters, prune to the newest 4 per `{hash}_` prefix on book open.
- **Stale offsets from an in-flight worker.** Already handled by the existing pattern:
  replacing `st.offset_rx` drops the old receiver, and the old worker writes to the old
  style's cache path. The new cache key is what keeps that write from poisoning the new
  style.
- **`PAD_LEFT` misses.** Any call site left on the const silently draws at the old
  margin while wrapping at the new one (ragged/overflowing lines). The const is deleted
  and replaced by `MARGIN_DEFAULT_PX` + `pad_left()`, so the compiler finds every site.

## Out of scope

- Top/bottom margins (`PAD_TOP` is the Slint header band - D3).
- Per-book font *face*, justification, hyphenation.
- Per-book TTS voice - already handled separately by `apply_book_voice`.
- A "save as default / apply to all books" action. Worth a follow-up once per-book lands.
