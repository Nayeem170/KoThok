# Requirement: feat-chapter-word-single-header

## Feature

Redesign the chapter/word list overlay to use a single header instead of two stacked header bands. The overlay currently shows a 110px Slint header (back button) followed by a 48px Rust-painted band (tabs + close), totaling 158px before list content.

## Current layout

1. `chapter_overlay.slint`: 110px header with back button (red circle) + "Chapters" title
2. `word_list.rs` (paint_tab_bar): 48px band at y=110-158 with [Chapters] [Words] tabs + X close button
3. `search_results.rs`: Replaces tab bar with [< back arrow] + result count text

## Desired layout (Option A)

Single 110px header:
- Back button (left, red circle, brand-red #F42A41)
- Tab pills (center: "Chapters" / "Words")
- Close button (right, red circle, brand-red #F42A41)

On search results view:
- Back button (left, red) - navigates back to word list
- Context title (center) - e.g. "dawn" - 3 matches
- Close button (right, red) - exits overlay

## Classification

Feature (UI redesign)
