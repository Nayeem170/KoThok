# Test Plan: <task-name>

<!-- List every test scenario BEFORE writing test code. -->
<!-- The reviewer confirms coverage is complete. Only then do you write tests. -->

## Requirement coverage

<!-- Map each requirement point to at least one test scenario -->

| # | Requirement point | Test scenario | Edge case? |
|---|------------------|---------------|------------|
<!-- | 1 | User can highlight text | highlight_selects_word_range | no | -->
<!-- | 2 | Highlight persists across pages | highlight_survives_page_turn | no | -->
<!-- | 3 | Highlight survives app restart | highlight_loaded_from_positions_file | no | -->

## Edge cases

| # | Scenario | What it tests | Input | Expected |
|---|----------|--------------|-------|----------|
<!-- | 1 | empty selection | no text selected when releasing | tap without drag | no highlight created | -->
<!-- | 2 | selection at chapter boundary | selecting last word of chapter | drag to last row | highlight clamps to chapter end | -->
<!-- | 3 | multilingual text | highlight Bangla text | select Bangla chars | byte range correct for multibyte | -->

## Error paths

| # | Scenario | What it tests | Expected |
|---|----------|--------------|----------|
<!-- | 1 | positions file corrupt | malformed JSON in positions | highlight load fails gracefully, no panic | -->

## Test file placement

<!-- Which existing test file does each scenario go in? -->
<!-- Follow existing patterns: unit tests in mod, integration in tests/ -->

| Scenario | Test file | Follows pattern of |
|----------|-----------|-------------------|
<!-- | highlight_selects_word_range | src/data/highlight/tests.rs | bookmark_roundtrip in position.rs | -->

## Mock data notes

<!-- What mock data is needed? Must be realistic, not placeholder. -->
<!-- Example: real Bangla text, real chapter structure, real byte offsets -->
