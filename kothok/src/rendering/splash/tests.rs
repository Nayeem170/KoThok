// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;

/// Every panel KoThok runs on. The composition has to hold on all of them --
/// that is the whole reason placement moved out of a baked PNG.
const FLEET: &[(&str, usize, usize)] = &[
    ("Touch/Mini", 600, 800),
    ("Glo/Aura/Nia", 758, 1024),
    ("Clara", 1072, 1448),
    ("Aura H2O", 1080, 1430),
    ("Libra", 1264, 1680),
    ("Elipsa", 1404, 1872),
    ("Forma/Sage", 1440, 1920),
];

fn overlaps(a: &Rect, b: &Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// The shared left edge *is* the composition -- it is what makes the splash and
/// the first page of a book line up.
#[test]
fn every_part_shares_the_reading_margin() {
    for &(name, w, h) in FLEET {
        let l = splash_layout(w, h);
        for (i, part) in l.parts.iter().enumerate() {
            assert_eq!(
                part.x, SPLASH_MARGIN as i32,
                "{name}: part {i} left edge drifted off the text margin"
            );
        }
        assert_eq!(l.status_x, SPLASH_MARGIN, "{name}: status line off margin");
    }
}

#[test]
fn nothing_leaves_the_screen() {
    for &(name, w, h) in FLEET {
        let l = splash_layout(w, h);
        for (i, p) in l.parts.iter().enumerate() {
            assert!(p.x >= 0 && p.y >= 0, "{name}: part {i} starts off-screen");
            assert!(
                p.x + p.w <= w as i32,
                "{name}: part {i} runs off the right edge ({} > {w})",
                p.x + p.w
            );
            assert!(
                p.y + p.h <= h as i32,
                "{name}: part {i} runs off the bottom ({} > {h})",
                p.y + p.h
            );
        }
        assert!(
            l.status_baseline < h,
            "{name}: status baseline below the screen"
        );
    }
}

/// Ink Bloom reveals one part per stage and presents only that part's rect. If
/// two parts overlapped, revealing one would re-drive pixels belonging to
/// another -- which is exactly the non-monotone update the design exists to
/// avoid.
#[test]
fn parts_never_overlap_so_the_reveal_stays_monotone() {
    for &(name, w, h) in FLEET {
        let l = splash_layout(w, h);
        for i in 0..SPLASH_STAGES {
            for j in (i + 1)..SPLASH_STAGES {
                assert!(
                    !overlaps(&l.parts[i], &l.parts[j]),
                    "{name}: parts {i} and {j} overlap; a stage would re-ink an earlier one"
                );
            }
        }
    }
}

#[test]
fn parts_stack_in_reading_order() {
    for &(name, w, h) in FLEET {
        let l = splash_layout(w, h);
        for i in 1..SPLASH_STAGES {
            assert!(
                l.parts[i].y > l.parts[i - 1].y,
                "{name}: part {i} does not sit below part {}",
                i - 1
            );
        }
        let last = l.parts[SPLASH_STAGES - 1];
        assert!(
            l.status_baseline as i32 > last.y + last.h,
            "{name}: status line collides with the lockup"
        );
    }
}

/// Display type is sized as a share of the screen, not in `dp` -- a hero needs
/// proportional presence, so the words must occupy the same fraction of a Nia
/// as of a Sage.
#[test]
fn display_type_keeps_its_share_of_the_screen() {
    let reference = {
        let l = splash_layout(1072, 1448);
        l.parts[2].w as f32 / 1072.0
    };
    for &(name, w, h) in FLEET {
        let l = splash_layout(w, h);
        let share = l.parts[2].w as f32 / w as f32;
        assert!(
            (share - reference).abs() < 0.01,
            "{name}: widest word takes {share:.3} of the width, expected ~{reference:.3}"
        );
    }
}

#[test]
fn stage_rect_is_none_outside_the_reveal() {
    let l = splash_layout(1072, 1448);
    assert!(l.stage_rect(0).is_none(), "stage 0 inks nothing");
    assert!(l.stage_rect(SPLASH_STAGES + 1).is_none());
    for s in 1..=SPLASH_STAGES {
        assert_eq!(l.stage_rect(s), Some(l.parts[s - 1]));
    }
}

/// The status line is the one non-monotone element, so its rect has to actually
/// cover the text it replaces.
#[test]
fn status_rect_covers_its_own_baseline() {
    for &(name, w, h) in FLEET {
        let l = splash_layout(w, h);
        let r = l.status_rect(w);
        assert!(
            r.y <= l.status_baseline as i32 && r.y + r.h >= l.status_baseline as i32,
            "{name}: status rect does not span its baseline"
        );
        assert!(r.x + r.w <= w as i32, "{name}: status rect exceeds width");
    }
}

#[test]
fn every_opening_stage_has_a_status_message() {
    assert_eq!(OPENING_STATUS.len(), SPLASH_STAGES);
    for (i, m) in OPENING_STATUS.iter().enumerate() {
        assert!(!m.is_empty(), "stage {i} has no status message");
    }
}

/// Render the finished splash to a PNG next to the package, so the marketing
/// site can ship the real composed screen instead of a hand-drawn stand-in.
///
/// The boot splash never lives in `st.buffer` (setup paints it on a private
/// buffer and presents directly), so a live header/corner capture comes back
/// black. This composes the same image offline. Gated behind `screenshot`
/// (the asset-generation feature) so it never runs in normal CI.
#[test]
#[cfg(feature = "screenshot")]
fn render_splash_png_for_site() {
    use image::codecs::png::PngEncoder;
    use image::{ExtendedColorType, ImageEncoder};
    use slint::platform::software_renderer::Rgb565Pixel;

    let w = crate::w();
    let h = crate::h();
    let mut buf = vec![Rgb565Pixel(0xFFFF); w * h];
    paint_kothok_splash(&mut buf);

    let mut rgb = vec![0u8; w * h * 3];
    for (i, px) in buf.iter().enumerate() {
        let v = px.0;
        let r = ((v >> 11) & 0x1f) as u8;
        let g = ((v >> 5) & 0x3f) as u8;
        let b = (v & 0x1f) as u8;
        rgb[i * 3] = (r << 3) | (r >> 2);
        rgb[i * 3 + 1] = (g << 2) | (g >> 4);
        rgb[i * 3 + 2] = (b << 3) | (b >> 2);
    }

    let dir = std::env::var("CARGO_MANIFEST_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    let out = dir.join("splash-render.png");
    let file = std::fs::File::create(&out).expect("create splash png");
    PngEncoder::new(file)
        .write_image(&rgb, w as u32, h as u32, ExtendedColorType::Rgb8)
        .expect("encode splash png");
    eprintln!("render_splash_png_for_site: wrote {}", out.display());
}
