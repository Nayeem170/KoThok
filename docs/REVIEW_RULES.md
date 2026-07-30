# Review Rules

Auto-growing rule set for the agentic pipeline reviewer. The reviewer reads this
file at the start of every review iteration. New rules are added when the same
feedback pattern recurs.

Format: `[source] rule text`

## Seeded from AGENTS.md and CODE_CONVENTIONS.md

### Typography and encoding
- [seed] ASCII-only in all source files: no em dash, en dash, smart quotes, ellipsis char, unicode arrows, decorative symbols, emoji
- [seed] Use `-` instead of em/en dash, `->` instead of arrow, `...` (three dots) instead of ellipsis character
- [seed] LF line endings only (CRLF breaks the cross build)
- [seed] No non-breaking spaces or zero-width spaces

### Code style
- [seed] No comments unless explaining non-obvious WHY (hardware quirk, deliberate trade-off). Do not narrate what code does.
- [seed] No dead code, no unused imports, no placeholder comments
- [seed] No AI phrasing in docs/UI: no "seamless", "effortless", "premium", "cutting-edge"

### Engineering principles
- [seed] Never implement fallbacks. Always fix the actual root cause.
- [seed] One responsibility per module/function. God objects are defects.
- [seed] Fail loud at boundaries (expect with message), fail soft on device paths (never panic in event/render/audio/input)
- [seed] Determinism over cleverness. No implicit state, hidden mutability, time-based fallbacks.
- [seed] Separation: pure logic in kobo-core with unit tests, device adapter in kothok-app.

### Render pipeline
- [seed] Clear -> composite -> present, exactly once per frame. Never present twice for one frame.
- [seed] Never decode inside present.
- [seed] Render is a pipeline, not a scatter.

### Audio/layout sync
- [seed] Any code path that calls build_state() MUST also reload audio: page_utterances() -> Cmd::Reload -> Cmd::Seek
- [seed] apply_font_reflow() and check_font_repaginate() both follow this pattern.

### Git
- [seed] Conventional commits: feat:/fix:/chore:/docs:/refactor:
- [seed] Branch naming: type/ticket-name from develop
- [seed] No secrets or API keys in code
- [seed] Merge with --no-ff to preserve branch history
- [seed] Never commit directly on main or develop

### Testing
- [seed] All tests must pass before commit: cross test -p kothok-app
- [seed] Tests for new functionality, not just regression tests
- [seed] Test fixture data (Bangla, Arabic, CJK, Thai, Devanagari) is exempt from ASCII-only rule

### Device-specific
- [seed] Frontlight controller on Kobo Libra Colour: lm3630a_led (max_brightness=100)
- [seed] leda/ledb are color channels, NOT main frontlight
- [seed] After extended sleep, toggle bl_power to force driver reinit

## Learned rules

- [feat/word-list-select-open-flow|0] UI mocks must be HTML/CSS files (mock.html), not ASCII art. Render the screens as styled HTML matching the device layout, colors, and dimensions. Include interactive states (hover, selected, disabled).
<!-- Format: [task-name|0] rule description (counter starts at 0, reset when rule triggers feedback, incremented at end of each task) -->
<!-- Only add a rule when the same feedback has been given more than once. -->

## Rule lifecycle

- Max 50 active rules. When full, retire the lowest-count rule (move to "Archived" section).
- Counter is inline: `[source|N]`. When a rule triggers feedback during a review, reset its counter to 0. At the end of each task (Step 8), increment all active rule counters by 1.
- Rules with counter >= 5 are retirement candidates.
- The reviewer reads only active rules each iteration.
- Archived rules are kept for reference but not checked automatically.

## Archived
<!-- Retired rules moved here. Format: [source|N at retirement] rule description -->
