// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
//! Capture-only: writes the presented frame to disk as a PNG.
//!
//! Exists to produce real UI stills for the marketing site, so the 3D device on
//! the web page shows the shipping UI instead of a hand-drawn replica that
//! drifts every redesign. Gated behind the `screenshot` feature; never shipped.

use std::fs::{self, File};
use std::io::Write;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use image::codecs::png::PngEncoder;
use image::{ExtendedColorType, ImageEncoder};

use crate::data::config::SHOTS_DIR;

/// Convert the live RGB565 framebuffer to RGB8.
///
/// Inverse of `kobo_core::device::fb::dump_ppm`. The low bits are replicated
/// into the gap left by widening each channel, so full-scale input lands on
/// 0xFF rather than 0xF8 - without it the paper-white background renders as a
/// dull grey once it is up on the site.
fn rgb565_to_rgb8(buf: &[u8], w: usize, h: usize) -> Vec<u8> {
    let mut rgb = vec![0u8; w * h * 3];
    for i in 0..w * h {
        let off = i * 2;
        let v = (buf[off] as u16) | ((buf[off + 1] as u16) << 8);
        let r = ((v >> 11) & 0x1f) as u8;
        let g = ((v >> 5) & 0x3f) as u8;
        let b = (v & 0x1f) as u8;
        rgb[i * 3] = (r << 3) | (r >> 2);
        rgb[i * 3 + 1] = (g << 2) | (g >> 4);
        rgb[i * 3 + 2] = (b << 3) | (b >> 2);
    }
    rgb
}

/// Write `buf` to `SHOTS_DIR` as a PNG, returning the path written.
///
/// Named by wall-clock seconds so repeated captures never overwrite and the
/// capture order survives the trip off the device.
pub fn capture(buf: &[u8], w: usize, h: usize) -> std::io::Result<PathBuf> {
    let expected = w * h * 2;
    if buf.len() < expected {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("framebuffer is {} bytes, need {}", buf.len(), expected),
        ));
    }

    fs::create_dir_all(SHOTS_DIR)?;

    let rgb = rgb565_to_rgb8(buf, w, h);
    let mut png = Vec::new();
    PngEncoder::new(&mut png)
        .write_image(&rgb, w as u32, h as u32, ExtendedColorType::Rgb8)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let path = PathBuf::from(SHOTS_DIR).join(format!("shot-{stamp}.png"));

    let mut f = File::create(&path)?;
    f.write_all(&png)?;
    // Onboard writes are lost on reboot unless they are flushed explicitly, and
    // run.sh reboots the device on exit - without this the captures vanish.
    f.sync_all()?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn white_stays_fully_white() {
        // 0xFFFF must widen to #FFFFFF, not #F8FCF8.
        let rgb = rgb565_to_rgb8(&[0xFF, 0xFF], 1, 1);
        assert_eq!(rgb, vec![0xFF, 0xFF, 0xFF]);
    }

    #[test]
    fn black_stays_black() {
        let rgb = rgb565_to_rgb8(&[0x00, 0x00], 1, 1);
        assert_eq!(rgb, vec![0x00, 0x00, 0x00]);
    }

    #[test]
    fn channels_are_not_swapped() {
        // 0xF800 is pure red; a byte-order slip would surface it as blue.
        let rgb = rgb565_to_rgb8(&[0x00, 0xF8], 1, 1);
        assert_eq!(rgb, vec![0xFF, 0x00, 0x00]);
        // 0x001F is pure blue.
        let rgb = rgb565_to_rgb8(&[0x1F, 0x00], 1, 1);
        assert_eq!(rgb, vec![0x00, 0x00, 0xFF]);
    }

    #[test]
    fn capture_rejects_a_short_buffer() {
        let err = capture(&[0u8; 8], 64, 64).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }
}
