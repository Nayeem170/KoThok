# Bug Reproduction: fix-reading-mode-bangla-title

## Bug type
Component state bug (missing UI element)

## Reproduction approach
Manual reproduction (Slint markup bug - element missing from component tree)

## Preconditions
- A Bangla EPUB book is open and displaying in reading mode
- `meta.rs::set_book_meta()` has detected Bangla script in the title via `has_bangla()`
- `book-title-img` is set to a pre-rendered raster image
- `book-title-img-h` is set to a value > 0

## Steps
1. Open a Bangla EPUB (title contains Bengali script characters)
2. Navigate to reading mode (content.slint renders the page)
3. Look at the header bar between the library icon (left) and the right-side icon group

## Expected result
The book title is visible in the header, rendered as a raster image (same as audio mode).

## Actual result
The header title area is empty. No book name is displayed.

## Root cause trace
1. `meta.rs:128-131` - `has_bangla(title)` returns true, so `text_image()` pre-renders the title to a raster and sets `book-title-img` + `book-title-img-h > 0`
2. `content.slint:87` - `ScrollableText` has `visible: root.book-title-img-h == 0`. For Bangla books, `book-title-img-h > 0`, so this evaluates to false -- the text element is hidden
3. `content.slint:82-88` - No `Image` element exists in the header to display the pre-rendered raster. Nothing renders.
4. Compare with `audio_player.slint:91-101` - Has both `Image` (visible when `img-h > 0`) and `ScrollableText` (visible when `img-h == 0`). Bangla titles display correctly there.

## Acceptance criteria
- Given a Bangla book title, When reading mode header renders, Then the pre-rendered title raster is visible (not blank)
- The title matches the audio mode header display
