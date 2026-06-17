---
name: kobo-dev
description: Development workflow for the Kobo e-reader Rust/Slint project — code conventions, testing, formatting, refactoring rules, crate architecture
---

# kobo-dev

Development rules and workflow for the BitOps EReader (Kobo Libra Colour) Rust project using Slint UI.

## Crate architecture

```
kothok/
├── crates/
│   ├── kothok-core/   # pure logic: epub, html, pagination, clock, capabilities
│   │                          NO libc, NO sysfs, NO device paths. Desktop testable.
│   ├── kobo-audio/           # audio: A2DP socket, Edge-TTS, decode/resample pipeline
│   ├── kobo/                 # device app: Slint UI, framebuffer, input, power, device I/O
│   │                          ONLY crate allowed raw libc/sysfs/ioctl
│   ├── sim/                  # desktop simulator: Slint + winit backend
├── Cargo.toml                # workspace
├── Cross.toml                # cross-compilation pre-build deps
└── docs/                     # CODE_CONVENTIONS.md, REFACTOR_PLAN.md, SHIPPING.md
```

**Dependencies point downward only**: kobo → core, kobo → audio. Never core → app.

## Code conventions (full rules in `docs/CODE_CONVENTIONS.md`)

### Size limits
- Files: ~400 lines max
- Functions: ~60 lines max
- `fn main()`: ~80 lines max (wiring only)
- Functions: no 5+ params — bundle into a struct

### Module structure
- One responsibility per module (`fb.rs` = framebuffer, `power.rs` = frontlight/suspend/wakelock)
- No `utils.rs`/`misc.rs`
- Event loop: `read → dispatch → execute → render`, dispatch via `match` on action enum

### Types
- No 3+ field tuples through multiple functions — use named structs
- Group 5+ threaded values into a state struct (`&LoopState`)

### Naming
- Standard Rust: `snake_case`, `CamelCase`, `SCREAMING_SNAKE_CASE`
- Names state intent, not type
- No abbreviations beyond `fb`, `bt`, `tts`, `px`, `ch`

### Constants
- Name every non-trivial literal as a `const`
- Voice IDs, config keys, sysfs strings MUST be consts
- Device paths centralized in one place

### Error handling
- No bare `let _ = fallible();` without `// best-effort:` reason
- Use `anyhow::Result` at app boundaries, thin `enum` error for variant branching
- Log every swallowed device-I/O failure at `warn`
- Never add a fallback to mask a root cause — fix the cause

### Logging
- Use `log` crate (`error`/`warn`/`info`/`debug`/`trace`), not `println!`/`eprintln!`

### Comments
- No comments unless explaining *why* (hardware quirks, non-obvious constraints)
- `// SAFETY:` required on every `unsafe` block

### Resource management
- RAII for all OS resources (mmap, wakelock, frontlight state, fd)
- `Fb` and `WakeLock` are the reference patterns

### Rendering / e-ink
- Book text rendered `color: transparent` in Slint, painted by Rust `composite_text` after
- Render pages onto cleared buffer (white-fill) before compositing
- FULL (GC16) only to clear ghosting; default to PARTIAL
- Collapse open/transition into single FULL for final frame

### Unsafe
- Every `unsafe` block preceded by `// SAFETY:` comment
- Keep `unsafe` blocks as small as possible
- No `unsafe`-derived aliasing hazard across `.await`

### Panic policy
- `panic="abort"` in release — panic = SIGABRT = reboot
- No `unwrap()`/`expect()`/`[i]` indexing on device paths
- `expect()` only in one-time startup wiring
- Validate indices from touch coords, page/chapter numbers, offsets before use

### Testing
- `cargo test -p kobo -p kothok-core` must pass before commit
- Pure-logic helpers extracted from event loop must have tests
- Move testable logic to `kothok-core`

## Formatting & linting

```powershell
cargo fmt --check
cargo clippy -- -D warnings
```

All code must be `cargo fmt`-clean and `cargo clippy`-clean.

## Tests

```powershell
cargo test -p kobo -p kothok-core
```

Run before every commit.

## Key dependencies

| Crate | Purpose |
|-------|---------|
| `slint = "1.8.0"` | UI framework (software renderer, `libm`, `compat-1-2`) |
| `fontdue` | font rasterization |
| `rustybuzz` | text shaping (complex scripts) |
| `image` | PNG/JPEG/GIF/BMP decode |
| `tokio` | async runtime (audio worker) |
| `libc` | raw syscalls (kobo crate only) |
| `epub` | EPUB reading |
| `scraper` | HTML → text + offset map |
| `symphonia` | MP3/PCM decode |
| `tokio-tungstenite` | Edge-TTS WebSocket |
| `rubato` | audio resampling |

## Slint UI

- UI files in `kothok/crates/kobo/ui/` and `kothok/crates/sim/ui/`
- Software renderer only (no GPU on e-ink)
- Build: `slint-build = "1.8.0"` in build-dependencies

## Refactoring

Active restructuring tracked in `docs/REFACTOR_PLAN.md`:
- Splitting the `main()` god file (~700 lines → `AppState` + handlers)
- Removing `#[allow(clippy::too_many_arguments)]` debt
- Adding tests for `app.rs` loop handlers

## Engineering principle

Never implement fallbacks. Fix the root cause.
