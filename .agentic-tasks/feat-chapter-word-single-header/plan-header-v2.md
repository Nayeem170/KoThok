# Plan v2: single action button + round tab toggle + centre title

Revision of the shipped single-header (78d45ef / b24a07a) after on-device review.

## What changes

Header stays 110px. Contents become:

```
(empty)                 Chapters                 [toggle] | [close|back]
                     centred 33px bold           w-216    w-121   w-99
```

- One red 76px action button at the far right. Icon and action depend on mode:
  - chapters / words list -> `close.svg`, closes the overlay
  - occurrence results   -> `back.svg`, returns to the word list
- To its left, a 3px x 50px pipe separator, then a green 76px round toggle that
  swaps Chapters <-> Words. Same pipe idiom as the reading header
  (`content.slint:140-144`), and the same right-edge stack order.
- Centre title returns: "Chapters" / "Words" / the results title.
- 3px bottom rule at y=107 restored (was in the pre-78d45ef header).
- Pill tabs and the old left-hand back button are deleted; the left side of the
  header is now empty.

## Analysis

### Why the pills go

They are laid out inside a `HorizontalLayout` that Slint sizes and positions
itself, so their painted geometry and their touch geometry are not pinned by
this file the way every other header control is. Every control that is known to
work on device (library, gear, chapters, back) is an absolutely positioned
`Rectangle`/`ActionButton` with explicit `x`/`y`/`width`/`height` inside the
110px header rectangle. The new toggle uses that proven construction, so it does
not inherit whatever the layout was doing.

I could not reproduce the dead-tap on host, so this is the likely cause, not a
confirmed one. It stops mattering once the layout is gone.

### Real bug found while reading this: `active-tab` binding is one-way

`reader.slint:271` binds `active-tab: root.chapter-overlay-active-tab` — one
way, parent to child. `chapter_overlay.slint:73` then assigns
`root.active-tab = 0` from the pill handler. In Slint, assigning to a property
drops its incoming binding permanently. After the first tab tap:

- `loop_run/callbacks.rs:254`'s `reader.set_chapter_overlay_active_tab(0)` on
  overlay open no longer reaches the overlay, so reopening the overlay shows the
  stale tab; and
- the Open button fires `chapter-selected(idx, active-tab)` with the stale tab.
  `callbacks.rs:80` treats `tab == 1` as "open the word list", so a chapter tap
  can silently do nothing instead of jumping.

Fix: make the binding two-way (`<=>`), same idiom already used for
`visible-flag`, `chapter-preview-idx`, `chapter-pending`.

### Accepted trade-off

In results mode the single button is Back, so there is no one-tap exit from
results — you go back to the word list, then close. That is what was asked for.

## Geometry (w = 1072)

Laid out right-to-left from the 23px edge padding, exactly like the reading
header's system-button cluster.

| element | x | y | w | h | notes |
| --- | --- | --- | --- | --- | --- |
| title | 240 | 0 | width-480 | 110 | 33px, weight 700, `text-secondary`, elide |
| toggle | width-216 | 17 | 76 | 76 | radius 38, `brand-green`, hidden in results mode |
| pipe | width-121 | 30 | 3 | 50 | `track-color`, hidden in results mode |
| action button | width-99 | 17 | 76 | 76 | radius 38, `brand-red` |
| bottom rule | 0 | 107 | width | 3 | `track-color` |

19px gap either side of the pipe. The title box is symmetric about the screen
centre (240px margin each side), so the heading stays optically centred; its
right edge clears the toggle by 24px, and the empty left margin is the price of
putting both controls on the right.

## Steps

1. **`kothok/ui/components/assets/words.svg`** (new). 24x24 white capital "A"
   drawn as strokes, same flat style as `chapters.svg` / `close.svg`. A letter
   reads as "words" at the 28px `ActionButton` icon size and stays distinct from
   `chapters.svg` (three rules) and `library.svg` (four spines); a magnifier was
   tried first and rejected as it implies typing a query, which this tab has no
   way to do.

2. **`kothok/ui/components/chapter_overlay.slint`**
   - Delete the pill `HorizontalLayout` (lines 58-112) and the left-hand back
     `ActionButton` (lines 40-56).
   - Keep the right-hand `ActionButton` at `x: root.width - 99px` and make it
     the single control: `icon: root.results-active ? back.svg : close.svg`,
     click branches back-from-results vs close-to-book (the branch currently
     living on the deleted left button), and still clears
     `chapter-preview-idx` / `chapter-pending`.
   - Add pipe `Rectangle` and toggle `ActionButton` to its left, both under
     `if !root.results-active`.
   - Toggle icon shows the *target* tab: `active-tab == 0 ? words.svg :
     chapters.svg`. Handler:
     `root.chapter-preview-idx = -1; root.active-tab = root.active-tab == 0 ? 1 : 0;
     root.tab-switch(root.active-tab);`
   - Replace the two centre elements with one `Text`:
     `results-active ? results-title : (active-tab == 0 ? "Chapters" : "Words")`.
   - Restore the 3px bottom rule.

3. **`kothok/ui/reader.slint`** — `active-tab <=> root.chapter-overlay-active-tab;`

4. **`kothok/src/rendering/word_list.rs`** — fix the stale `TAB_BORDER`/`INK`
   doc comment left over from iteration 1 review item 2 (it still talks about
   tabs; the constants are now only row-selection styling).

No other Rust changes. `tab-switch` -> `overlay_tab_switch_cell` ->
`loop_run/callbacks.rs:278` already resets scroll and selection per tab, and
`CH_LIST_TOP` stays 110 so no list geometry moves.

## Verification

Host: `cross build --target armv7-unknown-linux-musleabihf --release -p kothok-app`
-- **done, clean**. Nothing under `kothok/src` changed (only `.slint` and the new
`.svg`), so the 334-test baseline and clippy/fmt status are untouched.

Device (see the on-device test workflow):
1. Open the overlay -> title reads "Chapters" centred, rightmost button is a red
   X, toggle sits to its left across the pipe and shows the words glyph.
2. Tap toggle -> title "Words", list swaps, toggle shows the chapters glyph.
   Tap again -> back to Chapters. Repeat 5x; every tap must register.
3. Tap X on both tabs -> overlay closes to the book.
4. Words -> select a word -> Open -> results. Title is "word - N matches",
   pipe and toggle are gone, the rightmost button is a red back arrow.
5. Back -> word list with the word still selected; toggle and pipe return.
6. Close the overlay, reopen it -> title is "Chapters" again (this is the
   one-way-binding regression; it must not stick on "Words").
7. Chapters tab -> tap a row -> Open -> jumps to that chapter (guards the
   stale-`tab` path in `callbacks.rs:80`).
