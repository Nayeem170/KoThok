# Pipeline configuration

## Models

developer_model = GLM 5.2 (xhigh)
reviewer_model = Claude Sonnet 5 (high)

## Commands

preflight = docker info
preflight_fix = Start-Process Docker Desktop.exe

build = cross build --target armv7-unknown-linux-musleabihf --release -p kothok-app
test = cross test -p kothok-app --target armv7-unknown-linux-musleabihf
lint = cross clippy -p kothok-app --target armv7-unknown-linux-musleabihf -D warnings
cargo_fmt_check = cargo fmt --manifest-path kothok/Cargo.toml -- --check

## Branching

base_branch = develop
branch_prefix = feat/

## Additional repos

none

## Convention files

AGENTS.md
docs/CODE_CONVENTIONS.md
docs/REVIEW_RULES.md

## Iteration limits

max_per_loop = 10
max_global = 50

## Deploy

deploy = User runs deploy-usb.ps1 or deploys via USB mount
