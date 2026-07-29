# Pipeline Orchestrator

This document describes how the main Kilo session orchestrates the two agents.
You (the main session) are the orchestrator. You manage the flow, relay messages,
create branches, and handle user interaction.

## Agents

- **Developer**: Agent Manager session using the developer model from config.md
- **Reviewer**: Agent Manager session using the reviewer model from config.md

Both are persistent Agent Manager worktree sessions. Create them once per task,
prompt them across iterations, stop them when the task is done.

## State tracking

Write `.agentic-tasks/<task>/state.md` on every phase transition. This is the
restart checkpoint. Format:

```
phase: <current phase name>
global_iterations: <total review iterations across all loops>
phase_iterations:
  mock: <N>
  plan: <N>
  testplan: <N>
  code: <N>
updated: <ISO timestamp>
```

Global budget: max_global_iterations from config.md (default 50) total
iterations across all loops. Check after EVERY increment, not just at DoD.
When exceeded, run ABORT path.

Per-loop counters NEVER reset on re-entry. If Step 5 runs 5 iterations,
accepts, then Step 6 sends it back, Step 5 continues from iteration 6.
This is what makes the global budget effective — the counters accumulate.

## Task lifecycle

### Step 0: Initialize task

```
1. Read .agentic/config.md for project settings
2. Create task directory: .agentic-tasks/<task-name>/
3. Copy templates from .agentic/task-template/ into the task directory
4. Write the user's requirement into requirement.md
5. Create branch + worktree:
   - Branch: type/<task-name> from base_branch (config.md)
   - Worktree: Agent Manager worktree mode
   - For additional_repos: create branches in each (same branch name)
6. Start two Agent Manager sessions:
   - Developer: worktree mode, developer model + variant from config.md
   - Reviewer: worktree mode, reviewer model + variant from config.md
7. Write state.md: phase=requirement, all counters 0
```

### Step 1: Requirement clarification

```
1. Prompt developer: "Phase 1 - read requirement.md, explore codebase,
   ask any clarifying questions"
2. If developer asks questions -> relay to user -> relay answers back
3. When developer confirms requirement is clear -> write state.md, proceed
```

### Step 2: Design decisions

```
1. Prompt developer: "Phase 2 - explore codebase, identify design decisions
   (database, architecture, data structures, storage, algorithms). Research
   options (web search). Present options with trade-offs."
2. Prompt reviewer: "Feasibility-check these design options against the
   actual codebase. Read the source the developer would modify. Flag any
   option that is infeasible (wrong API, missing dependency, impossible
   on this hardware). Do NOT pick for the user — just remove options
   that cannot work."
3. Relay feasible options to user:
   "These design decisions need your input:
    <list of decisions with feasible options and trade-offs>
    Please choose for each."
4. Relay user's choices back to developer.
5. Write state.md, proceed to Step 2.5
```

Design decisions always come before mock so the UI mock reflects the actual
data model and architecture.

### Step 2.5: UI Mock (only if feature involves UI)

```
Check: does the requirement involve UI changes?

IF YES:
  Check: was a mock/screenshot/design provided by the user?

  IF MOCK PROVIDED and no requirement gaps:
    -> Copy mock to task directory, proceed to Step 3

  IF NO MOCK PROVIDED:
    1. Prompt developer: "Phase 2.5 - create a UI mock for this feature
       based on the approved design decisions. Write it to mock.md."
    2. MOCK REVIEW LOOP (max iterations from config.md):
       a. Prompt reviewer: "Review the UI mock at .agentic-tasks/<task>/mock.md
          against the requirement AND design decisions. Check: completeness,
          usability, alignment with existing UI patterns, can the approved
          architecture supply the data this mock shows?"
       b. If reviewer says the architecture cannot supply the mock's data:
          - Escalate back to Step 2 (revisit design decisions)
          - Do NOT iterate on the mock
       c. If reviewer gives other feedback -> relay to developer -> revise -> repeat
       d. When reviewer accepts -> proceed
    3. Present mock to user:
       "Here is the proposed UI mock for this feature. <show mock>
        Reply 'approved' to proceed, or describe changes."
    4. If user requests changes -> relay to developer -> revise -> re-review -> re-present
    5. When user approves -> write state.md, proceed to Step 3

IF NO (no UI changes):
  -> Proceed directly to Step 3
```

### Step 3: Plan + plan review loop

```
1. Prompt developer: "Phase 3 - write plan.md with user-approved design
   decisions, and definition-of-done.md."
2. Wait for developer to confirm plan is written
3. PLAN REVIEW LOOP (max iterations from config.md):
   a. Prompt reviewer: "Review the plan at .agentic-tasks/<task>/plan.md
      against the codebase and conventions. Read actual source files.
      Note: design decisions are user-approved - review feasibility and
      consistency only, do not override user's choices."
   b. If reviewer says "PLAN ACCEPTED" -> write state.md, proceed to Step 3.5
   c. If reviewer gives feedback:
      - Write feedback to .agentic-tasks/<task>/iterations/<N>-plan-review.md
      - Prompt developer: "Revise the plan based on this feedback: <feedback>"
      - Wait for developer to confirm revision
      - Increment phase + global counters
      - If global budget exceeded -> ABORT
      - Write state.md
   d. If max iterations reached -> stop, present to user for manual review
```

### Step 3.5: Test case plan + test case review loop

```
1. Prompt developer: "Phase 3.5 - write test-plan.md listing every test
   scenario. Do NOT write test code yet."
2. Wait for developer to confirm test plan is written
3. TEST PLAN REVIEW LOOP (max iterations from config.md):
   a. Prompt reviewer: "Review the test plan at
      .agentic-tasks/<task>/test-plan.md. Check coverage."
   b. If reviewer says "TEST PLAN ACCEPTED" -> write state.md, proceed to Step 4
   c. If reviewer gives feedback:
      - Write feedback to .agentic-tasks/<task>/iterations/<N>-testplan-review.md
      - Prompt developer: "Add missing test scenarios: <feedback>"
      - Increment phase + global counters
      - If global budget exceeded -> ABORT
      - Write state.md
   d. If max iterations reached -> stop, present to user
```

### Step 4: Implementation

```
1. Prompt developer: "Phase 4 - implement the approved plan AND write test
   code following the approved test plan. Run build and test. Confirm when
   build+test pass."
2. Developer runs:
   - Preflight: docker info. If down, run preflight_fix from config.md.
     Poll docker info every 5s up to 120s. Only escalate if Docker is
     not installed.
   - Build: cross build. Fix all errors.
   - Test: cross test (ALL tests). Fix all failures.
   - Lint: if config.md lint is non-empty, run it. Fix all warnings.
   - Iterate until build+test+lint all pass.
   - Commit all changes (AFTER fixes).
    - Rebuild + retest + re-lint against committed tree.
    - Verify git status --porcelain -- ':!.agentic-tasks' is empty.
3. Developer MUST dump raw output from the rebuild to:
   .agentic-tasks/<task>/iterations/build-latest.log
   Line 1: `=== HEAD: <git rev-parse HEAD> | <timestamp> ===`
   Line 2: `=== TREE: CLEAN ===` (or DIRTY with file list)
   Last line: `=== TOTAL PASSED: <N> ===` (sum across all test result lines)
   AND copy to iterations/build-1.log (B=1, preserved for audit).
   This is the evidence the reviewer reads. "I ran it" is not enough.
4. Wait for developer to confirm: rebuild passes, tree clean, log written.
5. Write state.md, proceed to Step 5.
```

### Step 5: Code + test review loop

```
1. CODE REVIEW LOOP (max iterations from config.md):
   a. Prompt reviewer: "Review the code changes. Read git diff against
      plan AND test-plan. Also read iterations/build-latest.log for the
      raw build+test+lint output - verify it actually passed. Branch:
      <branch-name>. Use git rev-parse <branch-name> to verify the log's
      HEAD header SHA."
   b. Reviewer checks:
      - Implementation code: conventions, AI artifacts, scope, regression
      - Test code: scenario match, mock data accuracy, assertion specificity
       - Build log: does it show 0 test failures, 0 warnings from the
         developer's code? (dependency warnings are OK)
         HEAD SHA matches git rev-parse <branch>?
         TREE says CLEAN (second line)?
         TOTAL PASSED footer = sum of test result lines, >= scenarios?
         (not the developer's claim - the actual log output)
      - Git hygiene: commits, branch naming, no secrets, LF endings
   c. If reviewer says "CODE ACCEPTED" -> write state.md, proceed to Step 6
   d. If reviewer gives feedback:
      - Write feedback to .agentic-tasks/<task>/iterations/<N>-code-review.md
       - Prompt developer: "Address this feedback, build+test+lint until
         clean, commit, rebuild against committed tree, verify
         porcelain -- ':!.agentic-tasks' empty, dump build-latest.log
         (HEAD + TREE: CLEAN + TOTAL PASSED) AND build-<B>.log (increment B),
         confirm when done: <feedback>"
      - Increment phase + global counters
      - If global budget exceeded -> ABORT
      - Write state.md
   e. If max iterations reached -> stop, present to user
```

### Step 6: Definition of done verification

```
1. Prompt reviewer: "Verify the definition-of-done checklist at
   .agentic-tasks/<task>/definition-of-done.md. Read actual code to confirm
   each item."
2. If reviewer flags missing items:
   - Increment global counter
   - If global budget exceeded -> stop, present to user
   - Else -> back to Step 5 (code review)
3. If all items pass -> write state.md, proceed to Step 7
```

### Step 7: User device check

```
1. Present to user:
   - Summary of what was implemented
   - Files changed (git diff stat)
   - Test results (from build-latest.log)
   - DoD status
2. Build and deploy:
   - If deploy in config.md is non-empty: orchestrator runs it.
   - If deploy is empty: user deploys manually. State this explicitly:
     "Deploy the binary to the device. Binary path: <path from build>.
      Use the deploy script: kothok/scripts/deploy.ps1"
3. Tell user: "Test on device. Reply 'confirmed' to merge, 'abort' to
   cancel, or describe issues."
4. If user reports issues -> TRIAGE:
   - Design/architecture flaw (wrong waveform, wrong storage layout, wrong
     data model) -> back to Step 3 (replan with the issue as new input)
   - Code bug (crash, wrong value, display glitch) -> back to Step 5
     (code review with the issue as feedback)
5. If user says 'abort' -> run ABORT path (below)
6. If user confirms -> proceed to Step 8
```

### Step 8: Merge and cleanup

```
1. SYNC develop (all repos):
   a. git fetch origin
   b. git checkout develop && git pull origin develop
   c. Repeat for each additional_repo
   d. If pull fails (divergent history) -> stop, present to user

2. PRE-MERGE VERIFICATION (in each repo with changes):
   a. Checkout feature branch
   b. Merge develop into feature branch
   c. If conflicts: prompt developer "Phase 6 - resolve merge conflicts.
      Preserve intent of both branches. Do not blindly pick one side."
   d. Developer: build+test+lint until clean, commit (merge + fixes),
      rebuild against committed tree, verify porcelain -- ':!.agentic-tasks'
      empty, dump raw output (HEAD + TREE: CLEAN + TOTAL PASSED) to
      iterations/merge-verify-build.log.
   e. If conflicts were resolved -> mandatory reviewer pass (Conflict
      Resolution Review stage) on the merged files AND
      merge-verify-build.log. No exceptions. Developer does not
      self-grade conflict fixes.
   f. If build or test fails -> fix in worktree -> rebuild -> repeat
   g. Only proceed when clean - this proves existing code AND new
      implementation work together against latest develop

3. MERGE OUT (dependency order):
   a. Merge additional_repos first (kobo-core, then kothok-media)
      - Each: checkout develop, merge feature branch (--no-ff)
      - If any merge fails -> REVERT all already-merged repos -> stop
   b. Merge primary repo (EReader) last
      - checkout develop, merge feature branch (--no-ff)
      - If fails -> REVERT additional_repos -> stop
   c. Push all develop branches to origin

4. CLEANUP:
   - Delete worktree
   - Delete feature branches in all repos
   - Stop both Agent Manager sessions
   - Update docs/REVIEW_RULES.md (cap at 50 rules, retire rules that
     haven't fired in 5 tasks)
   - Write final-summary.md
   - Write state.md: phase=DONE
5. Present completion to user
```

### ABORT path

```
If user says abort, or global budget exceeded with no resolution:

1. Stop both Agent Manager sessions
2. Delete worktree
3. Delete feature branches in ALL repos (primary + additional)
4. Write state.md: phase=ABORTED, reason=<why>
5. Write final-summary.md: what was attempted, what failed
6. Tell user: "Task aborted. Branches cleaned up. State saved at
   .agentic-tasks/<task>/state.md"
```

## Phase summary

```
Step 1   Requirement clarification        [user: answers questions]
Step 2   Design decisions                 [user: chooses architecture]
Step 2.5 UI mock (if applicable)          [user: approves mock]
Step 3   Plan + plan review loop          [reviewer: max 10]
Step 3.5 Test plan + review loop          [reviewer: max 10]
Step 4   Implementation + build gate      [developer: dumps build-latest.log]
Step 5   Code + test review loop          [reviewer: max 10, reads build-latest.log]
Step 6   DoD verification                 [reviewer: verifies source]
Step 7   User device check                [user: confirms or triages]
Step 8   Merge + cleanup                  [orchestrator: deps first, rollback]
ABORT    Kill task                        [user: 'abort', or budget exceeded]
```

User is involved at 4 points: Step 1 (questions), Step 2 (design decisions),
Step 2.5 (mock), Step 7 (device check). Everything else is autonomous.

Global budget: max_global_iterations from config.md (default 50). Checked
after every increment. Abort when exceeded.

## Gap handling

### Docker pre-flight
Developer runs preflight. If Docker is down, developer runs preflight_fix
(starts Docker), polls docker info every 5s up to 120s. Only escalate to user
if Docker is not installed at all.

### Multi-repo
additional_repos paths are relative to the repo root (primary_repo). State this
explicitly. Branches created in each. Merge order: dependencies first. On
failure, revert already-merged repos to prevent split-brain develop.

### Scope guard
Plan has explicit out-of-scope section. Reviewer rejects changes outside plan.
If legitimate additional work is discovered, developer amends plan and it goes
through plan review again.

### Regression
Developer runs FULL test suite every iteration. Dumps raw output to
build-latest.log. Reviewer reads build-latest.log, not developer claims.
Reviewer also checks for regression indicators in code (changed signatures,
removed guards, altered flow).

### Build verification
Developer builds + fixes + commits, then rebuilds against the committed
tree and dumps raw output to iterations/build-latest.log. The log has a
three-part integrity block:
- Line 1: `=== HEAD: <sha> | <timestamp> ===`
- Line 2: `=== TREE: CLEAN ===` (or DIRTY with file list)
- Last line: `=== TOTAL PASSED: <N> ===` (sum across all test binaries)

Cleanliness is checked with `git status --porcelain -- ':!.agentic-tasks'`
so task artifacts (logs, response files) don't count against the tree check.

Two of the three checks are independently verifiable by the reviewer:
- HEAD SHA: verified against `git rev-parse <branch>` (not HEAD - different
  worktree). Mismatch = stale or fabricated log.
- TOTAL PASSED: self-checksumming. Reviewer sums all `test result:` lines
  in the raw log and compares to the footer. Mismatch = fabricated count.

The third, TREE, is a procedural assertion, not an independent check:
cleanliness is a point-in-time property already gone by review time, and
the reviewer's worktree has its own status. It catches honest mistakes
(the developer forgot to commit a file) but not deliberate fabrication.

Accepted residual risk: the build log is written by the developer, the same
agent whose claims motivated the gate. The HEAD SHA and TOTAL PASSED checks
catch stale and fabricated logs. Neither agent independently re-runs the
build. This is an accepted trade-off to save tokens.

### Post-merge verification
Before merging out, merge develop into the feature branch, rebuild + retest.
Only merge when the branch is clean against current develop.

### Test quality
Test plan reviewed BEFORE code. Test code reviewed AFTER. Reviewer checks both
against actual source, not claims.

## Restarting after interruption

1. Read .agentic-tasks/<task>/state.md
2. state.md has: current phase, iteration counts, global count
3. Resume from the current phase with the recorded counters
4. Recreate Agent Manager sessions if needed
5. If state.md says ABORTED or DONE -> task is finished, do nothing

## Feedback deduplication

When the reviewer gives feedback that matches a pattern from a previous
iteration or previous task:
1. Add the rule to docs/REVIEW_RULES.md (under Learned rules)
2. Reset that rule's tasks_since_fired counter to 0
3. REVIEW_RULES.md caps at 50 active rules
4. At end of each task: increment all rules' tasks_since_fired by 1
5. Rules with tasks_since_fired >= 5 are retirement candidates
6. When full, retire the oldest candidate (move to Archived section)
7. The reviewer reads REVIEW_RULES.md at the start of every iteration
