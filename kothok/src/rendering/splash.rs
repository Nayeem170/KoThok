// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
//! The KoThok splash, composed at runtime from separate word images.
//!
//! Not a flattened screen. `splash-portrait.png` used to carry the whole
//! composition as one raster, which meant the logo's position was baked into
//! pixels (moving it required re-exporting the asset), the art was rescaled by
//! awkward non-integer factors on every panel that is not a Clara, and the
//! wordmark could not be animated because it was fused into the background.
//!
//! Instead the four parts ship separately and this module places them. The
//! layout is `splash_layout`, which is a function you can tune and test rather
//! than an image you have to re-author. See `ui/kothok-splash-screen.svg` for
//! the authoring source the parts are exported from.
//!
//! # Ink Bloom
//!
//! The words appear one at a time, one per startup stage. The reveal is
//! **monotone**: every stage only adds ink and none is ever taken back, so no
//! pixel changes state twice and flicker is impossible by construction rather
//! than merely reduced. That is also exactly the case `WAVE_DU` exists for.
//!
//! The one exception is the status line, whose text is replaced each stage. It
//! gets its own small rectangle and a clearing waveform. Flicker is area times
//! frequency, and one short line updated four times over a whole boot is
//! nothing next to the old spinner's 112x112 badge re-driven continuously.

use slint::platform::software_renderer::Rgb565Pixel;
use std::sync::OnceLock;

use kobo_core::rendering::density::dpf;
use kobo_core::rendering::loader::{box_downscale, Rect};

use crate::rendering::common::rgb565_as_bytes;
use crate::rendering::text_render::{self, DecodedRgba};

const WORD_READ: &[u8] = include_bytes!("../../ui/word-read.png");
const WORD_LISTEN: &[u8] = include_bytes!("../../ui/word-listen.png");
const WORD_ANYWHERE: &[u8] = include_bytes!("../../ui/word-anywhere.png");
const LOCKUP: &[u8] = include_bytes!("../../ui/lockup-kothok.png");

/// Design canvas the parts were composed on, and the ink positions measured
/// from that render. Everything else is derived, so the composition can be
/// retuned here rather than in an image editor.
const DESIGN_W: f32 = 1072.0;
const DESIGN_H: f32 = 1448.0;
/// Top of each word's ink on the design canvas. Evenly spaced 170 apart.
const DESIGN_WORD_TOP: [f32; 3] = [213.0, 383.0, 553.0];
const DESIGN_LOCKUP_TOP: f32 = 995.0;
const DESIGN_STATUS_BASELINE: f32 = 1290.0;

/// Ink size of each exported part on the design canvas, in draw order. Taken
/// from the tight crops in `ui/`, so layout can be computed without decoding a
/// single PNG - which is what lets it be tested across the whole fleet.
const DESIGN_PART_SIZE: [(f32, f32); SPLASH_STAGES] = [
    (321.0, 111.0), // Read
    (374.0, 111.0), // Listen
    (641.0, 141.0), // Anywhere (the 'y' descender makes this the tallest)
    (592.0, 70.0),  // with . mark . KoThok
];

/// Left margin, in raw pixels, deliberately **not** density-scaled.
///
/// This is `PAD_LEFT + GUTTER_W + GUTTER_PAD` from `rendering::layout` - the
/// exact x a book's text starts at. Those are raw constants on every panel, so
/// matching them raw is what makes the splash and the first page of a book
/// share a left edge. That shared edge is the whole composition.
pub const SPLASH_MARGIN: usize = crate::rendering::layout::PAD_LEFT
    + crate::rendering::layout::GUTTER_W
    + crate::rendering::layout::GUTTER_PAD;

/// Number of Ink Bloom stages: three words, then the signature lockup.
pub const SPLASH_STAGES: usize = 4;

/// Where every piece of the splash sits on this panel.
pub struct SplashLayout {
    /// Ink rect of each part, in draw order: Read, Listen, Anywhere, lockup.
    pub parts: [Rect; SPLASH_STAGES],
    pub status_x: usize,
    pub status_baseline: usize,
    pub status_px: f32,
}

impl SplashLayout {
    /// Rect newly inked by `stage` (1-based). Stage 0 inks nothing.
    pub fn stage_rect(&self, stage: usize) -> Option<Rect> {
        if stage == 0 || stage > SPLASH_STAGES {
            return None;
        }
        Some(self.parts[stage - 1])
    }

    /// Rect of the status line, which is rewritten rather than added to and so
    /// needs a clearing refresh.
    pub fn status_rect(&self, screen_w: usize) -> Rect {
        let h = (self.status_px * 1.6) as i32;
        Rect {
            x: self.status_x as i32,
            y: self.status_baseline as i32 - h,
            w: (screen_w - self.status_x * 2) as i32,
            h: h + (self.status_px * 0.5) as i32,
        }
    }
}

/// Compose the splash for a panel.
///
/// Display type scales with **width**: a hero's job is proportional presence,
/// not physical legibility, so the words keep the same share of the screen on
/// every device. The status line scales with **density** (`dpf`) instead,
/// because that one is read rather than seen.
///
/// Vertically the whole block is scaled by the same factor and centred, which
/// keeps the composition rigid instead of letting the gaps drift with aspect
/// ratio.
pub fn splash_layout(screen_w: usize, screen_h: usize) -> SplashLayout {
    let s = screen_w as f32 / DESIGN_W;
    // Centre the scaled design canvas in the real one. On panels slightly
    // taller than 1072x1448 this is a small negative offset, which is correct:
    // the content sits well inside the canvas either way.
    let v_off = (screen_h as f32 - DESIGN_H * s) / 2.0;
    let y_of = |design_y: f32| (design_y * s + v_off).max(0.0) as usize;

    let sizes = part_sizes(screen_w);
    let x = SPLASH_MARGIN;
    let mut parts = [Rect {
        x: 0,
        y: 0,
        w: 0,
        h: 0,
    }; SPLASH_STAGES];
    for i in 0..SPLASH_STAGES {
        let top = if i < 3 {
            DESIGN_WORD_TOP[i]
        } else {
            DESIGN_LOCKUP_TOP
        };
        let (pw, ph) = sizes[i];
        parts[i] = Rect {
            x: x as i32,
            y: y_of(top) as i32,
            w: pw as i32,
            h: ph as i32,
        };
    }

    SplashLayout {
        parts,
        status_x: x,
        status_baseline: y_of(DESIGN_STATUS_BASELINE),
        status_px: dpf(30.0),
    }
}

/// Decoded parts, scaled for this panel. Decoding four PNGs is far too slow to
/// repeat per frame, and the panel size never changes at runtime, so they are
/// decoded once on first use.
fn parts_cache(screen_w: usize) -> &'static [Option<DecodedRgba>; SPLASH_STAGES] {
    static CACHE: OnceLock<[Option<DecodedRgba>; SPLASH_STAGES]> = OnceLock::new();
    CACHE.get_or_init(|| {
        let raw = [WORD_READ, WORD_LISTEN, WORD_ANYWHERE, LOCKUP];
        let sizes = part_sizes(screen_w);
        std::array::from_fn(|i| {
            text_render::decode_image_rgba(raw[i], sizes[i].0.max(1), screen_w * 2)
        })
    })
}

/// Rendered size of each part, derived from the design sizes rather than from a
/// decode. Keeping layout independent of image decoding is what allows
/// `splash_layout` to be exercised for every panel in the fleet in a unit test.
fn part_sizes(screen_w: usize) -> [(usize, usize); SPLASH_STAGES] {
    let s = screen_w as f32 / DESIGN_W;
    std::array::from_fn(|i| {
        let (dw, dh) = DESIGN_PART_SIZE[i];
        ((dw * s).round() as usize, (dh * s).round() as usize)
    })
}

/// Paint the splash with the first `stage` parts inked.
///
/// Opening and closing are not separate modes: closing is simply every stage
/// inked with a status line describing the session instead of the boot. One
/// paint path means the exit screen cannot drift away from the entry screen.
///
/// `stage` counts parts, so `SPLASH_STAGES` is the finished screen. Callers
/// paint into a persistent buffer and present only `stage_rect(stage)`; the
/// full repaint here is cheap and keeps the buffer authoritative.
pub fn paint_splash(buffer: &mut [Rgb565Pixel], stage: usize, status: &str) {
    let w = crate::w();
    let h = crate::h();
    let layout = splash_layout(w, h);
    buffer.fill(Rgb565Pixel(0xFFFF));
    let buf = rgb565_as_bytes(buffer);
    let cache = parts_cache(w);

    for i in 0..stage.min(SPLASH_STAGES) {
        let (Some(img), r) = (cache[i].as_ref(), layout.parts[i]) else {
            continue;
        };
        text_render::blit_rgb565_image_alpha(
            buf,
            w,
            &img.rgba,
            img.width,
            img.height,
            r.x.max(0) as usize,
            r.y.max(0) as usize,
            w,
            h,
        );
    }

    if !status.is_empty() {
        text_render::blit_rgb565(
            buf,
            w,
            status,
            layout.status_px,
            layout.status_x,
            layout
                .status_baseline
                .saturating_sub(layout.status_px as usize),
            w,
            h,
        );
    }
}

/// The finished splash, every part inked and no status line. Used for the sleep
/// screen and as the cover-page backdrop, where there is no progress to report.
pub fn paint_kothok_splash(buffer: &mut [Rgb565Pixel]) {
    paint_splash(buffer, SPLASH_STAGES, "");
}

/// The splash as a stand-in **cover**, for books that ship without one.
///
/// Composed by painting the real splash and scaling the result down, rather
/// than by decoding a pre-flattened `splash-portrait.png`. That PNG was the
/// only remaining copy of the old pre-Ink-Bloom composition, so the library
/// grid and the audio disk kept showing a splash design the boot screen had
/// stopped using -- two sources of truth, and the stale one was the one nearly
/// every coverless book displayed. There is now exactly one splash, and it is
/// this module.
///
/// Deliberately uncached. A full-size paint plus a downscale is not cheap, but
/// it happens once per book per size and the callers already cache the result
/// (`covers::CoverCache` keyed by path and size, `LoopState::disk_cover` by
/// path). Caching the full-size RGB here would hold ~4.5MB resident on a device
/// that has little to spare, to save work that is already saved.
pub fn splash_cover(max_w: usize, max_h: usize) -> Option<text_render::DecodedImage> {
    let (w, h) = (crate::w(), crate::h());
    if w == 0 || h == 0 || max_w == 0 || max_h == 0 {
        return None;
    }
    let mut buffer = vec![Rgb565Pixel(0xFFFF); w * h];
    paint_kothok_splash(&mut buffer);

    let mut rgb = Vec::with_capacity(w * h * 3);
    for p in &buffer {
        let v = p.0;
        rgb.push((((v >> 11) & 0x1F) as u32 * 255 / 31) as u8);
        rgb.push((((v >> 5) & 0x3F) as u32 * 255 / 63) as u8);
        rgb.push(((v & 0x1F) as u32 * 255 / 31) as u8);
    }

    // Fit width first, then cap height, matching `text_render::decode_image` so
    // the fallback lands in a cover slot exactly like real artwork would.
    let mut nw = max_w;
    let mut nh = ((h as f32) * (max_w as f32 / w as f32)).round() as usize;
    if nh > max_h {
        nw = ((nw as f32) * (max_h as f32 / nh as f32)).round() as usize;
        nh = max_h;
    }
    if nw == 0 || nh == 0 {
        return None;
    }
    Some(text_render::DecodedImage {
        rgb: box_downscale(&rgb, w, h, nw, nh),
        width: nw,
        height: nh,
    })
}

/// Status text for each opening stage. One line per real startup milestone, so
/// a slow boot explains itself instead of just being slow.
pub const OPENING_STATUS: [&str; SPLASH_STAGES] = [
    "Waking up",
    "Loading fonts",
    "Scanning library",
    "Opening your book",
];

#[cfg(test)]
mod tests;
