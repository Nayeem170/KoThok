# Changelog

All notable changes to KoThok are documented here.

## [Unreleased]

### Reading
- Nested table of contents: chapter overlay now shows the book's nav tree (NCX/nav.xhtml) with depth-based indentation instead of a flat chapter list. Entries with anchors jump to the exact in-chapter position. Books without a nav tree fall back to one row per spine chapter.
- In-text link navigation: tapping a hyperlink in the body text follows it to its target chapter and anchor (footnote-style links keep audio context).
- Double-tap 2x magnifier: a double-tap anywhere in the content area toggles a magnifying-glass crop centred on the tap point. Swipe or tap again to exit.
- Ordered and unordered list markers (1., -, etc.) now render inline in the row text, not as separate layout elements.
- Heading hierarchy (h1-h6) rendered with per-level scale derived from body_px.
- Link underlines: hyperlinks now show a baseline underline for visual distinction.
- Blockquote left border: rendered in pure ink black instead of near-invisible grey.
- Snapshot-based drag scroll replaces the old snap-on-release scroll for the chapter list and other scroll regions (smoother, no jump).

### Read-aloud (TTS)
- Symbol-density classifier: dense `<pre>` blocks (JSON, XML, code > 15% symbols) get a short "Code block." placeholder for TTS instead of reading raw punctuation. Transcript-style blocks below the threshold are still read in full.
- Script/style leak fixed: orphan text collector no longer dumps raw `<script>`/`<style>`/`<link>` content at chapter end.
- PCM LRU cache (8 MB, keyed by text + voice + rate): already-heard pages replay from cache with no Edge TTS round-trip.
- Seek does not tear down the A2DP sink: the player stays open across skips, eliminating the silence gap from close/reopen.
- Voice switch resumes from the current utterance instead of restarting the page.
- Radio reconnect on wake: WiFi/BT are brought back up correctly after extended sleep.
- No-blink transitions: redundant frontlight-off + present calls removed from the wake sequence.

### Library
- Book content cache now stores the TOC tree alongside chapters (`CACHE_FORMAT` bumped 2 -> 3).

### Panel / UI
- About screen checks for app updates.
- Heading scale uses the live `body_px`, not the compile-time `BODY_PX` constant, so font-size changes re-scale headings correctly.
- CJK grid preloaded for fonts that need it.

### Infrastructure
- Manual-install packaging (`KoThok-<version>-manual-install.zip`): drag-and-drop install for users without PowerShell 7.
- `BUILD_TAG` is now the Cargo version (`v0.2.0`) via `env!("CARGO_PKG_VERSION")` instead of a hardcoded string.
- Font load status frames during splash.
- Sample book installer added to deploy scripts.
- Panic-safe indexing throughout rendering: out-of-bounds row/page access logs instead of crashing.
- Crash report written to `.adds/crash.log` via panic hook.
- E-ink tuning: chapter overlay cards switched from grey fill to white (border-only state), eliminating the grey-area ghosting that forced GC16 on overlay transitions. Overlay open/close now uses GL16 like the reading panel.
- Performance: image name lookup uses a HashMap (was O(n^2) linear scan); Vec allocation hoisted out of pre-block wrap loop.
- 7 review findings closed: MONO_SCALE constant, BT fail count reset, picker repaint guard, figure leading gap, caption hit-test extraction, block indent clamp, orphan text gap-only collector.
- Heading-level tag aliases resolved: `row_flags()` returns 0 for non-body rows, pinned with tests.

## [0.2.0] - 2026-07-21

### Reading
- Reading-mode auto-sleep is now user-configurable: Off (default) / 5 min / 15 min. No more mid-read sleep interrupts.
- Bookmark anchors to the first line of the current page when the cursor is stale or audio is off. Works in any mode, online or offline.
- Stale footer status ("Bookmarked page N") now clears on TTS auto page-turn, so the live page count shows through.
- Reading marker spans the whole page on wake and page-turn (no more half-highlight).
- One sleep setting applies to both reading and audio mode (was 60s fixed for audio).

### Read-aloud (TTS)
- Audio resume preserves cursor position across wake/sleep (no restart from page top).
- Cursor color and sentence-band rendering fixed after font/layout changes.
- Bangla TTS voice selection fixed.
- Settings panel now closes correctly on back tap in audio mode.
- Double-flicker on wake eliminated (frontlight off + redundant present removed).

### Audio mode
- Page count and chapter number below the disk now update on auto-advance (was stuck at Page 1).
- Left/right swipe added for page navigation (was bypassed in audio mode).
- Bookmark jump seeks within the full chapter audio instead of replacing it with one page.
- Progress bar and swipe navigation seek within the chapter instead of reloading page audio.

### Fonts
- On-demand font download over WiFi: missing fonts auto-download the first time a book in that script is opened. No prompt, no language picker.
- NotoSans.ttf (Latin/Greek/Cyrillic) can also auto-download and installs for all three scripts.
- Lets the KoboRoot.tgz ship without 17 MB of CJK fonts - they download on first use.

### Library
- EPUB scanner skips hidden directories (.adds, .kobo, etc.). Test books and extracted content no longer pollute the library.

### Panel / UI
- WiFi and Bluetooth selectors are now tri-state: off (black) / connecting (red) / connected (green), with live status labels.
- Unified headers with round icon buttons across all screens.
- Portrait splash screen redesign.
- Library page header.
- About screen updated: contact info, GitHub, LinkedIn.
- Sleep timeout selector added to the Settings panel under Display.
- Version shown on About page from a single source (Cargo.toml via env!).

### Infrastructure
- Cross-platform uninstaller (uninstall.bat / .command / .sh via USB file method).
- USB deploy script (deploy.ps1) for rapid binary updates with MD5 verification.
- kothok-edge-tts bumped to 0.2.9 (published on crates.io).
- gesture.rs split into a gesture module (532 -> 248 + 236 lines).
- Script-test EPUB generator with --deploy flag (targets .adds/kothok/, not the book folder).
- Audio regression test: verifies no sentence is dropped across page boundaries.

### Known limitations
- Exit to nickel requires reboot
- A2DP Bluetooth fatigues after many connect/disconnect cycles
- Color e-ink: partial updates may leave ghosting, full updates flash
- PDF not supported (EPUB only)

## [0.1.0] - 2026-07-11

First public release.

### Reading
- EPUB support with cover, chapter, and image rendering
- Kaleido colour e-ink support (Clara Colour, Libra Colour)
- Page-turn by edge swipe
- Whole-book seek bar with saved-position marker
- Live font-size change that preserves reading position
- Chapter list with single-tap preview and double-tap open
- Sleep/wake preserves page and brightness
- Arabic, Bengali, Devanagari, Thai, CJK script support

### Read-aloud (TTS)
- Edge-TTS synthesis streamed to Bluetooth A2DP speaker
- Inter-sentence and paragraph gaps baked into audio
- Per-book-language voice selection (auto-detect script)
- Mid-sentence page break: visual page turns while audio continues without interruption
- Voice choice remembered per language
- Draggable sliders for brightness, speed, font size, and volume

### Library
- Animated splash with spinner during font loading and book scanning
- Cover grid with book covers cached by path
- Most-recently-read book shown first

### Connectivity
- WiFi and Bluetooth toggles with grace periods
- Friendly on-screen messages when network or speaker unavailable
- Parsed-book disk cache: first open parses, re-opens are instant
- Dynamic voice list fetched from Edge when WiFi available

### Infrastructure
- 3-repo architecture: kothok-app, kobo-core, kothok-edge-tts
- Both libraries published to crates.io
- Cross-platform installer: install.bat (Windows), install.command (macOS), install.sh (Linux)
- Downloads pre-built binary from GitHub releases (no Rust/Docker needed for users)
- NickelMenu integration via KoboRoot.tgz

### Known limitations
- Exit to nickel requires reboot
- A2DP Bluetooth fatigues after many connect/disconnect cycles
- Color e-ink: partial updates may leave ghosting, full updates flash
- PDF not supported (EPUB only)
