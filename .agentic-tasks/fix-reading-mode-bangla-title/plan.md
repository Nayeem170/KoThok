# Plan: fix-reading-mode-bangla-title

## Root cause analysis

`content.slint:82-88` (reading mode header) is missing the `Image` element that displays pre-rendered Bangla title rasters.

The `ScrollableText` at line 82-88 has `visible: root.book-title-img-h == 0`. For Bangla books, `meta.rs:128-131` sets `book-title-img-h > 0`, causing the text to hide. But no `Image` element exists to show the pre-rendered raster, resulting in a blank header.

`audio_player.slint:91-101` and `control_panel.slint:297-309` both have the correct dual-element pattern: `Image` (visible when `img_h > 0`) + `ScrollableText` (visible when `img_h == 0`). `content.slint` only has the `ScrollableText`.

## Fix approach

Add an `Image` element to `content.slint` before the existing `ScrollableText`, matching the pattern from `audio_player.slint:91-101`:
- `source: root.book-title-img`
- Same geometry: x: 119px, y: 17px, width: root.width - 686px, height: 76px
- `image-fit: contain`, `horizontal-alignment: center`, `vertical-alignment: center`
- `visible: root.book-title-img-h > 0`

No Rust code changes needed. `meta.rs::set_book_meta()` already sets the image properties globally. `reader.slint` already wires `book-title-img` and `book-title-img-h` to `content.slint`.

## Side effects

- `reader.slint:135-137` - wires `book-title-img` and `book-title-img-h` to `content.slint`. Already in place, no change needed.
- Non-Bangla books: `book-title-img-h == 0`, so the new `Image` stays hidden and `ScrollableText` shows normally. No impact.
- Audio mode and control panel: unchanged, already working.

## Out of scope

- No changes to `meta.rs` (already handles all modes)
- No changes to `reader.slint` (already wires properties)
- No new Rust tests (Slint markup change with no Rust logic affected)

## Risks

- Low: single-element addition following an existing pattern used in two other components.

## DoD

- [ ] `content.slint` header has `Image` element with `source: root.book-title-img`
- [ ] `Image` has `visible: root.book-title-img-h > 0`
- [ ] `Image` geometry matches `ScrollableText` (same x, y, width, height)
- [ ] `ScrollableText` visibility unchanged (`visible: root.book-title-img-h == 0`)
- [ ] `cross build` succeeds
- [ ] `cross test` passes (all tests, matching baseline of 336)
- [ ] Conventional commit message
- [ ] Branch from develop, not main
- [ ] Git clean after commit (no untracked/modified outside .agentic-tasks/)
