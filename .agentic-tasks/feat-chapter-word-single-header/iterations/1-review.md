# Code Review Revision 1

## BLOCKING

1. Results context title is empty (chapter_overlay.slint:114-119). The Text element shown in results mode has no text: binding. The plan specifies "show context title (word - N matches)" but there is no in-out property <string> results-title on the component, no binding from reader.slint, and no Rust code setting it. The title will render blank in results mode.

Fix: Add in-out property <string> results-title to chapter_overlay.slint, bind it to the Text element text: property, add a corresponding property/callback to reader.slint, and set it from Rust when entering results mode.

## SUGGESTION

2. Stale comment in word_list.rs:15-20. The TAB_BORDER/INK comment says "Tabs speak the library picker filter-pill language..." but tabs are now in Slint, not Rust. The constants remain for word list row selection styling. The comment should either describe that purpose or be removed.

3. DoD checkbox typo (definition-of-done.md:26): - ] tab_btn_rects()... is missing the opening [ bracket (- ] instead of - [ ]). Cosmetic but makes the checkbox rendering broken if parsed by tooling.
