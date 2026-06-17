use slint::platform::software_renderer::Rgb565Pixel;

use crate::rendering::text_render;

use crate::rendering::common::{rgb565_as_bytes, KOTHOK_LOGO};
use crate::rendering::draw::measure_text;

pub fn paint_kothok_splash(buffer: &mut [Rgb565Pixel]) {
    buffer.fill(Rgb565Pixel(0xFFFF));
    let buf_bytes = rgb565_as_bytes(buffer);
    if let Some(img) = text_render::decode_image(KOTHOK_LOGO, crate::w(), crate::h()) {
        let (rgb, iw, ih) = (img.rgb, img.width, img.height);
        let ox = (crate::w() - iw) / 2;
        let oy = (crate::h() - ih) / 2;
        let mut work: Vec<[f32; 3]> = rgb
            .chunks_exact(3)
            .map(|c| [c[0] as f32, c[1] as f32, c[2] as f32])
            .collect();
        let sw = crate::w();
        let sh = crate::h();
        for ry in 0..ih {
            let py = oy + ry;
            if py >= sh {
                break;
            }
            for rx in 0..iw {
                let px = ox + rx;
                if px >= sw {
                    continue;
                }
                let idx = ry * iw + rx;
                let [r, g, b] = work[idx];
                let r5 = (r.round().clamp(0.0, 255.0) as u8) >> 3;
                let g6 = (g.round().clamp(0.0, 255.0) as u8) >> 2;
                let b5 = (b.round().clamp(0.0, 255.0) as u8) >> 3;
                let qv = ((r5 as u16) << 11) | ((g6 as u16) << 5) | b5 as u16;
                let off = (py * sw + px) * 2;
                if off + 2 <= buf_bytes.len() {
                    buf_bytes[off] = (qv & 0xff) as u8;
                    buf_bytes[off + 1] = (qv >> 8) as u8;
                }
                let er = r - r5 as f32 * 255.0 / 31.0;
                let eg = g - g6 as f32 * 255.0 / 63.0;
                let eb = b - b5 as f32 * 255.0 / 31.0;
                let spread = |cell: &mut [f32; 3], wt: f32| {
                    cell[0] += er * wt;
                    cell[1] += eg * wt;
                    cell[2] += eb * wt;
                };
                if rx + 1 < iw {
                    spread(&mut work[idx + 1], 7.0 / 16.0);
                }
                if ry + 1 < ih {
                    if rx > 0 {
                        spread(&mut work[idx + iw - 1], 3.0 / 16.0);
                    }
                    spread(&mut work[idx + iw], 5.0 / 16.0);
                    if rx + 1 < iw {
                        spread(&mut work[idx + iw + 1], 1.0 / 16.0);
                    }
                }
            }
        }
    } else {
        // best-effort: logo decode failed — just the name centered.
        let label = "KoThok";
        let px = crate::h() as f32 * 0.08;
        let lw = measure_text(label, px);
        let lx = (crate::w()).saturating_sub(lw) / 2;
        text_render::blit_rgb565(
            buf_bytes,
            crate::w(),
            label,
            px,
            lx,
            crate::h() / 2,
            crate::w(),
            crate::h(),
        );
    }
}

pub fn paint_splash_spinner(buffer: &mut [Rgb565Pixel], angle_deg: u32) {
    let w = crate::w();
    let h = crate::h();
    let buf_bytes = rgb565_as_bytes(buffer);
    let cx = w as f32 / 2.0;
    let cy = h as f32 - 130.0;
    let badge_r = 52.0f32;
    let arc_r = 36.0f32;
    let arc_thick = 9.0f32;
    let span = 300u32;
    for py in 0..h {
        let dy = py as f32 - cy;
        if dy.abs() > badge_r + 2.0 {
            continue;
        }
        for px in 0..w {
            let dx = px as f32 - cx;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > badge_r + 2.0 {
                continue;
            }
            let off = (py * w + px) * 2;
            if off + 2 > buf_bytes.len() {
                continue;
            }
            let mut val: u16 = 0xFFFF;
            if (dist - arc_r).abs() <= arc_thick {
                let a = (-dy).atan2(dx).to_degrees().rem_euclid(360.0);
                let start = angle_deg as f32;
                let end = start + span as f32;
                let in_arc = if end <= 360.0 {
                    a >= start && a < end
                } else {
                    a >= start || a < end - 360.0
                };
                if in_arc {
                    val = 0x0000;
                }
            }
            buf_bytes[off] = (val & 0xff) as u8;
            buf_bytes[off + 1] = (val >> 8) as u8;
        }
    }
}

pub fn splash_spinner_rect() -> (i32, i32, i32, i32) {
    let w = crate::w() as i32;
    let h = crate::h() as i32;
    let cx = w / 2;
    let cy = h - 130;
    let r = 56;
    (cx - r, cy - r, r * 2, r * 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splash_spinner_rect_is_square_and_centered_horizontally() {
        let (x, y, w, h) = splash_spinner_rect();
        assert_eq!(w, h, "spinner must be square");
        assert_eq!(w, 112, "spinner diameter is 2*r = 112");
        let screen_w = crate::w() as i32;
        assert_eq!(x + w + x, screen_w, "spinner must be centered on screen");
        let screen_h = crate::h() as i32;
        assert!(
            y > 0 && (y + h) < screen_h,
            "spinner must be fully on-screen"
        );
    }

    #[test]
    fn splash_spinner_rect_sits_near_bottom() {
        let (x, _y, _w, h) = splash_spinner_rect();
        let screen_h = crate::h() as i32;
        let r = h / 2;
        let cx = x + r;
        let screen_w = crate::w() as i32;
        assert_eq!(cx, screen_w / 2, "centered horizontally");
        assert!(
            screen_h - 130 > screen_h / 2,
            "sanity: anchor is below center"
        );
    }
}
