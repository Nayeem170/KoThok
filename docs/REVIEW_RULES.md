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

### Type-change completeness (the rustc question)

- [seed] For any change that modifies a type (add an enum variant, add or remove a struct field, change a function signature, add a trait impl), ask: what does rustc do about this type change?
- [seed] rustc enumerates affected sites exhaustively at build time UNLESS an escape hatch silences it. Check for escape hatches: `_ =>` or `_ => {}` match arms, `..Default::default()` in struct literals (or `#[derive(Default)]` on the struct), catch-all or blanket impls, `#[non_exhaustive]`.
- [seed] If NO escape hatch exists: the compiler owns site discovery. At S4 (cross build) every un-updated site is a hard error (E0004 non-exhaustive match, E0063 missing field). The reviewer verifies the plan accounts for those sites and checks semantics per site; it does not have to discover them. Grep the source for the sites (struct literals, match expressions, trait impls) and confirm the plan's Files table covers them.
- [seed] If an escape hatch EXISTS: the compiler will NOT flag un-updated sites, so they silently fall through. This is silent-regression risk. The reviewer MUST manually enumerate every affected site and verify each. Flag the escape hatch itself (HIGH): a `_ => {}` that swallows a new variant, or a `..Default::default()` that hides a missing field, is the defect.
- [seed] This is NOT a ban on `_ =>` or `..Default`. It treats their presence as "enumeration silenced, review manually here" and their absence as "compiler enumerates, verify semantics." Generalizes to trait impls, From/Into conversions, serde field ordering, and match patterns.

### Verdict vs. load-bearing sub-claims

- [seed] A review verdict and the reasoning supporting it are separately falsifiable. A scoped axis can return the correct verdict on a false sub-claim: the reviewer checks the conclusion ("no W/H mutation in our scenarios" -- correct) but not the load-bearing reason ("about.rs fleet tests have the same hazard" -- false).
- [seed] Distinguish from an axis-never-given miss (the Type-change completeness / rustc-question case above): there the reviewer was never asked to check something; here the axis was given and the verdict was right, but a supporting sub-claim was wrong. Different failure mode, different remedy -- do not apply the scoping remedy to a verification-depth problem.
- [seed] Remedy: when a verdict rests on a factual sub-claim about the source, verify the sub-claim directly (read the cited line), not just that the verdict is internally consistent. A correct conclusion built on an unchecked premise passes review and ships the wrong rationale.
- [seed] Apply especially to attribution citations ("X is the precedent for Y"), hazard claims ("Z has the same risk"), and existence claims ("W exists at path P") -- load-bearing sub-claims a verdict can ride on without verifying.

### Test-vs-impl tautology

- [seed] A test must assert against a value the implementation does not derive. If the test recomputes its expected value using the same expression the implementation used, it validates the impl against itself and is anchored to nothing external. It cannot fail for the reason it exists, and a false green stops anyone from looking.
- [seed] Diagnostic: take the assertion's expected side and trace where each term comes from. If every term is either a literal in the test or a value returned by the function under test, the test is tautological. A real test names at least one constant from the system the code must agree with -- a layout coordinate, a file-format field count, a protocol size, a spec threshold.
- [seed] Concrete case: `tab_bar_geom` derived `seg_w` by subtracting `label_w + PAD_PX + 2*gap` from the panel width; `fleet_measure_text` then asserted `trailing_gap` computed from that same expression. Both passed on all 7 panels while the close button -- the thing the bar must actually clear, at `w - 99` -- appeared in neither. Moving or resizing the close button would not have failed the test.
- [seed] Remedy: assert against the external constant directly (`23 + 3*seg_w + 2*gap <= w - 99 - gap`). Where a measurement is involved, demote it from an input of the formula to the fit check on the result (`measure_text("Chapters", font_px) <= seg_w`), per `about.rs:604-618`.
- [seed] Applies beyond geometry: a serializer tested by round-tripping through its own parser, a hash checked against a value the same function produced, a cache test whose expected value is read from the cache.

### Reference-port completeness

- [seed] When a fix is ported from a reference site (a sibling file, an earlier commit, a "same as X" instruction), enumerate every site the reference touched before porting. A reference fix is usually multi-site; porting the facet that is easiest to locate leaves the rest, and the ticket reports the concern as closed.
- [seed] Diagnostic: diff the reference fix and list its changed hunks. Every hunk is a claim about what the concern required. If the port has fewer hunks than the reference, name which reference hunk each missing one corresponds to and why it does not apply here. "Does not apply" is a finding to state, not a gap to leave silent.
- [seed] Concrete case: `marks.rs` fixed book keying at two sites -- the save-side filter (bare `starts_with(book_path)` -> `starts_with("{book_path}|")`) and the load-side parse (`splitn(8)`). The port to `position.rs` applied only the load-side facet; `splitn(8)` satisfied the word "keying" in the requirement while the save-side prefix collision shipped unfixed through a green gate and an ACCEPTED review.
- [seed] A loosely-named concern in the requirement is the enabling condition. "Fix the keying" names a topic, not a site set. Where a requirement names a concern rather than sites, the plan must enumerate the sites before implementation, and the reviewer verifies the enumeration -- not just the fix.
- [seed] Test the defect's real shape, not the surface touched. The keying defect requires two books whose paths share a prefix; no existing test constructed that, so the gate was blind by construction. Prove a new test discriminates by running it against unfixed code and watching it fail before the fix lands.

### Requirement-clause coverage

- [seed] Every clause in an accepted requirement is both an implementation obligation and a review checkpoint. A clause no diff hunk satisfies has not been deferred -- it has been dropped, and the ticket closes as complete.
- [seed] Clause coverage is not diff review. Reading the full diff shows what the code does; it cannot show what the requirement asked for and the code never mentions. Absence has no hunk. That is why a green gate and an ACCEPTED review both pass over it.
- [seed] Diagnostic: before the verdict, list the requirement's clauses and name, for each, the `file:line` that satisfies it. A clause with no citation is a finding. "The plan did not cover it" is not an exemption -- the requirement was accepted, so the plan is the thing that failed.
- [seed] Concrete case: `feat-highlights-bookmarks` shipped three clauses unimplemented. C1 ("long press ... selects the word under the finger") shipped anchored at the TTS reading cursor. C5 ("selects that highlight and the bar offers Remove instead of Highlight") was absent entirely. D3 ("a long press arms the row ... and tapping it deletes ... a single misread gesture must not destroy a mark") shipped as a one-gesture delete -- live data loss. All three passed a green gate and an ACCEPTED review.
- [seed] Second-order cost: a dropped clause resurfaces as a fresh design question. The follow-up ticket re-derives options for a decision the accepted requirement already made, and can land a worse answer than the one already agreed. Before opening any follow-up, grep the original requirement for the behaviour -- the spec may already answer it, down to the draw call.
- [seed] Applies to prose, not just numbered lists. "User data is never discarded without the user asking" is an obligation with a test behind it, not a sentiment.

### Collapse the seam, do not test it

- [seed] When two sites must agree -- paint and hit-test over the same geometry, a setter and a clear over the same state, a stored value and the re-derivation of it -- make them derive from one source so the agreement is structural, not remembered. A fact computed twice that must match is a defect waiting for the next edit to one side.
- [seed] State as well as geometry. The rule generalizes from row bounds and label rects to mutable state: an invariant that field B clears whenever field A clears must be enforced by making B live inside A, not by remembering to clear both at every site. A parallel field that must track another's lifecycle is the same seam as a duplicated geometry computation.
- [seed] Concrete case (geometry): the marks-list delete target. Paint and hit-test both needed the row's bounds and the delete-label rect; computing them independently meant a paint/hit-test drift would pass the gate. The fix shared one `row_visible` derivation between paint and hit-test. Where an independent copy is kept deliberately (so a comparison test stays non-vacuous), say so in a comment -- the exception is intentional, not a missed collapse.
- [seed] Concrete case (state): the selection lock. A parallel `locked` field that must clear whenever `selection` clears is a seam -- every clear site must remember both. Collapsing it puts `locked` inside `Selection`, so `st.selection = None` / `.take()` drops it atomically and no site can forget. The rename `selection-active: bool` -> `selection-mode: int` is the same move: deleting the old setter turned every stale site into a compile error instead of a silent drift.
- [seed] Diagnostic: list the sites that must agree. If more than one derives the same fact, ask whether they can read one source. If they genuinely cannot (real independence), name why in a comment and assert against an external constant -- a layout coordinate, a spec threshold -- not against the sibling computation. "They agree today" is not a guarantee.
- [seed] Remedy order: prefer (a) one source both sites read, then (b) a type change that makes drift a compile error (field inside the owning struct, rename-not-add so stale sites error, re-derive instead of store), and only then (c) a test. A test is the last resort for a seam you cannot collapse, never a substitute for collapsing one you can.

## Learned rules

- [feat/word-list-select-open-flow|0] UI mocks use Pencil CLI (pen.dev) by default. Read `pen_cli` from `.agentic/config.md` for the binary name (default: `pen`). Write mock.md with device dimensions, prompt, design decisions, interactive states. Generate with `<pen_cli> --out mock.pen --prompt-file mock.md --enable-preview`. Export preview with `<pen_cli> --in mock.pen --export mock-preview.png`. If Pencil unavailable, fall back to mock.html with device dimensions.
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
