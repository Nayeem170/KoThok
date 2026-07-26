// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;

const FLEET: &[(&str, usize, usize)] = &[
    ("Touch/Mini", 600, 800),
    ("Glo/Aura/Nia", 758, 1024),
    ("Clara", 1072, 1448),
    ("Aura H2O", 1080, 1430),
    ("Libra", 1264, 1680),
    ("Elipsa", 1404, 1872),
    ("Forma/Sage", 1440, 1920),
];

/// Nearest and farthest distance from the disk centre to any point of a rect.
fn rect_range(cx: f32, cy: f32, r: (usize, usize, usize, usize)) -> (f32, f32) {
    let (x, y, w, h) = r;
    let x0 = x as f32 - cx;
    let y0 = y as f32 - cy;
    let x1 = x0 + w as f32;
    let y1 = y0 + h as f32;
    let dx = x0.max(-x1).max(0.0);
    let dy = y0.max(-y1).max(0.0);
    let near = (dx * dx + dy * dy).sqrt();
    let fx = x0.abs().max(x1.abs());
    let fy = y0.abs().max(y1.abs());
    let far = (fx * fx + fy * fy).sqrt();
    (near, far)
}

/// The invariant the whole design rests on.
///
/// Every grain's A2 rectangle must stay inside the black annulus at every
/// angle, on every panel. Outside it on the inside, and A2 would threshold the
/// colour cover into 1-bit noise; outside it on the outside, and A2 would strip
/// the colour from the progress ring. Both failures look like the bug this
/// design was built to remove, so the geometry gets asserted rather than
/// eyeballed.
#[test]
fn grain_boxes_stay_in_annulus() {
    for &(name, w, h) in FLEET {
        let (cx, cy) = disk_center(w, h);
        let u = unit(w);
        let cover_r = COVER_UNITS * u;
        let vinyl_r = VINYL_OUTER_UNITS * u;
        // Whole revolution in 1-degree steps: an axis-aligned square reaches
        // furthest at 45 degrees, so sampling only the step angles would miss
        // the worst case.
        for deg in 0..360 {
            let now = deg as f32;
            let prev = now - MARKER_STEP_DEG;
            for (i, r) in grain_boxes(w, h, prev, now).into_iter().enumerate() {
                let (near, far) = rect_range(cx, cy, r);
                assert!(
                    near >= cover_r,
                    "{name} grain {i} @{deg}deg: rect reaches {near:.1}px from centre, inside \
                     the {cover_r:.1}px cover -- A2 would threshold the artwork"
                );
                assert!(
                    far <= vinyl_r,
                    "{name} grain {i} @{deg}deg: rect reaches {far:.1}px from centre, past the \
                     {vinyl_r:.1}px vinyl edge -- A2 would strip the ring's colour"
                );
            }
        }
    }
}

/// Each rect has to contain both of its grain's positions, or the previous dot
/// is left behind as a trail.
#[test]
fn grain_boxes_cover_both_positions() {
    for &(name, w, h) in FLEET {
        let u = unit(w);
        for deg in (0..360).step_by(7) {
            let now = deg as f32;
            let prev = now - MARKER_STEP_DEG;
            let boxes = grain_boxes(w, h, prev, now);
            for (i, &(x, y, bw, bh)) in boxes.iter().enumerate() {
                let dot = GRAINS[i].1 * u;
                for angle in [prev, now] {
                    let (mx, my) = grain_center(w, h, i, angle);
                    assert!(
                        mx - dot >= x as f32 - 1.0
                            && mx + dot <= (x + bw) as f32 + 1.0
                            && my - dot >= y as f32 - 1.0
                            && my + dot <= (y + bh) as f32 + 1.0,
                        "{name} grain {i} @{deg}deg: dot at {angle} falls outside its own \
                         refresh rect"
                    );
                }
            }
        }
    }
}

/// The point of the redesign: five small rects still cost far less than the one
/// whole-label rect they replaced.
#[test]
fn grain_boxes_are_far_smaller_than_the_old_label_box() {
    let (w, h) = (1072, 1448);
    let grain_px: usize = grain_boxes(w, h, 0.0, MARKER_STEP_DEG)
        .iter()
        .map(|&(_, _, bw, bh)| bw * bh)
        .sum();
    // The old build re-drove a square just past the 62-unit label.
    let old_half = (62.0 * unit(w) + 3.0) as usize;
    let old_px = (old_half * 2) * (old_half * 2);
    assert!(
        old_px / grain_px >= 4,
        "expected a large reduction in re-driven pixels, got {old_px} -> {grain_px}"
    );
}

#[test]
fn grain_boxes_never_leave_the_screen() {
    for &(name, w, h) in FLEET {
        for deg in (0..360).step_by(11) {
            for (i, (x, y, bw, bh)) in grain_boxes(w, h, deg as f32 - MARKER_STEP_DEG, deg as f32)
                .into_iter()
                .enumerate()
            {
                assert!(x + bw <= w, "{name} grain {i}: rect past right edge");
                assert!(y + bh <= h, "{name} grain {i}: rect past bottom edge");
                assert!(bw > 0 && bh > 0, "{name} grain {i}: degenerate rect");
            }
        }
    }
}

/// The settle pass has to cover everywhere any grain can have been, or A2
/// ghosting is left behind on the part of an orbit it missed.
///
/// It deliberately does *not* have to avoid the ring: the settle runs on GC16,
/// which reproduces colour. Only the A2 rects are constrained.
#[test]
fn settle_box_contains_every_grain_box() {
    for &(name, w, h) in FLEET {
        let (ax, ay, aw, ah) = settle_box(w, h);
        for deg in (0..360).step_by(5) {
            for (i, (x, y, bw, bh)) in grain_boxes(w, h, deg as f32 - MARKER_STEP_DEG, deg as f32)
                .into_iter()
                .enumerate()
            {
                assert!(
                    x >= ax && y >= ay && x + bw <= ax + aw && y + bh <= ay + ah,
                    "{name} grain {i} @{deg}deg: rect escapes the settle rect"
                );
            }
        }
    }
}

/// Every grain must fit between the cover and the vinyl edge, or there is no
/// clearance for it to orbit in at all. This is the unit-space version of
/// `grain_boxes_stay_in_annulus`, and it is the one that fails first and most
/// legibly when the `GRAINS` table is retuned.
///
/// The bound is the rect's *corner*, not its edge. A refresh rect is
/// axis-aligned, so at 45 degrees around the orbit its nearest corner is
/// `sqrt(2)` half-extents from the disk centre rather than one. Checking the
/// radial distance instead is what let a grain reach 80.6px into an 81.2px
/// cover on the smallest panel while this test still passed.
#[test]
fn grains_fit_the_annulus() {
    for (i, &(orbit_r, dot_r, _)) in GRAINS.iter().enumerate() {
        // Half-extent: the dot, its anti-aliasing slack, and half the arc the
        // grain sweeps in one step -- the rect spans both ends of that step.
        let half = dot_r + MARKER_PAD_UNITS + orbit_r * MARKER_STEP_DEG.to_radians() / 2.0;
        let reach = half * std::f32::consts::SQRT_2;
        assert!(
            orbit_r - reach > COVER_UNITS,
            "grain {i}: rect corner reaches {:.1}, inside the {COVER_UNITS} cover",
            orbit_r - reach
        );
        assert!(
            orbit_r + reach < VINYL_OUTER_UNITS,
            "grain {i}: rect corner reaches {:.1}, past the {VINYL_OUTER_UNITS} vinyl edge",
            orbit_r + reach
        );
    }
}

/// Rotation is only legible if the grains are irregular.
///
/// Equal phases alias -- the field returns to an identical picture on a short
/// cycle and can read as stationary or as running backwards. Equal radii remove
/// the differential-speed cue that says the grains belong to one rigid body.
/// Both are easy to reintroduce by "tidying" the table, so both are asserted.
#[test]
fn grains_are_irregular() {
    assert!(
        GRAIN_COUNT >= 3,
        "fewer than three grains cannot read as a body"
    );
    for a in 0..GRAIN_COUNT {
        for b in (a + 1)..GRAIN_COUNT {
            assert!(
                (GRAINS[a].0 - GRAINS[b].0).abs() > 0.5,
                "grains {a} and {b} share an orbit radius"
            );
            assert!(
                (GRAINS[a].2 - GRAINS[b].2).abs() > 0.5,
                "grains {a} and {b} share a phase"
            );
        }
    }
    // No pair of gaps around the ring is equal, so there is no rotation of the
    // field that maps it onto itself.
    let mut phases: Vec<f32> = GRAINS.iter().map(|g| g.2).collect();
    phases.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let gaps: Vec<f32> = (0..GRAIN_COUNT)
        .map(|i| (phases[(i + 1) % GRAIN_COUNT] - phases[i]).rem_euclid(360.0))
        .collect();
    for a in 0..gaps.len() {
        for b in (a + 1)..gaps.len() {
            assert!(
                (gaps[a] - gaps[b]).abs() > 1.0,
                "phase gaps {a} and {b} are equal -- the field can alias"
            );
        }
    }
}
