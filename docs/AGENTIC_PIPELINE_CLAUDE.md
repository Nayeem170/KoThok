# Agentic Pipeline - Claude Code implementation

The same 8-step pipeline as `docs/AGENTIC_PIPELINE.md`, implemented for Claude Code
instead of kilo. Two independent implementations, one shared design and one shared
set of config and templates.

Invoke with `/agentic`.

## Source of truth

| Priority | File | Scope |
|----------|------|-------|
| 1 (highest) | `.agentic/config.md` | Build commands, pipeline mode, deploy, pen_cli |
| 2 | `.claude/skills/agentic/SKILL.md` | Full pipeline flow and step definitions |
| 3 | `.claude/agents/pipeline-reviewer.md` | Severity spec, gate rules, review checklist |
| 4 | `docs/REVIEW_RULES.md` | Auto-growing rule set, read every review iteration |
| 5 (lowest) | `AGENTS.md` | Project facts, device constraints, build/deploy commands |

Templates are shared with the kilo implementation. Project templates in
`.agentic/task-template/` override global ones in `~/.config/kilo/agent/task-template/`.

## What differs from the kilo implementation

The pipeline design is identical: same 8 steps, same feature and bug flows, same
severity ladder, same iteration caps, same measured-metrics rule, same review rules
lifecycle. Only the agent mechanics differ.

| Concern | kilo | Claude Code |
|---------|------|-------------|
| Orchestrator lives in | `~/.config/kilo/agent/agentic-orchestrator.md` (primary mode) | `.claude/skills/agentic/SKILL.md` (skill) |
| Reviewer definition | `.agentic/reviewer.md`, prepended to a prompt | `.claude/agents/pipeline-reviewer.md`, is the system prompt |
| Spawn reviewer | `agent_manager(mode: "local")` | `Agent` with `subagent_type: "pipeline-reviewer"` |
| Follow-up review | `action: "prompt"` with session id | `SendMessage` with the agent id |
| Get the verdict | poll `action: "list"`, then `kilo_local_recall` | returned directly by the tool call |
| Reviewer model | `reviewer_id` + `reviewer_variant` in config | `model:` in the agent frontmatter |
| Developer worktree | `agent_manager(mode: "worktree")` + manual `git worktree add/remove` | `Agent` with `isolation: "worktree"`, auto-cleaned |
| Stop a session | `action: "stop"` | subagents end when they return |
| Phase tracking | `state.md` only | `state.md` plus `TodoWrite` for live visibility |

Three consequences worth knowing:

**No polling.** Review gates are a single synchronous `Agent` call with
`run_in_background: false`. The verdict comes back in the tool result. The whole
poll-until-idle-then-recall dance in the kilo version has no equivalent here.

**Reviewer continuity is cheap.** `SendMessage` continues the same reviewer with
its context intact, so iteration N+1 already knows the accepted plan and the prior
feedback. Use it for every review after the first; a fresh `Agent` call starts cold
and re-derives everything.

**Subagents do not survive a session.** The kilo restart path can reattach to a
running reviewer. Here, restart always spawns a fresh reviewer and must re-send the
artifacts it needs for the current phase. In worktree mode the developer must also
be respawned with every approved artifact, since it has neither memory nor the
pipeline steps.

## Config fields

`.agentic/config.md` is shared. These fields apply to both implementations:

```
base_branch      develop
preflight        docker info
build/test/lint/cargo_fmt_check
deploy           printed to the user at S7
pen_cli          pen
max_per_loop     10
max_global       50
```

These are kilo-only and ignored by Claude Code, which reads the model from the
agent frontmatter instead:

```
reviewer_id / reviewer_variant
developer_id / developer_variant
pipeline_mode
```

`pipeline_mode` is kilo-only. Worktree is activated in both implementations
when the user's command mentions worktree, parallel work, or multiple tickets.
No config field controls this -- it's detected from the prompt.

To change the reviewer model here, edit `model:` in
`.claude/agents/pipeline-reviewer.md` (`opus`, `sonnet`, or `haiku`).

## Secrets

`PEN_CLI_KEY` and any provider key live in the user environment only, never in
`.agentic/config.md` - that file is tracked in git. The gitleaks pre-commit hook
catches accidents, but its config is machine-local, so it is a backstop rather
than a guarantee.

## Runtime unknowns

Two behaviors the documents cannot settle, worth confirming on a throwaway ticket
before trusting worktree mode with real work:

- Whether a subagent prompt can carry `mock-preview.png` as an image. S2.5 has a
  stated fallback (send prose, defer visual approval to the user) either way.
- How `isolation: "worktree"` interacts with a branch that already exists, and
  whether auto-cleanup can discard uncommitted developer work.

## Pipeline flows

### Startup

```mermaid
flowchart TD
    U["User runs /agentic"] --> K["Load SKILL.md + config.md + AGENTS.md"]
    K --> ST["Classify: feature or bug"]
    ST --> BB["Determine base branch (prefer develop)"]
    ST --> SL["Generate branch slug (replace / with -)"]
    ST --> PM{"pipeline_mode?"}
    PM -- direct --> DIR["Direct: orchestrator = developer"]
    PM -- worktree --> WTM["Worktree: spawn developer (worktree) + reviewer"]
    ST --> PF["Preflight + test base branch"]
    PF --> PF2{"Pass?"}
    PF2 -- No --> FAIL["Stop: report to user"]
    PF2 -- Yes --> BL["Record tests_baseline"]
    BL --> DIR
    BL --> WTM
```

### Feature pipeline

```mermaid
flowchart TD
    S0["S0: Initialize"] --> S0p{"Preflight + base tests pass?"}
    S0p -- No --> STOP["Stop"]
    S0p -- Yes --> S1
    S1["S1: Requirement"] --> S2
    S2["S2: Design decisions"] --> S2r["Reviewer: feasibility filter"]
    S2r --> S2u["USER: choose option"]
    S2u --> UI{"UI changes?"}
    UI -- Yes --> S25
    UI -- No --> S3
    S25["S2.5: UI mock"] --> S25r["Reviewer reviews (max 10)"]
    S25r --> S25u["USER: approve mock"]
    S25u --> S3
    S3["S3: Plan + DoD"] --> S3r["Reviewer reviews plan (max 10)"]
    S3r --> S35["S3.5: Test plan"]
    S35 --> S35r["Reviewer reviews coverage (max 10)"]
    S35r --> S4
    S4["S4: Implementation"] --> S4b["Build gate + commit + build-latest.log"]
    S4b --> S5
    S5["S5: Code + test review"] --> S5r["Reviewer: git diff + build-latest.log"]
    S5r --> |"ACCEPTED"| S6
    S5r --> |"FEEDBACK"| S5
    S6["S6: DoD verification"] --> S6r{"All DoD items pass?"}
    S6r -- No --> S5
    S6r -- Yes --> S7
    S7["S7: User acceptance"] --> R{"Result"}
    R -- "New bug" --> SUB["Bug-fix sub-pipeline"]
    R -- "Code bug" --> S5
    R -- "Design flaw" --> S3
    R -- "Confirmed" --> S8
    R -- "Abort" --> ABT["Abort"]
    SUB --> S7
    S8["S8: Merge + cleanup"] --> S8p["USER: confirm push"]
    S8p --> S8g["Merge --no-ff + push + final-summary.md"]
```

### Bug pipeline

```mermaid
flowchart TD
    S0B["S0: Initialize (fix/ prefix)"] --> S1B["S1: Requirement"]
    S1B --> S15["S1.5: Bug reproduction"]
    S15 --> S15r["Reviewer reviews reproduction (max 10)"]
    S15r --> UI2{"UI layout change?"}
    UI2 -- Yes --> S25B["S2.5: UI mock (before/after)"]
    UI2 -- No --> S3B
    S25B --> S3B
    S3B["S3: Root cause + fix plan"] --> S3Br["Reviewer reviews plan (max 10)"]
    S3Br --> S4B["S4: Implement fix"]
    S4B --> S5B["S5: Code + test review"]
    S5B --> S6B["S6: DoD verification"]
    S6B --> S7B["S7: User acceptance"]
    S7B --> S8B["S8: Merge + cleanup"]
```

### Bug-fix sub-pipeline (within S7)

```mermaid
flowchart TD
    CLASSIFY{"Caused by this ticket?"}
    CLASSIFY -- Yes --> S7A["S7-A: Reproduce (max 10)"]
    CLASSIFY -- No --> LOG["Log only, do not fix on this branch"]
    S7A --> S7B2["S7-B: Plan (max 10)"]
    S7B2 --> S7C["S7-C: Fix + build gate"]
    S7C --> S7D["S7-D: Review (max 10)"]
    S7D --> S7E["S7-E: DoD re-check"]
    S7E --> |"Items broken"| S7D
    S7E --> |"All hold"| RETEST["User retests"]
```

### Iteration limits

```mermaid
flowchart TD
    REV["Review iteration"] --> CAP{"Per-loop >= 10?"}
    CAP -- No --> CONT["Continue feedback loop"]
    CONT --> REV
    CAP -- Yes --> STOP["STOP: present to user"]
    STOP --> USER{"User choice"}
    USER -- "Accept as-is" --> NEXT["Phase complete"]
    USER -- "Abort" --> ABORT["Pipeline abort"]
    USER -- "Override" --> RESET["Reset counter, continue"]
    RESET --> REV
    REV --> GLOB{"Global >= 50?"}
    GLOB -- Yes --> ABORT
```

### Severity flow

```mermaid
flowchart LR
    I["Reviewer finds issues"] --> B{"Any BLOCKING / CRITICAL / HIGH?"}
    B -- Yes --> FIX["Fix those (minimal, targeted)"]
    FIX --> RE2["Re-review"]
    B -- No --> LOG["Log MEDIUM + SUGGESTION"]
    LOG --> ACC["ACCEPTED"]
    RE2 --> B
```
