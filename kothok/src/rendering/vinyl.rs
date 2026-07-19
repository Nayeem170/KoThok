// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use kobo_core::rendering::text_render;

const RED: [u8; 3] = [0xF4, 0x2A, 0x41];
const GREEN: [u8; 3] = [0x00, 0x6A, 0x4E];
const BLACK: [u8; 3] = [0x11, 0x11, 0x11];
const WHITE: [u8; 3] = [0xFF, 0xFF, 0xFF];
const SPINDLE: [u8; 3] = [0x00, 0x00, 0x00];

const COVER_TILT_DEG: f32 = -12.0;

/// Mock reference geometry: a 428-wide layout with a 230-unit disk. Panels range
/// from 758px (Nia) to 1440px (Sage) wide, so sizes are derived from the device's
/// reported width rather than fixed.
const MOCK_W: usize = 428;
const MOCK_DISK_UNITS: usize = 230;
/// Audio header/footer bands, in px. Must match `audio_player.slint`.
pub const AUDIO_HEADER_H: usize = 110;
pub const AUDIO_FOOTER_H: usize = 371;

/// Side of the rendered disk for a given panel width.
pub fn disk_px(screen_w: usize) -> usize {
    (screen_w * MOCK_DISK_UNITS / MOCK_W).max(64)
}

/// Disk origin y: centred in the body band. Mirrors `audio_player.slint`'s
/// `disk-y` binding -- the two must agree or the A2 rect misses the marker.
pub fn disk_y(screen_w: usize, screen_h: usize) -> usize {
    let band = screen_h
        .saturating_sub(AUDIO_HEADER_H)
        .saturating_sub(AUDIO_FOOTER_H);
    AUDIO_HEADER_H + band.saturating_sub(disk_px(screen_w)) / 2
}

// ---------------------------------------------------------------------------
// Geometry
//
// Nothing in the middle of the disk moves. That is the whole design.
//
// The disk used to spin its label, which forced the A2 rectangle to cover the
// entire label -- and A2 re-drives every pixel it covers whether or not that
// pixel changed. ~100k pixels re-driven four times a second is what read as
// flickering, and it also banned artwork from the centre, because A2 is 2-level
// and would have thresholded a cover into harsh noise.
//
// Rotating the centre is not recoverable by tuning. In colour it needs a
// 16-level waveform, which does not fully clear, so repeated turns accumulate
// ghosting until a GC16 full flash is forced. Dithered to 1-bit it fits A2, but
// then ~85k pixels of high-frequency stipple are re-driven every step and the
// whole region visibly churns. Both were tried and both look like the bug this
// design exists to remove. So the centre is still, and the motion is carried by
// a few small grains orbiting in the black vinyl annulus. That gives one
// governing rule:
//
//     EVERY A2 RECTANGLE MUST STAY INSIDE THE ANNULUS.
//
// which buys three things at once: the cover is never re-driven (so it can be
// full-colour GL16 art), the red/green ring is never touched (so it keeps its
// colour), and each rectangle's background is uniform black (so the grain is
// the only thing in it that ever changes state). `grain_boxes_stay_in_annulus`
// in the tests asserts it directly, for every panel, at every angle.
// ---------------------------------------------------------------------------

/// Black vinyl body, out to the mock's 90 units.
const VINYL_OUTER_UNITS: f32 = 90.0;
/// Cover art radius. Shrunk from the mock's 62 to widen the annulus: that extra
/// 4 units is the clearance the invariant above needs on both sides.
const COVER_UNITS: f32 = 58.0;
/// Slack around a grain's rect, so anti-aliased edges are inside it.
const MARKER_PAD_UNITS: f32 = 1.5;

/// The grains that ride the annulus, as `(orbit radius, dot radius, phase)`.
///
/// Five rather than one because a single orbiting dot does not read as
/// rotation. One moving feature gives the eye one displacement vector, which
/// fits a dot sliding along a track just as well as it fits a disk carrying it,
/// so the percept never resolves. Two or more features sharing an angular
/// velocity cannot be explained by anything except rotation.
///
/// All three columns are deliberately unequal:
///
/// * **Phase** -- evenly spaced grains stepping `MARKER_STEP_DEG` would return
///   to an identical picture every `360 / (spacing / step)` ticks. That is the
///   wagon-wheel effect, and it can read as stationary or as running backwards.
///   Irregular phases have no rotational symmetry to alias into.
/// * **Radius** -- an outer grain sweeps further per tick than an inner one.
///   That difference is what a rigid body does and what a set of independent
///   dots does not, so it is the second cue that the disk is one object.
/// * **Dot size** -- makes the grains individually identifiable, so the eye can
///   track a particular one around rather than seeing an anonymous swarm.
///
/// The band these can sit in is narrower than it looks. A refresh rect is
/// axis-aligned, so its nearest corner is `sqrt(2)` times its half-extent from
/// the disk centre, not one half-extent -- and the half-extent is the dot, plus
/// `MARKER_PAD_UNITS`, plus half the arc the grain sweeps in one step. Against a
/// 58..90 annulus that leaves roughly 69..79 to orbit in, which is why the radii
/// are closer together than the eye would suggest.
///
/// `grains_fit_the_annulus` asserts the corner bound directly, so a retune here
/// cannot silently break the invariant above.
const GRAINS: [(f32, f32, f32); GRAIN_COUNT] = [
    (71.0, 3.0, 0.0),
    (73.0, 4.0, 58.0),
    (74.5, 2.5, 137.0),
    (76.5, 3.0, 212.0),
    (77.5, 2.5, 295.0),
];

/// How many grains orbit. Also the number of A2 rectangles presented per tick.
pub const GRAIN_COUNT: usize = 5;

/// Degrees the grains advance per tick. Must match `COVER_ROTATION_STEP` in
/// `loop_run::callbacks` -- the rects are sized to span exactly one step.
pub const MARKER_STEP_DEG: f32 = 5.0;

fn unit(screen_w: usize) -> f32 {
    disk_px(screen_w) as f32 / 230.0
}

/// Centre of the disk in screen space.
fn disk_center(screen_w: usize, screen_h: usize) -> (f32, f32) {
    let size = disk_px(screen_w);
    let cx = screen_w.saturating_sub(size) / 2 + size / 2;
    let cy = disk_y(screen_w, screen_h) + size / 2;
    (cx as f32, cy as f32)
}

/// Centre of grain `i` at `deg`, in screen space. 0 degrees is the top of the
/// disk, matching how the progress ring is drawn.
fn grain_center(screen_w: usize, screen_h: usize, i: usize, deg: f32) -> (f32, f32) {
    let (cx, cy) = disk_center(screen_w, screen_h);
    let u = unit(screen_w);
    let (orbit_r, _, phase) = GRAINS[i];
    let a = (deg + phase - 90.0).to_radians();
    (cx + a.cos() * orbit_r * u, cy + a.sin() * orbit_r * u)
}

/// The rectangle an A2 update must cover to move grain `i` from `prev_deg` to
/// `now_deg`: it has to span both positions, so the old dot is erased in the
/// same pass that draws the new one.
pub fn grain_box(
    screen_w: usize,
    screen_h: usize,
    i: usize,
    prev_deg: f32,
    now_deg: f32,
) -> (usize, usize, usize, usize) {
    let u = unit(screen_w);
    let pad = (GRAINS[i].1 + MARKER_PAD_UNITS) * u;
    let (ax, ay) = grain_center(screen_w, screen_h, i, prev_deg);
    let (bx, by) = grain_center(screen_w, screen_h, i, now_deg);
    let x0 = ax.min(bx) - pad;
    let y0 = ay.min(by) - pad;
    let x1 = ax.max(bx) + pad;
    let y1 = ay.max(by) + pad;
    let x0 = x0.max(0.0) as usize;
    let y0 = y0.max(0.0) as usize;
    let w = ((x1.max(0.0) as usize).saturating_sub(x0)).max(1);
    let h = ((y1.max(0.0) as usize).saturating_sub(y0)).max(1);
    (
        x0,
        y0,
        w.min(screen_w.saturating_sub(x0)),
        h.min(screen_h.saturating_sub(y0)),
    )
}

/// Every grain's rect for one step.
///
/// Disjoint rather than one enclosing rect on purpose: the grains are spread
/// around the orbit, so a rect spanning all five would cover the entire disk --
/// cover and colour ring included -- and hand it to a 2-level waveform. Five
/// small A2 regions cost a fraction of that one big one.
pub fn grain_boxes(
    screen_w: usize,
    screen_h: usize,
    prev_deg: f32,
    now_deg: f32,
) -> [(usize, usize, usize, usize); GRAIN_COUNT] {
    std::array::from_fn(|i| grain_box(screen_w, screen_h, i, prev_deg, now_deg))
}

/// Region for the settle pass that clears A2's ghosting once the marker stops.
///
/// Covers the whole disk, ring included. That is safe precisely because the
/// settle runs on `GC16` rather than A2: the 16-level waveform reproduces the
/// ring's colour, so unlike the grain rects this one has no reason to stay
/// inside the annulus. Spanning everything also clears ghosting the grains may
/// have left anywhere on their orbits.
pub fn settle_box(screen_w: usize, screen_h: usize) -> (usize, usize, usize, usize) {
    let (cx, cy) = disk_center(screen_w, screen_h);
    let r = (disk_px(screen_w) as f32) / 2.0;
    let x0 = (cx - r).max(0.0) as usize;
    let y0 = (cy - r).max(0.0) as usize;
    let side = (r * 2.0) as usize;
    (
        x0,
        y0,
        side.min(screen_w.saturating_sub(x0)),
        side.min(screen_h.saturating_sub(y0)),
    )
}

/// Pre-render the full vinyl disk: progress ring, black body, cover art, grains.
///
/// Rendered in Rust because Slint's software renderer segfaults on the
/// `clip: true` + `border-radius` circular crop and cannot stroke a two-colour
/// progress arc. `book_frac` (0..1) fills the ring green clockwise from the top,
/// red for the remainder.
///
/// `cover` is the book's artwork, already decoded and scaled to the cover
/// circle's diameter -- `covers::disk_cover` substitutes the KoThok splash art
/// for books that ship without one, so in practice this is `None` only when
/// there is no book at all, and then the title and author are set in the middle
/// instead. Whichever image it is gets the same `COVER_TILT_DEG` tilt: the
/// centre is still, but a tilted still reads as a record that has been set down
/// rather than as a diagram. Because it never rotates it is never inside an A2
/// rectangle, so it keeps its full tonal range.
pub fn render_vinyl_disk(
    cover: Option<&text_render::DecodedImage>,
    title: &str,
    author: &str,
    book_frac: f32,
    marker_deg: f32,
    size: usize,
) -> slint::Image {
    let size = size.max(16);
    let mut buf = vec![0u8; size * size * 3];
    for px in buf.chunks_exact_mut(3) {
        px.copy_from_slice(&WHITE);
    }

    let sz = size as f32;
    let center = sz / 2.0;
    let ring_outer_r = sz * (110.0 / 230.0);
    let ring_inner_r = sz * (96.0 / 230.0);
    let vinyl_outer_r = sz * (VINYL_OUTER_UNITS / 230.0);
    let cover_r = (sz * (COVER_UNITS / 230.0)).max(1.0);
    let spindle_r = sz * (6.0 / 230.0);

    let frac = book_frac.clamp(0.0, 1.0);
    let arc_deg = frac * 360.0;

    // Grain centres in disk-local coordinates, resolved once rather than per
    // pixel: the inner loop runs `size * size` times and this does not.
    let grains: [(f32, f32, f32); GRAIN_COUNT] = std::array::from_fn(|i| {
        let (orbit_r, dot_r, phase) = GRAINS[i];
        let a = (marker_deg + phase - 90.0).to_radians();
        let r = sz * (orbit_r / 230.0);
        (a.cos() * r, a.sin() * r, sz * (dot_r / 230.0))
    });

    let label = cover
        .is_none()
        .then(|| render_label_rgb565(title, author, (cover_r * 2.0) as usize));
    let (sin_t, cos_t) = COVER_TILT_DEG.to_radians().sin_cos();

    for py in 0..size {
        for px in 0..size {
            let dx = px as f32 - center + 0.5;
            let dy = py as f32 - center + 0.5;
            let dist = (dx * dx + dy * dy).sqrt();
            let off = (py * size + px) * 3;

            if dist <= spindle_r {
                buf[off..off + 3].copy_from_slice(&SPINDLE);
            } else if dist <= cover_r {
                // Both the artwork and the fallback label are tilted by the same
                // angle, so a book with a cover and a book without still sit at
                // the same jaunty angle on the platter.
                let rx = dx * cos_t - dy * sin_t;
                let ry = dx * sin_t + dy * cos_t;
                let color = match (cover, &label) {
                    (Some(img), _) => sample_cover(img, rx, ry, cover_r),
                    (None, Some(l)) => sample_label(l, (cover_r * 2.0) as usize, rx, ry, cover_r),
                    _ => WHITE,
                };
                buf[off..off + 3].copy_from_slice(&color);
            } else if dist <= vinyl_outer_r {
                // Black vinyl. The grains ride here, which is why the annulus is
                // uniform: each A2 box lands on flat black, so its dot is the
                // only thing in it that ever changes state.
                let lit = grains.iter().any(|&(gx, gy, gr)| {
                    let ddx = dx - gx;
                    let ddy = dy - gy;
                    ddx * ddx + ddy * ddy <= gr * gr
                });
                buf[off..off + 3].copy_from_slice(if lit { &WHITE } else { &BLACK });
            } else if dist <= ring_inner_r {
                // gap: stays white
            } else if dist <= ring_outer_r {
                let angle = dy.atan2(dx).to_degrees();
                let prog = ((angle + 90.0) + 360.0) % 360.0;
                let color = if prog <= arc_deg { GREEN } else { RED };
                buf[off..off + 3].copy_from_slice(&color);
            }
        }
    }

    let pb = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
        &buf,
        size as u32,
        size as u32,
    );
    slint::Image::from_rgb8(pb)
}

/// Sample the cover, circular-cropped and centre-cropped to a square.
///
/// `dx`/`dy` arrive already rotated by the caller. Rotating the sample
/// coordinates rather than the image is what keeps the tilt free: the circle
/// being sampled is inscribed in the square the art was fitted to, and a circle
/// is invariant under rotation, so no angle can ever sample outside the source.
fn sample_cover(img: &text_render::DecodedImage, dx: f32, dy: f32, cover_r: f32) -> [u8; 3] {
    if img.width == 0 || img.height == 0 {
        return WHITE;
    }
    // Map the cover circle onto the image's short edge so the art fills the
    // disk rather than letterboxing inside it.
    let short = img.width.min(img.height) as f32;
    let scale = short / (cover_r * 2.0);
    let sx = ((dx + cover_r) * scale) as isize + ((img.width as f32 - short) / 2.0) as isize;
    let sy = ((dy + cover_r) * scale) as isize + ((img.height as f32 - short) / 2.0) as isize;
    if sx < 0 || sy < 0 || sx as usize >= img.width || sy as usize >= img.height {
        return WHITE;
    }
    let off = (sy as usize * img.width + sx as usize) * 3;
    if off + 2 >= img.rgb.len() {
        return WHITE;
    }
    [img.rgb[off], img.rgb[off + 1], img.rgb[off + 2]]
}

/// Fallback centre when there is no book at all: the title over the author.
///
/// Set square here and tilted once by the caller, along with the artwork. A
/// fixed tilt is still readable in a way the old continuous rotation was not.
fn render_label_rgb565(title: &str, author: &str, d: usize) -> Vec<u8> {
    let mut buf = vec![0xFFu8; d * d * 2];
    if d == 0 {
        return buf;
    }
    let max_w = ((d as f32) * 0.78) as usize;
    let min_px = (d as f32) * 0.06;
    let mut px = (d as f32) * 0.13;
    let mut lines = kobo_core::rendering::layout::word_wrap_bytes(title, max_w, px);
    while lines.len() > 3 && px > min_px {
        px *= 0.85;
        lines = kobo_core::rendering::layout::word_wrap_bytes(title, max_w, px);
    }
    lines.truncate(3);

    let lh = text_render::line_height(px);
    let apx = px * 0.62;
    let alh = if author.is_empty() {
        0
    } else {
        text_render::line_height(apx)
    };
    let gap = if author.is_empty() {
        0
    } else {
        (px * 0.35) as usize
    };
    let total = lines.len() * lh + gap + alh;
    let mut y = d.saturating_sub(total) / 2;
    for l in &lines {
        let tw = text_render::word_width(&l.text, px) as usize;
        let x = d.saturating_sub(tw) / 2;
        text_render::blit_rgb565(&mut buf, d, &l.text, px, x, y, d, d);
        y += lh;
    }
    if !author.is_empty() {
        y += gap;
        let aw = text_render::word_width(author, apx) as usize;
        let ax = d.saturating_sub(aw) / 2;
        text_render::blit_rgb565(&mut buf, d, author, apx, ax, y, d, d);
    }
    buf
}

/// As `sample_cover`, for the no-book fallback label. `dx`/`dy` are pre-rotated
/// by the caller, so both centre paths share one tilt.
fn sample_label(label: &[u8], d: usize, dx: f32, dy: f32, cover_r: f32) -> [u8; 3] {
    let sx = (dx + cover_r) as isize;
    let sy = (dy + cover_r) as isize;
    if sx < 0 || sy < 0 || sx as usize >= d || sy as usize >= d {
        return WHITE;
    }
    let off = (sy as usize * d + sx as usize) * 2;
    if off + 1 >= label.len() {
        return WHITE;
    }
    let v = (label[off] as u16) | ((label[off + 1] as u16) << 8);
    let r = (((v >> 11) & 0x1F) as u32 * 255 / 31) as u8;
    let g = (((v >> 5) & 0x3F) as u32 * 255 / 63) as u8;
    let b = ((v & 0x1F) as u32 * 255 / 31) as u8;
    [r, g, b]
}

#[cfg(test)]
mod tests;
