# KoThok Project Pipeline Overrides

This file overrides the global agentic orchestrator with project-specific settings.
The global orchestrator (`~/.config/kilo/agent/agentic-orchestrator.md`) owns the pipeline flow.
Only project-specific deltas go here.

## Project-specific build gate

Read from `.agentic/config.md` commands:
- preflight: `docker info`
- preflight_fix: `Start-Process Docker Desktop.exe`
- build: `cross build --target armv7-unknown-linux-musleabihf --release -p kothok-app` (workdir: `kothok/`)
- test: `cross test -p kothok-app --target armv7-unknown-linux-musleabihf`
- lint: `cross clippy -p kothok-app --target armv7-unknown-linux-musleabihf -D warnings`
- fmt check: `cargo fmt --manifest-path kothok/Cargo.toml -- --check`

### Build log format (build-latest.log)

Three sections, separated by blank lines:

```
=== HEAD: <git rev-parse HEAD> | <ISO timestamp> ===
=== TREE: CLEAN ===
=== TOTAL PASSED: <N> ===
```

- HEAD SHA is independently verifiable via `git rev-parse HEAD`
- TREE CLEAN verified by `git status --porcelain -- ':!.agentic-tasks'` being empty
- TOTAL PASSED: count from test output

Include full build stdout/stderr, test stdout/stderr, lint output between header and footer.

## Project-specific review rules

Read `.agentic/reviewer.md` for the full review checklist.

Project-specific rules beyond the global reviewer:
- Files < ~400 lines, functions < ~60 lines
- No `unsafe` block without a `// SAFETY:` comment
- No `unwrap()`/`expect()` on device paths (event/render/audio/input). Use `.get()`/`?`
- audio/layout sync: `build_state()` paths have `page_utterances()` + `Cmd::Reload` + `Cmd::Seek`

## Step 7 override (device test)

Deploy: user runs `kothok/scripts/deploy-usb.ps1` or deploys via USB mount.
Target: Kobo Libra Colour.

## Step 8 override (merge)

No additional repos to merge (kobo-core is a git dependency in Cargo.toml).
Standard merge flow from global orchestrator.

## Git operations

- Branch from `develop` using `type/ticket-name`
- Commits: conventional commit format (feat:/fix:/chore:/docs:/refactor:/ci:)
- Merge: `git merge --no-ff <branch>` into develop
- No direct commits on main or develop
