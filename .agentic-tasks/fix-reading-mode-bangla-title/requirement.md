# Requirement: fix-reading-mode-bangla-title

## Bug

Bangla (Bengali script) book titles are not displayed in the reading mode header. The header title area appears blank when a Bangla EPUB is open in reading mode.

## Expected behavior

The pre-rendered Bangla title raster should appear in the reading mode header, matching the behavior of the audio mode header and the control panel header.

## Actual behavior

The reading mode header is blank for Bangla books. The title displays correctly in audio mode and the control panel.

## Context

- `meta.rs::set_book_meta()` detects Bangla titles via `has_bangla()` and pre-renders them as raster images using `text_image()`. It sets `book-title-img` and `book-title-img-h > 0` globally on the `Reader` component.
- `reader.slint:135-137` wires `book-title-img` and `book-title-img-h` to `content.slint`.
- `content.slint:82-88` has only a `ScrollableText` element with `visible: root.book-title-img-h == 0`. For Bangla books, `book-title-img-h > 0` hides the text element, and no `Image` element exists to display the pre-rendered raster.
- `audio_player.slint:91-108` and `control_panel.slint:297-309` both have the correct dual-element pattern: `Image` (visible when `img-h > 0`) + `ScrollableText` (visible when `img-h == 0`).

## Classification

Bug fix (component state bug - missing UI element).
