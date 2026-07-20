# Changelog

All notable changes to KoThok are documented here.

## [0.2.0] - 2026-07-21

### Reading
- Reading-mode auto-sleep is now user-configurable: Off (default) / 5 min / 15 min. No more mid-read sleep interrupts.
- Bookmark anchors to the first line of the current page when the cursor is stale or audio is off. Works in any mode, online or offline.
- Stale footer status ("Bookmarked page N") now clears on TTS auto page-turn, so the live page count shows through.
- Reading marker spans the whole page on wake and page-turn (no more half-highlight).

### Read-aloud (TTS)
- Audio resume preserves cursor position across wake/sleep (no restart from page top).
- Cursor color and sentence-band rendering fixed after font/layout changes.
- Bangla TTS voice selection fixed.
- Settings panel now closes correctly on back tap in audio mode.
- Double-flicker on wake eliminated (frontlight off + redundant present removed).

### Library
- EPUB scanner skips hidden directories (.adds, .kobo, etc.). Test books and extracted content no longer pollute the library.

### Panel / UI
- WiFi and Bluetooth selectors are now tri-state: off (black) / connecting (red) / connected (green), with live status labels.
- Unified headers with round icon buttons across all screens.
- Portrait splash screen redesign.
- Library page header.
- About screen updated: contact info, GitHub, LinkedIn.
- Sleep timeout selector added to the Settings panel under Display.

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
