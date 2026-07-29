# Agentic Pipeline Configuration

# Project-specific settings. Copy .agentic/ to another project and edit this file.
# Everything else (agent prompts, orchestrator, templates) is project-agnostic.

## Models

developer_model: "Z.ai: GLM 5.2"
developer_variant: "xhigh"
reviewer_model: "Claude Sonnet 5"
reviewer_variant: "high"

## Build commands

# Pre-flight check before build (return non-zero if not ready)
preflight: "docker info"
# If preflight fails, run this to fix the environment, then re-check
preflight_fix: "Start-Process 'C:\\Program Files\\Docker\\Docker\\Docker Desktop.exe'"
preflight_fail_message: "Docker Desktop could not be started. Ensure it is installed."

# Build the project
build: "cross build --target armv7-unknown-linux-musleabihf --release -p kothok-app"
build_workdir: "kothok"

# Run tests (ALL tests, not just new ones)
test: "cross test -p kothok-app --target armv7-unknown-linux-musleabihf"
test_workdir: "kothok"

# Lint / format check (optional, empty to skip)
# Note: cargo fmt is host-side and takes no --target; only clippy runs under cross.
lint: "cross clippy --target armv7-unknown-linux-musleabihf -p kothok-app -- -D warnings && cargo fmt -p kothok-app -- --check"
lint_workdir: "kothok"

## Repository layout

# Base branch for feature branches
base_branch: "develop"

# Primary repo root (where .git lives for this project)
primary_repo: "."

# Additional repos that may be touched
# Paths are relative to the repo root (primary_repo), NOT to build_workdir.
# Each gets its own branch on the same task branch name.
additional_repos:
  - path: "../kobo-core"
    base_branch: "develop"
  - path: "../kothok-media"
    base_branch: "main"

## Convention files (reviewer reads all of these)

convention_files:
  - "AGENTS.md"
  - "docs/CODE_CONVENTIONS.md"
  - "docs/REVIEW_RULES.md"

## Review loop

max_iterations: 10
# Total iterations across ALL loops before the pipeline aborts.
# Prevents unbounded S7(device) -> S5(code review) -> S7 cycles.
max_global_iterations: 50

## Deploy (manual - user deploys to physical device via USB)
# The agent cannot reach the Kobo. User runs deploy.ps1 after Step 7.
deploy: ""
deploy_workdir: ""
deploy_instructions: "Build output: kothok/target/armv7-unknown-linux-musleabihf/release/kothok-app. Deploy via: kothok/scripts/deploy.ps1 (the only script in kothok/scripts/)."
