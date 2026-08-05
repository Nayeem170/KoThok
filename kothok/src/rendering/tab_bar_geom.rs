// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use kobo_core::rendering::draw::measure_text;

const TAB_LABELS: &[&str] = &["Chapters", "Words", "Marks"];

pub fn tab_bar_geom(w: usize) -> (usize, usize, f32) {
    let s = w as f32 / 1264.0;
    let font_px = (33.0 * s).round().max(22.0);
    let gap = (16.0 * s).round().max(8.0) as usize;
    let close_left = w.saturating_sub(99);
    let reserve = 23 + 16 + 2 * gap;
    let label_w: usize = TAB_LABELS
        .iter()
        .map(|l| measure_text(l, font_px) as usize)
        .max()
        .unwrap_or(0);
    let avail = close_left.saturating_sub(reserve);
    let seg_w = if avail >= 3 * label_w {
        avail / 3
    } else {
        label_w
    };
    let seg_w = seg_w.min((240.0 * font_px / 33.0).round() as usize);
    (seg_w, gap, font_px)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            let label_w = measure_text("Chapters", font_px) as usize;
            let trailing_gap = w.saturating_sub(label_w + PAD_PX + 2 * gap + 3 * seg_w);
            assert!(
                label_w <= seg_w,
                "{name} (w={w}): \"Chapters\" ({label_w}px) does not fit seg_w ({seg_w}px)"
            );
            assert!(
                trailing_gap >= 16,
                "{name} (w={w}): trailing_gap={trailing_gap} < 16 (seg_w={seg_w})"
            );
        }
    }
}
