# Definition of Done: fix-reading-mode-bangla-title

## Build
- [ ] `cross build` succeeds with no warnings from new code
- [ ] `cross test` passes (ALL tests, not just new ones)

## Convention compliance
- [ ] ASCII-only in all source files (no em dash, smart quotes, unicode)
- [ ] LF line endings (no CRLF)
- [ ] No comments unless explaining non-obvious WHY
- [ ] No fallback implementations
- [ ] Conventional commit messages
- [ ] Branch named fix/reading-mode-bangla-title

## Requirement coverage
- [ ] `content.slint` has `Image` element with `source: root.book-title-img` - content.slint header area
- [ ] `Image` has `visible: root.book-title-img-h > 0` - content.slint header area
- [ ] `Image` geometry matches ScrollableText (same x, y, width, height) - content.slint header area

## Scope
- [ ] No changes outside plan.md scope (only content.slint modified)
- [ ] No unrelated refactoring
