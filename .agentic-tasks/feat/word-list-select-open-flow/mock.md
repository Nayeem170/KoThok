# UI mock: Words tab select-then-open flow

Screen: 1264 x 1680 pixels.

Key constants from source:
- Header: 0..110 (Slint-drawn, covered by Rust tab bar)
- Tab bar band: TAB_BAR_TOP=110 .. CH_LIST_TOP=158
- List area: CH_LIST_TOP=158 .. (1680 - CH_LIST_BOTTOM_PAD=136) = 1544
- Bottom strip (Open button): 1544..1680 (CH_LIST_BOTTOM_PAD=136)
- Row: CH_ROW_H=60, CH_ROW_PITCH=70, CH_ROW_X=40

## Screen 1: Word list (no selection)

```
+============================================================+
|  [Header 0-110px - Slint, hidden behind tab bar]           |
+------------------------------------------------------------+
|  TAB BAR (110-158px)                                       |
|  +----------+  +---------+                    +----+      |
|  | Chapters |  | Words   |                    |  X |      |
|  +----------+  +---------+                    +----+      |
|  ^inactive       ^active (black pill, white text)  ^close  |
+------------------------------------------------------------+
|  LIST AREA (158-1544px)                                    |
|                                                             |
|  +------------------------------------------------------+  |
|  |  algorithm                                            |  |  row 0
|  +------------------------------------------------------+  |
|  +------------------------------------------------------+  |
|  |  binary                                               |  |  row 1
|  +------------------------------------------------------+  |
|  +------------------------------------------------------+  |
|  |  cache                                                |  |  row 2  [viewport]
|  +------------------------------------------------------+  |
|  +------------------------------------------------------+  |
|  |  database                                             |  |  row 3
|  +------------------------------------------------------+  |
|  +------------------------------------------------------+  |
|  |  embedded                                             |  |  row 4
|  +------------------------------------------------------+  |
|  ...                                                       |
|  +------------------------------------------------------+  |
|  |  kernel                                               |  |  row N
|  +------------------------------------------------------+  |
|  |                                              [scroll] |  |
+------------------------------------------------------------+
|  BOTTOM STRIP (1544-1680px)                               |
|                                                             |
|  +========================================================+|
|  |                    Open                                ||
|  +========================================================+|
|  ^greyed out / disabled (no word selected)                 |
+============================================================+

State:
  chapter_tab = Words
  search_selected_word = NONE (no highlight)
  search_results_active = false

Touch targets:
  [Chapters pill]  -> chapter_tab = Chapters (switches tab)
  [Words pill]     -> no-op (already active)
  [X button]       -> chapter_overlay_open = false (close overlay)
  [word row i]     -> search_selected_word = i, repaint
  [Open button]    -> disabled, no action
  [list area]      -> scroll word list
```

## Screen 2: Word list (word selected)

```
+============================================================+
|  [Header 0-110px - Slint, hidden behind tab bar]           |
+------------------------------------------------------------+
|  TAB BAR (110-158px)                                       |
|  +----------+  +---------+                    +----+      |
|  | Chapters |  | Words   |                    |  X |      |
|  +----------+  +---------+                    +----+      |
|  ^inactive       ^active                          ^close  |
+------------------------------------------------------------+
|  LIST AREA (158-1544px)                                    |
|                                                             |
|  +------------------------------------------------------+  |
|  |  algorithm                                            |  |  row 0
|  +------------------------------------------------------+  |
|  +======================================================+  |
|  || binary                                               ||  |  row 1 **SELECTED**
|  +======================================================+  |
|  +------------------------------------------------------+  |
|  |  cache                                                |  |  row 2
|  +------------------------------------------------------+  |
|  +------------------------------------------------------+  |
|  |  database                                             |  |  row 3
|  +------------------------------------------------------+  |
|  ...                                                       |
+------------------------------------------------------------+
|  BOTTOM STRIP (1544-1680px)                               |
|                                                             |
|  +========================================================+|
|  |                    Open                                ||
|  +========================================================+|
|  ^ENABLED (brand-red bg, white text)                       |
+============================================================+

State:
  chapter_tab = Words
  search_selected_word = 1  ("binary")
  search_results_active = false

Visual change vs Screen 1:
  Row 1: white fill + 0x94B2 border -> INK (black) fill + white text (inverted pill)
  Open button: greyed out -> enabled (brand-red background)

Touch targets:
  [word row i]     -> search_selected_word = i, repaint (change selection)
  [Open button]    -> search_results_active = true, search_results_scroll = 0
  [X button]       -> chapter_overlay_open = false (close overlay)
  [Chapters pill]  -> chapter_tab = Chapters (switches tab)
  [list area]      -> scroll word list
```

## Screen 3: Search results

```
+============================================================+
|  HEADER BAND (110-158px)                                   |
|  +----+  "<binary> - 5 matches"                          |
|  |  < |                                                   |
|  +----+                                                   |
|  ^back arrow pill
+------------------------------------------------------------+
|  LIST AREA (158-1544px)                                    |
|                                                             |
|  +------------------------------------------------------+  |
|  | Ch3: The binary search algorithm...                  |  |  result 0
|  +------------------------------------------------------+  |
|  +------------------------------------------------------+  |
|  | Ch7: ...binary tree node stores the key...           |  |  result 1
|  +------------------------------------------------------+  |
|  +------------------------------------------------------+  |
|  | Ch7: ...binary heap property...                      |  |  result 2
|  +------------------------------------------------------+  |
|  +------------------------------------------------------+  |
|  | Ch12: ...binary representation of floating point...  |  |  result 3
|  +------------------------------------------------------+  |
|  ...                                                       |
|  |                                              [scroll] |  |
+------------------------------------------------------------+
|  BOTTOM STRIP (1544-1680px)                               |
|                                                             |
|  +========================================================+|
|  |                    Open                                ||
|  +========================================================+|
|  ^greyed out / disabled (no result selected on this screen) |
+============================================================+

State:
  chapter_tab = Words
  search_selected_word = 1  (unchanged)
  search_results_active = true
  search_results_scroll = 0

Layout:
  Header band replaces tab bar (same vertical slot 110-158px)
  Back arrow: pill at BACK_X=20, same height as tab pills
  Title: '"<word>" - N matches'
  Rows: chapter label (left, 16px) + snippet text (right, body_px)
  Bottom strip: same Open button position, disabled on this screen

Touch targets:
  [< back arrow]  -> search_results_active = false, search_results_scroll = 0
                     (returns to word list, selection preserved)
  [result row i]   -> jump_to_occurrence(hit[i]) -> closes overlay, jumps to page
  [list area]      -> scroll results list
```

## Screen 4: After back from search results

```
+============================================================+
|  [Header 0-110px - Slint, hidden behind tab bar]           |
+------------------------------------------------------------+
|  TAB BAR (110-158px)                                       |
|  +----------+  +---------+                    +----+      |
|  | Chapters |  | Words   |                    |  X |      |
|  +----------+  +---------+                    +----+      |
+------------------------------------------------------------+
|  LIST AREA (158-1544px)                                    |
|                                                             |
|  +------------------------------------------------------+  |
|  |  algorithm                                            |  |  row 0
|  +------------------------------------------------------+  |
|  +======================================================+  |
|  || binary                                               ||  |  row 1 **STILL SELECTED**
|  +======================================================+  |
|  +------------------------------------------------------+  |
|  |  cache                                                |  |  row 2
|  +------------------------------------------------------+  |
|  ...                                                       |
+------------------------------------------------------------+
|  BOTTOM STRIP (1544-1680px)                               |
|                                                             |
|  +========================================================+|
|  |                    Open                                ||
|  +========================================================+|
|  ^ENABLED (selection preserved)                            |
+============================================================+

State:
  chapter_tab = Words
  search_selected_word = 1  (preserved from before search results)
  search_results_active = false
  search_scroll = 0

Same as Screen 2. The back arrow from search results does NOT clear
search_selected_word. User can re-open search results or select a
different word.
```

---

## Side-by-side: Chapters tab flow (existing)

```
CHAPTERS TAB - SELECT                    CHAPTERS TAB - SELECTED                AFTER OPEN
+-----------------------------------+    +-----------------------------------+    +-----------------------------------+
| TAB BAR                           |    | TAB BAR                           |    READING VIEW
| [Chapters*]  [Words]         [X]  |    | [Chapters*]  [Words]         [X]  |    (overlay closed,
|                                    |    |                                    |    chapter switched)
| LIST AREA                         |    | LIST AREA                         |
| +-------------------------------+ |    | +===============================+ |
| | 1. Chapter One                 | |    | || 1. Chapter One              || |
| +-------------------------------+ |    | +===============================+ |
| +-------------------------------+ |    | +-------------------------------+ |
| | 2. Chapter Two                 | |    | | 2. Chapter Two               | | <- SELECTED
| +-------------------------------+ |    | +-------------------------------+ |
| +-------------------------------+ |    | +-------------------------------+ |
| | 3. Chapter Three               | |    | | 3. Chapter Three             | |
| +-------------------------------+ |    | +-------------------------------+ |
|                                    |    |                                    |
| +==================================|    | +==================================|
| |            Open (disabled)       ||    | |            Open (enabled)       ||
| +==================================|    | +==================================|
+-----------------------------------+    +-----------------------------------+

tap row -> selected=idx                     tap "Open" -> close overlay,
no open action until selected                switch to chapter[idx]

WORDS TAB - NEW FLOW (matches above)
+-----------------------------------+    +-----------------------------------+    +-----------------------------------+
| TAB BAR                           |    | TAB BAR                           |    SEARCH RESULTS
| [Chapters]  [Words*]         [X]  |    | [Chapters]  [Words*]         [X]  |    (replaces tab bar
|                                    |    |                                    |     with back arrow
| LIST AREA                         |    | LIST AREA                         |     + title)
| +-------------------------------+ |    | +===============================+ |    +-------------------------------+
| | algorithm                      | |    | || algorithm                    || |    | [<]  "binary" - 5 matches    |
| +-------------------------------+ |    | +===============================+ |    |                              |
| +-------------------------------+ |    | +-------------------------------+ |    | +----------------------------+ |
| | binary                        | |    | | binary                        | | | | Ch3: ...binary search... | |
| +-------------------------------+ |    | +-------------------------------+ |    | +----------------------------+ |
| +-------------------------------+ |    | +-------------------------------+ |    | +----------------------------+ |
| | cache                          | |    | | cache                          | |    | | Ch7: ...binary tree...    | |
| +-------------------------------+ |    | +-------------------------------+ |    | +----------------------------+ |
|                                    |    |                                    |    | tap row -> jump_to_page()
| +==================================|    | +==================================|    | tap back -> word list
| |            Open (disabled)       ||    | |            Open (enabled)       ||    |   (selection preserved)
| +==================================|    | +==================================|
+-----------------------------------+    +-----------------------------------+

tap row -> selected=idx                     tap "Open" -> search_results_active
no search results until selected            = true, show results
```

## Touch flow summary

```
                        Word list
                       (no selection)
                            |
              tap word row  |
          +-----------------+
          |
          v
                        Word list
                       (word selected)
                      /             \
          tap another      tap "Open"
          word row               |
          |                      v
          v               Search results
    (re-select)          /              \
                     tap result      tap back arrow
                     row i               |
                     |                   |
                     v                   v
              jump_to_page()     Word list
              close overlay     (same word selected)
```
