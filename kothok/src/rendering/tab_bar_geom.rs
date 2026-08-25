// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan

pub fn tab_bar_geom(w: usize) -> (usize, usize, f32) {
    let s = w as f32 / 1264.0;
    let font_px = (33.0 * s).round().max(22.0);
    let gap = (16.0 * s).round().max(8.0) as usize;
    let close_left = w.saturating_sub(99);
    let run_max = (close_left - 23 - 16 - gap) / 3;
    let seg_w = run_max.min((240.0 * font_px / 33.0).round() as usize);
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
            for label in ["Chapters", "Words", "Bookmarks"] {
                let label_w = measure_text(label, font_px) as usize;
                assert!(
                    label_w <= seg_w,
                    "{name} (w={w}): \"{label}\" ({label_w}px) does not fit seg_w ({seg_w}px)"
                );
            }
            assert!(
                23 + 3 * seg_w + 2 * gap <= w - 99 - gap,
                "{name} (w={w}): bar end ({}) does not clear close button (w-99={})",
                23 + 3 * seg_w + 2 * gap,
                w - 99 - gap,
            );
        }
    }
}
