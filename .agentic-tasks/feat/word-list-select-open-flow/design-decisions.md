# Design Decisions: feat/word-list-select-open-flow

## DD-1: Open button handling

| Option | Trade-off |
|--------|-----------|
| Slint button for both tabs | Consistent with existing chapters approach; no rendering duplication |
| Rust-drawn button | Requires removing Slint button, breaking chapters |

**Decision**: Use the existing Slint Open button for both tabs. The callback checks the active tab and routes accordingly.

## DD-2: Tap behavior on word row

| Option | Trade-off |
|--------|-----------|
| Tap selects only (no search results open) | Matches chapters pattern exactly |
| Tap selects + shows preview | Adds complexity, chapters don't do this |

**Decision**: Tap selects the word (highlights it), does NOT open search results. Open button activates search results.

## DD-3: Search results back behavior

| Option | Trade-off |
|--------|-----------|
| Back returns to word list, keeps selection | User can re-open same word's results |
| Back returns to word list, clears selection | Fresh state each time |

**Decision**: Back returns to word list with selected word still highlighted (matches chapters: going back to list keeps the chapter selected).

## DD-4: Draggable scrollbar for word list and chapters

| Option | Trade-off |
|--------|-----------|
| Draggable scrollbar on right edge | Fast-scroll long lists; matches platform expectations (file browsers, settings) |
| Visual-only scrollbar (current) | Already implemented; no touch complexity |

**Decision**: Add a draggable scrollbar on the right edge for both word list and chapter list. Touch-down on the scrollbar thumb starts a drag session. Touch-move updates scroll position proportionally. Touch-up ends drag. The list re-renders at the new scroll position on every move event. This replaces the visual-only scrollbar.

## DD-5: Scrollbar interaction zone

| Option | Trade-off |
|--------|-----------|
| Full-height scrollbar rail (tap anywhere to jump) | Easier to hit; standard on mobile |
| Thumb-only drag | Precise but harder to target on e-ink |

**Decision**: Tap anywhere on the scrollbar rail to jump to that position. Drag the thumb for proportional scrolling. This gives both fast-jump and fine-control.
