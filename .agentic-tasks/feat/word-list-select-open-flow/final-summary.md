# feat/word-list-select-open-flow — 2026-07-31

## User-facing changes

- **Words tab: select-then-open.** Tap a word to highlight it (inverted pill). Tap Open to show search results. Back preserves selection. Matches the Chapters tab two-step flow.
- **Search results: select-then-open.** Tap a result row to highlight it. Tap Open to jump to that page in the book. Open with nothing selected does nothing.
- **Draggable scrollbar** on all three overlay lists (Chapters, Words, Search Results). Drag the red thumb for proportional scroll. Tap the green rail for page-step (grip protection).
- **Inverted chapter highlight.** Current chapter pill uses INK fill + white text, matching the Words/Search Results selection style.
- **Taller rows.** 68px rows (was 60px) across all lists for easier touch targets.
- **Wider scrollbar thumb.** 32px red thumb on 12px green track, centered in a 100px gutter. Fat thumb reads as draggable on e-ink.

## Device-test bugs found and fixed

1. **Open button "scrolls down" instead of working.** Root cause: overlay release handler was inside `press_dispatched` block in `touch_release.rs`, but `touch_dispatch.rs` set `press_dispatched=false` for the Open button area. Fix: hoist overlay release out of `press_dispatched` branch into `chapter_overlay_release()`.
2. **Scrollbar invisible.** Five sub-causes: paint drew 1px wide (only drew track, never thumb), hit zone too narrow (no touch pad), three different `scroll_max` formulas producing inconsistent values, raw touch coords used instead of display coords (26px Y offset), thumb height too short to see. Fix: unified `thumb_metrics`/`thumb_top` helpers, `SB_THUMB_MIN_H=80`, display coord conversion, proper hit zone constants.
3. **Scrollbar not scrollable (thumb teleport on drag).** Root cause: used `thumb_h/2` as offset term so every move centered the thumb under the finger. Fix: store `sb_grab_offset` at press as `finger_y - thumb_top(...)`, pass to conversion as offset term.
4. **Rail tap causes violent scroll jump.** Root cause: rail tap treated same as thumb press, computed scroll from finger position directly. Fix: Thumb vs Rail split -- touch within +/-30px of thumb rect = drag mode, outside = page-step one screen toward tap.
5. **Occurrence page tap jumps directly (no select step).** Root cause: `search.rs` called `jump_to_occurrence()` on row tap. Fix: row tap sets `search_result_selected`/`search_selected_result` fields and repaints. Open button handler calls `jump_to_occurrence`.
6. **Scrollbar colors wrong (magenta thumb, teal track).** Root cause: hand-encoded RGB565 hex values had bit errors -- `0xF158` decoded to #F828C0 (magenta), `0x034F` decoded to #006878 (teal). Fix: added `rgb565(r, g, b)` const fn that takes readable RGB888 and packs correctly. Track now brand green, thumb brand red.

## Tests

- Before: 323 (develop baseline)
- After: 336
- Delta: +13 new

New tests cover: scrollbar hit test (5), scroll position math (4), scrollbar visibility (2), scroll round-trip (2), word-tap-select (1), back-preserves-selection (1).

## Patterns established

- **`rgb565()` const fn** -- all palette constants written as `rgb565(R, G, B)` in readable hex. Prevents silent hue shifts from bit packing typos.
- **Sentinel pattern for optional selection** -- `usize::MAX` passed to paint functions, no extra bool param needed. Used by `paint_word_list` and `paint_search_results`.
- **`thumb_metrics`/`thumb_top` shared helpers** -- both paint and hit-test use identical geometry math. Eliminates class of bugs where thumb drawn at one position but hit-tested at another.
- **Compile-time layout invariant** -- `const _: () = assert!(...)` ensures scrollbar band width exactly fills the gutter left by rows. Catches drift when constants change.
- **Thumb/Rail interaction split** -- touch within proximity of thumb = drag (with grab offset), outside = page-step. Prevents accidental teleport on rail tap while keeping one-finger operation.
- **`chapter_overlay_release()` extracted function** -- overlay touch handling isolated from the main release handler. Eliminates nesting bugs with `press_dispatched`.

## Known limitations

- Kaleido CFA mutes red. Even correct brand red (#F42A41) renders washed out on the colour filter array. If saturation is needed, push toward #FF0033 rather than re-editing the constant.
- Exit to nickel requires reboot (SIGSTOP/SIGCONT corrupts Qt QWS GUI state).
- BT A2DP fatigue after multiple connect/disconnect cycles -- re-pair from Bluetooth settings to fix.
- No offline TTS viable on ~1 GHz ARM -- Edge TTS over WiFi is the only option.

## Pipeline cost

Rough per-phase token estimates (developer + reviewer combined):
- S1 Requirement: ~15k
- S2 Design decisions: ~20k
- S2.5 UI mock: ~25k (3 iterations)
- S3 Plan: ~35k (2 iterations)
- S3.5 Test plan: ~15k (2 iterations)
- S4 Implementation: ~50k (build + test cycles)
- S5 Code review: ~30k (2 iterations)
- S6 DoD verification: ~10k
- S7 Device testing: ~80k (6+ fix-deploy-test cycles)
- Total: ~280k tokens

Branch: `feat/word-list-select-open-flow` (2 commits + uncommitted device fixes)
Files: 14 source files changed (~490 lines committed, ~200 uncommitted)
