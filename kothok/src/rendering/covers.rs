// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use kobo_core::EpubBook;

use crate::rendering::layout::word_wrap_bytes;
use crate::rendering::text_render;

use crate::rendering::common::rgb565_as_bytes;
use crate::rendering::draw::{measure_text, paint_placeholder_box};
use crate::rendering::splash::{paint_kothok_splash, splash_cover};

pub type CoverCache = std::collections::HashMap<
    (String, usize, usize),
    Option<crate::rendering::text_render::DecodedImage>,
>;

fn resolve_cover(
    cover_bytes: Option<&[u8]>,
    max_w: usize,
    max_h: usize,
) -> Option<crate::rendering::text_render::DecodedImage> {
    cover_bytes
        .and_then(|b| text_render::decode_image(b, max_w, max_h))
        .or_else(|| splash_cover(max_w, max_h))
}

pub(crate) fn paint_cover_cached(
    buf_bytes: &mut [u8],
    cache: &mut CoverCache,
    book_path: &str,
    cover_bytes: &Option<Vec<u8>>,
    x: usize,
    y: usize,
    max_w: usize,
    max_h: usize,
) {
    let decoded = cache
        .entry((book_path.to_string(), max_w, max_h))
        .or_insert_with(|| resolve_cover(cover_bytes.as_deref(), max_w, max_h));
    if let Some(img) = decoded {
        text_render::blit_rgb565_image(
            buf_bytes,
            crate::w(),
            &img.rgb,
            img.width,
            img.height,
            x,
            y,
            crate::w(),
            crate::h(),
        );
    } else {
        paint_placeholder_box(buf_bytes, crate::w(), crate::h(), x, y, max_w, max_h);
    }
}

pub fn render_book_cover_scaled(book_path: &str, buffer: &mut [Rgb565Pixel]) -> bool {
    let raw = match EpubBook::cover_bytes(book_path) {
        Some(b) => b,
        None => {
            paint_kothok_splash(buffer);
            return false;
        }
    };
    let decoded = match text_render::decode_image(&raw, crate::w(), crate::h() * 2) {
        Some(d) => d,
        None => {
            paint_kothok_splash(buffer);
            return false;
        }
    };
    let (rgb, iw, ih) = (decoded.rgb, decoded.width, decoded.height);
    buffer.fill(Rgb565Pixel(0xFFFF));
    let buf_bytes = rgb565_as_bytes(buffer);
    let ox = (crate::w() - iw) / 2;
    let oy = (((crate::h() as i64) - (ih as i64)) / 2).max(0) as usize;
    text_render::blit_rgb565_image(
        buf_bytes,
        crate::w(),
        &rgb,
        iw,
        ih,
        ox,
        oy,
        crate::w(),
        crate::h(),
    );
    true
}

pub fn text_image(text: &str, px: f32, max_w: usize, max_lines: usize) -> (slint::Image, u32) {
    let lines = word_wrap_bytes(text, max_w, px);
    let lh = text_render::line_height(px);
    let n = lines.len().min(max_lines).max(1);
    let h = (lh * n).max(1);
    let used_lines: Vec<_> = lines.iter().take(n).collect();
    let img_w = used_lines
        .iter()
        .map(|l| measure_text(&l.text, px).max(1))
        .max()
        .unwrap_or(max_w)
        .min(max_w);
    let mut buf = vec![Rgb565Pixel(0xFFFF); img_w * h];
    let buf_bytes = rgb565_as_bytes(&mut buf);
    let mut cy = 0usize;
    for line in &used_lines {
        text_render::blit_rgb565(buf_bytes, img_w, &line.text, px, 0, cy, img_w, h);
        cy += lh;
    }
    let mut rgb: Vec<u8> = Vec::with_capacity(img_w * h * 3);
    for p in &buf {
        let v = p.0;
        rgb.push((((v >> 11) & 0x1f) << 3) as u8);
        rgb.push((((v >> 5) & 0x3f) << 2) as u8);
        rgb.push(((v & 0x1f) << 3) as u8);
    }
    let pb = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
        &rgb,
        img_w as u32,
        h as u32,
    );
    (slint::Image::from_rgb8(pb), h as u32)
}

pub fn cover_image(cover_bytes: Option<&[u8]>, max_w: usize, max_h: usize) -> slint::Image {
    let Some(img) = resolve_cover(cover_bytes, max_w, max_h) else {
        return slint::Image::default();
    };
    let pb = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
        &img.rgb,
        img.width as u32,
        img.height as u32,
    );
    slint::Image::from_rgb8(pb)
}

/// Decoded cover art for the audio-mode disk, sized for a `size`-px disk.
///
/// Falls back to the KoThok splash art when a book has no cover of its own, so
/// the disk centre is never empty. The caller crops it to a circle, and the crop
/// applies to whichever image comes back.
pub fn disk_cover(
    book_path: &str,
    size: usize,
) -> Option<crate::rendering::text_render::DecodedImage> {
    if book_path.is_empty() || size == 0 {
        return None;
    }
    let raw = EpubBook::cover_bytes(book_path);
    // `max_h` is generous: the disk samples the image's short edge, so the art
    // only has to be at least `size` across that edge to fill the circle.
    resolve_cover(raw.as_deref(), size, size * 4)
}
