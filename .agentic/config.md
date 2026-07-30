# Pipeline configuration

## Models

# Current session IS the developer. No separate developer model needed.
# The orchestrator runs as the developer in the current session.

reviewer_id = azure-foundry/claude-sonnet-5
reviewer_variant = high

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

## Mock format

mock_format = html
