# UI Mock: feat-per-book-settings

## Device dimensions

width: 1264
height: 1680

## Prompt

Two new `CompactSlider` entries added to the Settings panel, directly below the existing Font slider in the DISPLAY section. Same component, same spacing (14px), same visual style.

### Current DISPLAY section layout:

```
[DISPLAY -------------------------------]
CompactSlider  Brightness  (panel-frac 0)
CompactSlider  Font        (panel-frac 2)
SelectorRow    Sleep
```

### New DISPLAY section layout:

```
[DISPLAY -------------------------------]
CompactSlider  Brightness  (panel-frac 0)
CompactSlider  Font        (panel-frac 2)
CompactSlider  Line        (panel-frac 4)
CompactSlider  Margin      (panel-frac 5)
SelectorRow    Sleep
```

### Slider details:

**Line** (panel-frac 4):
- Label: "Line"
- Value text: `{line-spacing-val}%` (e.g. "140%")
- Range: 110..200, step 5
- fill-frac: `(line-spacing-val - 110) / 90.0`

**Margin** (panel-frac 5):
- Label: "Margin"
- Value text: `{margin-val}px` (e.g. "24px")
- Range: 8..96, step 8
- fill-frac: `(margin-val - 8) / 88.0`

### New properties on ControlPanel:

```
in property <int> line-spacing-val: 140;
in property <int> margin-val: 24;
```

Forwarded through reader.slint like font-size-val.

## Design decisions reference

- D1: Global values become defaults; slider writes per-book only while book open
- D2: Separate bookstyles file (storage, not UI concern)
- D3: Side margins only (top/bottom out of scope)

## Interactive states

- **Default**: Line shows "140%", Margin shows "24px" -- matching pre-feature constants
- **Adjusted**: User drags Line to 180% and Margin to 48px. Values update live, text reflows after debounce (same as font slider today)
- **Per-book**: Open book A with different values, then book B. Sliders snap to B's values on open, A's values on re-open

## E-ink considerations

- Same REAGL waveform as font slider changes (already handled by apply_style_reflow)
- No additional ghosting risk -- same repaginate-and-clear pattern

## Existing UI patterns referenced

- CompactSlider component (slider.slint) -- reused for both new sliders
- panel-frac(int, float) callback routing pattern (same as brightness/speed/font/volume)
- Font debounce pattern (FONT_DEBOUNCE_MS) -- shared across all three style sliders
