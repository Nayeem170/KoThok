# Changelog

All notable changes to KoThok are documented here.

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
- Mid-sentence page break: visual page turns while audio continues seamlessly
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
