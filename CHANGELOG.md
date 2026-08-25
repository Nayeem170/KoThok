# Changelog

All notable changes to KoThok are documented here.

## [0.3.0] - 2026-08-25

### Reading
- Multiple bookmarks per book. The header bookmark icon toggles a bookmark at the current spot, and a book can hold any number of them.
- Every bookmark draws its own green tab on the progress bar, in reading and audio mode.
- The header jump arrow returns to the most recently set bookmark.
- The chapter overlay gains a Bookmarks tab beside Chapters and Words. Rows list page and chapter title in book order; tap a row then tap Open to jump; press and hold a row for half a second to delete that bookmark.
- Bookmarks persist per book. A position saved by an older single-bookmark build loads as a one-bookmark list.

### Read-aloud (TTS)
- Sleep timer in Settings: Off / 15 / 30 / 45 / 60 minutes, one countdown for both reading and audio mode. At expiry the device sleeps; a locked device sleeps too, and the view mode is kept across sleep and wake.
- The countdown keeps its deadline across auto chapter advance and A2DP sink reopen instead of restarting. A fresh play and a panel mode change restart it; resuming from pause keeps the frozen remaining time.
- Audio caption reworked: page info and sleep time stack on two lines, drawn as images (Slint Text failed in the audio body), darkened for e-ink, and spaced so the sleep line clears the footer. The chapter name moves up to make the gap instead of pushing page info down.
- Sleep timer arms on mode change while playing.

### Panel / UI
- Device panel shows total storage beside free space ("12.1 GB / 28.9 GB").
- Update deploys now sync the onboarding guide, so the version-change guide reopening shows current chapters.

## [0.2.0] - 2026-07-26

### Reading
- Double-tap 2x magnifier: a double-tap anywhere in the content area toggles a magnifying-glass crop centred on the tap point. Swipe or tap again to exit.
- In-text link navigation: tapping a hyperlink in the body text follows it to its target chapter and anchor (footnote-style links keep audio context).
- Nested table of contents: chapter overlay now shows the book's nav tree (NCX/nav.xhtml) with depth-based indentation instead of a flat chapter list. Entries with anchors jump to the exact in-chapter position. Books without a nav tree fall back to one row per spine chapter.
- Ordered and unordered list markers (1., -, etc.) now render inline in the row text, not as separate layout elements.
- Heading hierarchy (h1-h6) rendered with per-level scale derived from body_px.
- Link underlines: hyperlinks now show a baseline underline for visual distinction.
- Blockquote left border: rendered in pure ink black instead of near-invisible grey.
- Snapshot-based drag scroll replaces the old snap-on-release scroll for the chapter list and other scroll regions (smoother, no jump).
- Reading-mode auto-sleep is now user-configurable: Off (default) / 5 min / 15 min. No more mid-read sleep interrupts.
- Bookmark anchors to the first line of the current page when the cursor is stale or audio is off. Works in any mode, online or offline.
- Stale footer status ("Bookmarked page N") now clears on TTS auto page-turn, so the live page count shows through.
- Reading marker spans the whole page on wake and page-turn (no more half-highlight).
- One sleep setting applies to both reading and audio mode (was 60s fixed for audio).

### Read-aloud (TTS)
- Symbol-density classifier: dense `<pre>` blocks (JSON, XML, code > 15% symbols) get a short "Code block." placeholder for TTS instead of reading raw punctuation. Transcript-style blocks below the threshold are still read in full.
- Script/style leak fixed: orphan text collector no longer dumps raw `<script>`/`<style>`/`<link>` content at chapter end.
- PCM LRU cache (8 MB, keyed by text + voice + rate): already-heard pages replay from cache with no Edge TTS round-trip.
- Seek does not tear down the A2DP sink: the player stays open across skips, eliminating the silence gap from close/reopen.
- Voice switch resumes from the current utterance instead of restarting the page.
- Audio resume preserves cursor position across wake/sleep (no restart from page top).
- Cursor color and sentence-band rendering fixed after font/layout changes.
- Bangla TTS voice selection fixed.
- Settings panel now closes correctly on back tap in audio mode.
- Double-flicker on wake eliminated (frontlight off + redundant present removed).
- Radio reconnect on wake: WiFi/BT are brought back up correctly after extended sleep.
- No-blink transitions: redundant frontlight-off + present calls removed from the wake sequence.
- Page count and chapter number below the disk now update on auto-advance (was stuck at Page 1 in audio mode).
- Left/right swipe added for page navigation (was bypassed in audio mode).
- Bookmark jump seeks within the full chapter audio instead of replacing it with one page.
- Progress bar and swipe navigation seek within the chapter instead of reloading page audio.

### Library
- Onboarding guide: first launch opens a built-in guide book ("KoThok - Getting Started") instead of the library grid. Updates open the guide at a "What's New" chapter generated from this changelog. The guide lives in the library like any other book, so it can be reopened any time.
- Book content cache now stores the TOC tree alongside chapters (`CACHE_FORMAT` bumped 2 -> 3).
- EPUB scanner skips hidden directories (.adds, .kobo, etc.). Test books and extracted content no longer pollute the library.

### Fonts
- On-demand font download over WiFi: missing fonts auto-download the first time a book in that script is opened. No prompt, no language picker.
- NotoSans.ttf (Latin/greek/Cyrillic) can also auto-download and installs for all three scripts.
- Lets the KoboRoot.tgz ship without 17 MB of CJK fonts - they download on first use.

### Panel / UI
- About screen checks for app updates.
- WiFi and Bluetooth selectors are now tri-state: off (black) / connecting (red) / connected (green), with live status labels.
- Unified headers with round icon buttons across all screens.
- Portrait splash screen redesign.
- Library page header.
- About screen updated: contact info, GitHub, LinkedIn.
- Sleep timeout selector added to the Settings panel under Display.
- Version shown on About page from a single source (Cargo.toml via env!).
- Heading scale uses the live `body_px`, not the compile-time `BODY_PX` constant, so font-size changes re-scale headings correctly.
- CJK grid preloaded for fonts that need it.

### Installer
- Public installer (install.ps1 / install.bat / install.sh / install.command): downloads binary from GitHub releases, auto-detects first install vs update.
- Uninstaller (uninstall.ps1 / uninstall.bat / uninstall.sh / uninstall.command): removes KoThok files + strips its NickelMenu entry from the shared config, preserving other mods.
- Manual-install zip (KoThok-<version>-manual-install.zip) with drag-and-drop INSTRUCTIONS.txt.
- KoboRoot.tgz packaging produces the raw binary asset (`kothok-<version>`) that the installer downloads.
- Packaging uses plain tar instead of WSL (removed WSL dependency).
- run.sh self-chmods the binary before launch (removes exec-mode dependency on the tar tool).

### Infrastructure
- `BUILD_TAG` is now the Cargo version (`v0.2.0`) via `env!("CARGO_PKG_VERSION")` instead of a hardcoded string.
- Font load status frames during splash.
- Sample book installer added to deploy scripts.
- Crash report written to `.adds/crash.log` via panic hook.
- Performance: image name lookup uses a HashMap (was O(n^2) linear scan); Vec allocation hoisted out of pre-block wrap loop.
- 7 review findings closed: MONO_SCALE constant, BT fail count reset, picker repaint guard, figure leading gap, caption hit-test extraction, block indent clamp, orphan text gap-only collector.
- Heading-level tag aliases resolved: `row_flags()` returns 0 for non-body rows, pinned with tests.

### Known limitations
- Exit to nickel requires reboot
- A2DP Bluetooth fatigues after many connect/disconnect cycles
- Color e-ink: partial updates may leave ghosting, full updates flash
- PDF not supported (EPUB only)

