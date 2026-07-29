# Definition of Done: <task-name>

<!-- Every item must be verifiable by reading code or running a command. -->
<!-- The reviewer checks each item against actual source, not docs. -->

## Build
- [ ] `cross build` succeeds with no warnings from new code
- [ ] `cross test` passes (ALL tests, not just new ones)

## Convention compliance
- [ ] ASCII-only in all source files (no em dash, smart quotes, unicode)
- [ ] LF line endings (no CRLF)
- [ ] No comments unless explaining non-obvious WHY
- [ ] No fallback implementations
- [ ] Conventional commit messages
- [ ] Branch named type/<task-name>

## Requirement coverage
<!-- One item per requirement point. Map to specific code. -->
- [ ] <requirement point 1> - implemented in <file:function>
- [ ] <requirement point 2> - implemented in <file:function>

## Test coverage
- [ ] New feature has tests
- [ ] Edge cases covered
- [ ] Tests follow existing patterns in the codebase

## Scope
- [ ] No changes outside plan.md scope
- [ ] No unrelated refactoring

## Audio/layout sync (if applicable)
<!-- Only if touching TTS or layout -->
- [ ] build_state() callers also reload audio driver
- [ ] page_utterances + Cmd::Seek pattern followed
