---
name: agentic
description: Run the 8-step agentic development pipeline for a feature or bug fix - requirement, design, mock, plan, test plan, implementation, code review, DoD verification, device test, merge. Use when the user asks to build a feature or fix a bug through the full reviewed pipeline, or invokes /agentic.
---

# Agentic development pipeline (Claude Code)

You are the orchestrator. You are also the developer: all code reading, writing,
building, and testing happens in this session. The reviewer is a subagent.

This is the Claude Code implementation. The kilo implementation lives at
`~/.config/kilo/agent/agentic-orchestrator.md` and is a separate, parallel system.
Do not read it. Config and templates are shared between both.

## Mechanics

| Need | Tool |
|------|------|
| Spawn reviewer | `Agent` with `subagent_type: "pipeline-reviewer"`, `run_in_background: false` |
| Continue the same reviewer with context intact | `SendMessage` with the agent's id or name |
| Isolated developer (worktree mode only) | `Agent` with `isolation: "worktree"` |
| Phase tracking | `TodoWrite` - one todo per pipeline step |

Review gates are synchronous. Always pass `run_in_background: false` for reviews -
you cannot proceed until the verdict is in.

Prefer `SendMessage` over a fresh `Agent` call for every review after the first.
A continued reviewer keeps the accepted plan, the DoD, and prior feedback in context,
which is what makes iteration N+1 cheaper and more consistent than iteration 1.

## Startup

1. Read `.agentic/config.md`. If missing, read `AGENTS.md` or `CLAUDE.md`.
2. Read `docs/REVIEW_RULES.md` (the reviewer reads it too, but you need it to
   pre-empt known feedback).
3. Determine base branch from config `base_branch`. Default: prefer `develop`/`dev`, else `main`.
4. Classify the ticket: **feature** (new behavior, `feat/`) or **bug** (broken behavior, `fix/`).
5. Branch slug: branch name with `/` replaced by `-` (`feat/word-list` -> `feat-word-list`).
6. Pipeline mode: inspect the user's command/request. If it mentions worktree,
   parallel work, or multiple tickets, use worktree. Otherwise, direct.

## Pipeline mode

**Direct (default).** You are the developer. One reviewer subagent. Use this for
single tickets - simpler, lower latency, full file access.

**Worktree.** You become a coordinator: spawn a developer via `Agent` with
`isolation: "worktree"` plus a reviewer, and relay between them and the user. You
do not write code. Use only for parallel tickets.

Mode is fixed at S0 and written to `state.md`. If config changes mid-ticket, the
`state.md` value wins. Mode cannot change during a ticket.

In worktree mode the developer agent has no pipeline steps - send each step's
definition one at a time, and write every task-directory artifact yourself from
the developer's returned report. The developer writes only inside its worktree.
The `isolation: "worktree"` worktree is auto-cleaned when unchanged, so commit
before finishing.

## Iteration limits

- Per-loop cap: 10 iterations per review step
- Global cap: 50 iterations per pipeline (per branch, never summed across branches)
- `global_iterations` increments after every review cycle

At the per-loop cap: stop the loop, present remaining feedback plus your best
attempt. User picks accept-as-is / abort / override (resets the per-loop counter,
increments global). At the global cap: stop and present the same three choices.
The pipeline does not abort automatically and never force-deletes a branch on
a counter breach - branch deletion requires explicit user confirmation.

Fix only BLOCKING, CRITICAL, and HIGH. MEDIUM is logged and fixed if cheap.
SUGGESTION is logged only. A blocking fix is minimal and targeted, never bundled
with unrelated changes.

## Task directory

`<repo-root>/.agentic-tasks/<branch-slug>/` plus an `iterations/` subdirectory.

Templates: project templates in `.agentic/task-template/` override global ones in
`~/.config/kilo/agent/task-template/`.

```
state.md                 restart checkpoint
requirement.md           what the ticket delivers
design-decisions.md      features only
mock.md / mock.pen / mock-preview.png   UI changes only (features or bugs)
plan.md
test-plan.md
bug-reproduction.md      bugs only
definition-of-done.md    extracted from plan.md
build-latest.log
merge-verify-build.log
iterations/N-review.md   reviewer feedback
iterations/N-response.md your response
final-summary.md
```

Update `state.md` on every phase transition: set `phase`, refresh `updated`,
append `{phase, entered_at}` to `phase_log`. This is the restart checkpoint.

## Feature pipeline

### S0: Initialize

Shared (all modes):
1. Inject lessons learned from the last 3 completed `final-summary.md` files
   (their "Preventive rules generated" and "Pipeline health" sections). If none
   exist, skip.
2. Run preflight from config. If it fails, stop and report. Do not proceed.
3. Run the test command on the base branch. If tests fail, stop and report - a red
   base makes every downstream test delta meaningless. Never record a failing baseline.
4. Parse TOTAL PASSED. Record as `tests_baseline`.
5. Create the feature branch from the base branch.

Direct mode:
6. Create the task directory. Copy templates.
7. Spawn the reviewer (`Agent`, `subagent_type: "pipeline-reviewer"`). Keep its id.
8. Write `state.md` with `started_at`, `tests_baseline`, `pipeline_mode: direct`, S0 phase_log entry.

Worktree mode:
6. Create the task directory in the main working directory, not the worktree. Copy templates.
7. Spawn the developer (`Agent`, `isolation: "worktree"`) with the requirement, branch,
   base branch, `tests_baseline`, ticket type, and build commands. Keep its id.
8. Spawn the reviewer. Keep its id.
9. Write `state.md` with `started_at`, `tests_baseline`, `pipeline_mode: worktree`, S0 phase_log entry.

### S1: Requirement

Explore the codebase. Ask the user if anything is ambiguous. Write `requirement.md`.

### S2: Design decisions

Research options with real trade-offs. Send them to the reviewer for feasibility
filtering - the reviewer reads source to verify each option is actually buildable.
Present surviving options to the user. User chooses. Write `design-decisions.md`.

If the ticket changes UI -> S2.5. Otherwise -> S3.

### S2.5: UI mock

Read `pen_cli` from config (default `pen`). Requires `pen login` or `PEN_CLI_KEY`
in the user environment - never committed.

1. Write `mock.md`: device dimensions, screen descriptions, approved design
   decisions, interactive states, existing UI patterns referenced.
2. `<pen_cli> --out mock.pen --prompt-file mock.md --enable-preview`
3. `<pen_cli> --in mock.pen --export mock-preview.png --export-scale 2`
4. Present the preview to the user for visual approval first. The user is the
   only party who can reliably see the rendered mock. If rejected, update
   `mock.md` with feedback and regenerate (steps 2-3).
5. When user approves, send `mock.md` (context) + `mock-preview.png` (visual) to
   the reviewer for consistency review against design decisions (max 10 iterations).
   If the reviewer cannot read the image, send prose only and note that visual
   approval was done by the user at step 4.
6. On feedback, regenerate via a temp file so a same-file read/write cannot truncate:
   ```
   <pen_cli> --in mock.pen --out mock.new.pen --prompt-file mock.md
   Move-Item -Force mock.new.pen mock.pen
   <pen_cli> --in mock.pen --export mock-preview.png --export-scale 2
   ```
7. On ACCEPTED -> S3.

If the Pencil CLI is unavailable, fall back to `mock.html` at device dimensions.
Present to user first, then reviewer (same order).

### S3: Plan + DoD

Write `plan.md`: architecture, files to change, out-of-scope, risks, DoD.
Extract the DoD into `definition-of-done.md` - every item needs a pass/fail
criterion checkable by reading code or running a command.
Reviewer reviews against actual source. Max 10 iterations.

### S3.5: Test plan

Write `test-plan.md` - scenarios only, no code. Reviewer checks coverage. Max 10.

### S4: Implementation

Write code and tests. Run the build gate: preflight, build, test, lint, fmt check.
Fix every error. Commit. Dump `build-latest.log`.
Verify `git status --porcelain -- ':!.agentic-tasks'` is empty before proceeding.

### S5: Code + test review

Send the reviewer the full `git diff <base>..<branch>` and `build-latest.log`.
Feedback loop, max 10. Write each round to `iterations/N-review.md` and
`iterations/N-response.md`.

### S6: DoD verification

Reviewer checks every `definition-of-done.md` item against actual source.
Items missing due to incomplete implementation -> back to S5.
Items missing because the plan never covered them -> back to S3 (replan).
When all pass -> S7.

### S7: User acceptance test

Present summary, diff, and build results. Print the `deploy` instruction from config.
Route the user's verdict:
- New bug caused by this ticket -> bug-fix sub-pipeline below
- Simple code bug -> S5
- Design flaw -> S3
- Abort -> abort path
- Confirmed -> S7.5

### S7.5: Pipeline health check

Before advancing to S8, scan all iteration files for patterns:
- Same issue flagged 3+ times across iterations -> pipeline relay bug or developer
  avoidance. Flag to user.
- User had to repeat instructions the pipeline already received -> orchestrator
  context loss. Flag to user.
- Reviewer re-flagged fixed items -> relay gap. Log for reviewer improvement.
- Developer deferred BLOCKING items -> enforcement gap. Log for next task.

Present findings as "Pipeline health" section in the user acceptance summary.

### S8: Merge + cleanup

1. `git fetch origin && git checkout <base> && git pull origin <base>`
2. `git checkout <branch> && git merge <base>` - resolve conflicts
3. Rebuild and retest. Dump `merge-verify-build.log`.
4. Reviewer reviews the conflict resolutions.
5. **Ask the user to confirm the push.** Present branch, commit count, build result.
   Never push without explicit confirmation.
6. `git checkout <base> && git merge --no-ff <branch> && git push origin <base>`
7. Write `final-summary.md` using the global template. Every number from a real command.
8. End-of-task analysis: categorize feedback (code pattern -> R##, developer gap -> D##,
   pipeline relay -> P##). Present preventive rules to user with self-healing suggestions.
   Wait for user approval before writing new rules.
9. On user approval, write new rules to the appropriate files.
10. Review rules lifecycle (see below).
11. Worktree mode: ensure the developer's work is committed, then let the worktree
    be cleaned up.

## Bug pipeline

Same structure, with S1.5 (reproduction) replacing S2 (design decisions).

### S1: Requirement
Describe what is broken, reproduction steps, expected vs actual. Explore the
affected code.

### S1.5: Bug reproduction

| Bug type | Reproduction | Automated? |
|----------|-------------|------------|
| Logic/state | Unit test that FAILS | Yes |
| API/contract | Integration test that FAILS | Yes |
| Component state | Framework test (i-slint-backend-testing) | Yes |
| Layout/geometry | Framework test asserting element geometry | Yes |
| Rendering | Screenshot test vs expected output | Yes |
| Touch/interaction | Framework test simulating input events | Yes |
| Animation/timing | Given/When/Then script | No |
| Hardware-dependent (e-ink, frontlight, sleep) | Given/When/Then script | No |

Automated: write a test that currently FAILS. Do not fix the bug yet.
Manual: write `bug-reproduction.md` with exact preconditions, steps, expected vs
observed, and Given/When/Then acceptance criteria.

Reviewer checks it reproduces the actual bug, is isolated, and is precise. Max 10.

When accepted:
- If the fix changes UI layout or interaction -> S2.5 (mock the before/after so the
  user approves the visual change before code is written).
- Otherwise -> S3.

### S3: Root cause + fix plan
`plan.md` with root cause at a specific `file:line`, fix approach, side effects,
out-of-scope, DoD. Reviewer checks the root cause matches the reproduction and the
fix is minimal.

### S4: Implement fix
The reproduction test must now PASS. For manual reproductions, verify the fix
addresses each step. Build gate, commit, `build-latest.log`.

S5 through S8 are identical to the feature pipeline.

## Bug-fix sub-pipeline (within S7)

When device testing surfaces a new bug:

1. **Classify.** Caused by this ticket -> run the sub-pipeline. Pre-existing ->
   log it, do not fix on this branch, finish the current pipeline first.
2. **S7-A Reproduce** - failing test or script, reviewed. Increments `bug_repro` + global.
3. **S7-B Plan** - focused fix plan, reviewed. Increments `bug_plan` + global.
4. **S7-C Fix** - implement, reproduction passes, build gate, commit.
5. **S7-D Review** - reviewer reviews fix + passing test. Increments `bug_code` + global.
6. **S7-E DoD re-check** - original DoD must still hold. Broken -> back to S7-D.
7. Back to S7: user retests on device.

## Phase gating

Never skip a phase. Never send multiple phase instructions in one prompt. Each
phase completes on reviewer ACCEPTED or an explicit user decision.

## Review rules lifecycle (S8)

If `docs/REVIEW_RULES.md` exists:

a. Read `final-summary.md`. Extract recurring feedback patterns from the
   "Device-test bugs found and fixed" and "Patterns established" sections.
   Cross-reference `iterations/*.md`.
b. Add a rule for each recurring pattern as `[task-name|0]`.
c. Increment every active rule counter: `[source|N]` -> `[source|N+1]`.
d. Reset any rule that fired during this task to `[source|0]`.
e. Archive rules at counter >= 5 (append ` at retirement`).
f. If active rules exceed 50, retire the lowest-count rule.

## Final summary

Write `final-summary.md` from the template. Every number comes from a real command:

| Field | Source |
|-------|--------|
| Commits | `git rev-list --count <base>..<branch>` |
| Files/lines | `git diff --stat <base>..<branch>` |
| Tests before | `tests_baseline` from state.md |
| Tests after | TOTAL PASSED from build-latest.log |
| Tree clean | `git status --porcelain -- ':!.agentic-tasks'` |
| Iterations | `phase_iterations` from state.md, verbatim |
| Elapsed | `started_at` -> last `phase_log` entry |

Do not include token counts. You cannot measure your own token usage, so any
number would be invented. Iteration counts are the cost metric.

## Restart after interruption

Read `.agentic-tasks/<branch-slug>/state.md`: phase, `pipeline_mode`, ticket type,
iteration counts. Resume from the last completed step.

Subagents do not survive across sessions. On restart, spawn a fresh reviewer and
re-send the accepted artifacts it needs for the current phase (plan.md,
definition-of-done.md, and the current artifact). In worktree mode, also respawn
the developer with its initial prompt plus every approved artifact for the
resumed phase - it has no memory and no pipeline steps.

If config `pipeline_mode` differs from `state.md`, use `state.md`. Mode is pinned at S0.

## Abort

1. Write `state.md` with `phase: ABORTED`.
2. `git checkout <base_branch>` then `git branch -D <branch>`.
3. Write `final-summary.md`: what was attempted, what failed, where it stopped.
