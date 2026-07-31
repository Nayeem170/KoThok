# Design Decisions: feat-chapter-word-single-header

## Decision: Single header with back + tabs + close (Option A)

### Layout

Single 110px header band replacing the current two-band layout (110px Slint + 48px Rust-painted):

```
[Back (red)]  [Chapters pill | Words pill]  [Close (red)]
```

### Rationale for Option A over B/C

- A chosen by user
- Tabs are the primary navigation (switch Chapters/Words)
- Close always exits the overlay (consistent exit path)
- Back is context-aware: exits on list views, navigates back on results
- Red close button matches app design (brand-red #F42A41, same as back button)

### Close button color

Brand-red (#F42A41, same as `root.brand-red` in Slint components). Matches the back button, library button, and other primary action buttons throughout the app.

### Reference patterns

- `chapter_overlay.slint:33-45`: Back button (red circle, 76x76px at x:23 y:17)
- `content.slint:69-80`: Library button (red circle, 76x76px at x:23 y:17)
- `audio_player.slint:84-87`: Library button (same pattern)
- `word_list.rs`: Tab pills (active=ink/white, inactive=white/ink with border)

### Results view

Tabs replaced by center-aligned context title (e.g. "dawn" - 3 matches). Back button navigates to word list. Close exits overlay.

### Out of scope

- No changes to list row rendering or scrollbar behavior
- No changes to bottom-strip Open button
- No changes to chapter_overlay.slint layout below the header
