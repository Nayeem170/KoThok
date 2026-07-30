# Pipeline configuration

## Pipeline mode

# direct: orchestrator IS the developer (single-session, default)
# worktree: coordinator spawns developer (worktree) + reviewer (local)
pipeline_mode = direct

reviewer_id = azure-foundry/claude-sonnet-5
reviewer_variant = high

# worktree mode only: model/variant for the developer worktree session
# direct mode does not use these (current session IS the developer)
# Note: zai/glm-5-turbo supports low/medium/high variants. xhigh is only on kilo provider.
developer_id = zai/glm-5-turbo
developer_variant = high

## Commands

preflight = docker info
preflight_fix = Start-Process Docker Desktop.exe

build = cross build --target armv7-unknown-linux-musleabihf --release -p kothok-app
test = cross test -p kothok-app --target armv7-unknown-linux-musleabihf
lint = cross clippy -p kothok-app --target armv7-unknown-linux-musleabihf -D warnings
cargo_fmt_check = cargo fmt --manifest-path kothok/Cargo.toml -- --check

## Branching

base_branch = develop

## Branch naming

type_prefix: feat/ for features, fix/ for bugs
slug_rule: replace / in branch name with - (e.g. fix/word-list-bug -> fix-word-list-bug)

## Task directory

task_root = .agentic-tasks/
path: <repo-root>/.agentic-tasks/<branch-slug>/
branch_slug: branch name with / replaced by -

## Additional repos

additional_repos = []
(legacy: kobo-core is now a git dependency in Cargo.toml)

## Convention files

AGENTS.md
docs/CODE_CONVENTIONS.md
docs/REVIEW_RULES.md

## Iteration limits

max_per_loop = 10
max_global = 50
per_loop_cap_exit: present to user. User chooses: accept as-is (phase complete), abort, or override (reset counter, continue).

## Deploy

deploy = User runs deploy-usb.ps1 or deploys via USB mount

## Pencil

pen_cli = pen
(pen.dev CLI binary name. Requires pen login or PEN_CLI_KEY env var.)
(PEN_CLI_KEY must only exist in the user environment, never committed to git.)
