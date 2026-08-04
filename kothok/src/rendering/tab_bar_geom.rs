// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
pub fn tab_bar_geom(w: usize) -> (usize, usize, f32) {
    let s = w as f32 / 1264.0;
    let font_px = (33.0 * s).round().max(22.0);
    let seg_w_cap = (240.0 * font_px / 33.0).round() as usize;
    let gap = (16.0 * s).round().max(8.0) as usize;
    let run_max = if w > 99 + 23 + 16 + 2 * gap {
        (w - 99 - 23 - 16 - 2 * gap) / 3
    } else {
        0
    };
    let seg_w = seg_w_cap.min(run_max);
    (seg_w, gap, font_px)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kobo_core::rendering::draw::measure_text;

    const FLEET: &[(&str, usize)] = &[
        ("Touch/Mini", 600),
        ("Glo/Aura/Nia", 758),
        ("Clara", 1072),
        ("Aura H2O", 1080),
        ("Libra", 1264),
        ("Elipsa", 1404),
        ("Forma/Sage", 1440),
    ];

    #[test]
    fn fleet_measure_text() {
        for &(name, w) in FLEET {
            let (seg_w, gap, font_px) = tab_bar_geom(w);
            let trailing_gap = if w > 99 + 23 + 16 + 2 * gap + 3 * seg_w {
                w - 99 - 23 - 16 - 2 * gap - 3 * seg_w
            } else {
                0
            };
            let label_w = measure_text("Chapters", font_px);
            assert!(
                trailing_gap >= 16,
                "{name} (w={w}): trailing_gap={trailing_gap} < 16 (seg_w={seg_w}, gap={gap})"
            );
            assert!(
                label_w <= seg_w,
                "{name} (w={w}): \"Chapters\" ({label_w}px) does not fit seg_w ({seg_w}px)"
            );
        }
    }
}
