// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::rendering::common::{rgb565_as_bytes, BRAND_RED_RGB565};
use crate::rendering::draw::{fill_rounded_rect, measure_text};
use crate::rendering::fb::{dump_ppm, Fb, WAVE_GC16};
use crate::rendering::text_render;
use crate::VERSION;

use slint::platform::software_renderer::Rgb565Pixel;

const DESIGN_W: f32 = 1264.0;

const BAND_INK: u16 = 0x1082;
const WHITE: u16 = 0xFFFF;
const TAGLINE: u16 = 0xCE79;
const LABEL_CLR: u16 = 0x8C71;
const INK_TXT: u16 = 0x1082;
const CARD_ROLE: u16 = 0xB5B6;
const MUTED: u16 = 0x73AE;
const DIVIDER: u16 = 0xDEDB;
const FOOTER_CLR: u16 = 0x9CD3;
const LOGO_GREEN: u16 = 0x14EE;

const QR_MATRIX: [&str; 25] = [
    "1111111010111100001111111",
    "1000001001110010101000001",
    "1011101010101111101011101",
    "1011101001111111001011101",
    "1011101000011010001011101",
    "1000001011001110001000001",
    "1111111010101010101111111",
    "0000000000110011000000000",
    "1010001101010001100100101",
    "0101100111100011101101011",
    "1110111010010111100001101",
    "0011000111111001111011000",
    "1000101000111000101100001",
    "0110010100101011011100011",
    "1111001001101011001001101",
    "0001010001011011000111000",
    "1100111011101010111110010",
    "0000000010100100100010001",
    "1111111011011000101010001",
    "1000001000000001100010000",
    "1011101001111001111110010",
    "1011101000000001110010110",
    "1011101011111010010111011",
    "1000001001101000101110000",
    "1111111010111000111001001",
];

const WORDMARK_PNG: &[u8] = include_bytes!("../../ui/kothok-wordmark.png");

#[inline]
fn sc(n: f32, s: f32) -> usize {
    (n * s).round().max(0.0) as usize
}

pub fn show_about(fb: &Fb, buffer: &mut [Rgb565Pixel]) {
    let w = crate::w();
    let h = crate::h();
    let s = w as f32 / DESIGN_W;
    let cx = w / 2;

    buffer.fill(Rgb565Pixel(0xFFFF));
    let buf = rgb565_as_bytes(buffer);

    fill_rect(buf, w, h, 0, 0, w, sc(720.0, s), BAND_INK);
    draw_close_button(buf, w, h, s);
    draw_logo_mark(buf, w, h, s);
    draw_wordmark(buf, w, h, s);
    text_center(
        buf,
        w,
        h,
        "Read | Listen | Anywhere",
        30.0 * s,
        cx,
        sc(582.0, s),
        TAGLINE,
    );
    draw_version_pill(buf, w, h, cx, s);
    // The update banner and the "RUNNING ON <model>" block both moved to the
    // device settings panel, where the check is a button the reader presses
    // rather than a line that appears on a page they opened for other reasons.
    draw_info_column(buf, w, h, s);
    draw_author_card(buf, w, h, s);
    draw_qr_section(buf, w, h, s);
    draw_footer(buf, w, h, cx, s);

    if cfg!(feature = "ppm-dump") {
        dump_ppm(crate::data::config::PPM_DEBUG, buf, w, h);
    }
    fb.present(buf, w, h, false, 0, h, WAVE_GC16);
}

#[allow(clippy::too_many_arguments)]
fn fill_rect(
    buf: &mut [u8],
    w: usize,
    h: usize,
    x: usize,
    y: usize,
    rw: usize,
    rh: usize,
    color: u16,
) {
    let lo = (color & 0xff) as u8;
    let hi = (color >> 8) as u8;
    for ry in 0..rh {
        let py = y + ry;
        if py >= h {
            break;
        }
        for rx in 0..rw {
            let px = x + rx;
            if px >= w {
                break;
            }
            let off = (py * w + px) * 2;
            if off + 2 > buf.len() {
                break;
            }
            buf[off] = lo;
            buf[off + 1] = hi;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_thick_line(
    buf: &mut [u8],
    w: usize,
    h: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    thick: usize,
    color: u16,
) {
    let steps = ((x1 as f32 - x0 as f32).abs())
        .max((y1 as f32 - y0 as f32).abs())
        .ceil()
        .max(1.0) as usize;
    let half = thick as i32 / 2;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let fx = x0 as f32 + t * (x1 as f32 - x0 as f32);
        let fy = y0 as f32 + t * (y1 as f32 - y0 as f32);
        fill_rect(
            buf,
            w,
            h,
            (fx as i32 - half).max(0) as usize,
            (fy as i32 - half).max(0) as usize,
            thick,
            thick,
            color,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn fill_left_triangle(
    buf: &mut [u8],
    w: usize,
    h: usize,
    apex_x: usize,
    apex_y: usize,
    right_x: usize,
    top_y: usize,
    bot_y: usize,
    color: u16,
) {
    for y in top_y..=bot_y {
        let t = if y <= apex_y {
            if apex_y == top_y {
                0.0
            } else {
                (apex_y - y) as f32 / (apex_y - top_y) as f32
            }
        } else if bot_y == apex_y {
            0.0
        } else {
            (y - apex_y) as f32 / (bot_y - apex_y) as f32
        };
        let xl = (apex_x as f32 + t * (right_x - apex_x) as f32).round() as usize;
        if xl <= right_x {
            fill_rect(buf, w, h, xl, y, right_x - xl + 1, 1, color);
        }
    }
}

fn draw_close_button(buf: &mut [u8], w: usize, h: usize, s: f32) {
    let btn = sc(76.0, s);
    let bx = w.saturating_sub(sc(23.0, s) + btn);
    let by = sc(17.0, s);
    fill_rounded_rect(buf, w, h, bx, by, btn, btn, BAND_INK, WHITE, btn / 2);
    let thick = (5.0 * s).round().max(2.0) as usize;
    let p1 = sc(23.0, s);
    let p2 = sc(53.0, s);
    draw_thick_line(buf, w, h, bx + p1, by + p1, bx + p2, by + p2, thick, WHITE);
    draw_thick_line(buf, w, h, bx + p2, by + p1, bx + p1, by + p2, thick, WHITE);
}

fn draw_logo_mark(buf: &mut [u8], w: usize, h: usize, s: f32) {
    let ox = sc(548.0, s);
    let oy = sc(190.0, s);
    let k = sc(168.0, s) as f32 / 128.0;
    fill_rounded_rect(
        buf,
        w,
        h,
        ox + (8.0 * k) as usize,
        oy + (14.0 * k) as usize,
        (32.0 * k).round() as usize,
        (100.0 * k).round() as usize,
        BRAND_RED_RGB565,
        BRAND_RED_RGB565,
        (16.0 * k).round() as usize,
    );
    fill_left_triangle(
        buf,
        w,
        h,
        ox + (48.0 * k).round() as usize,
        oy + (64.0 * k).round() as usize,
        ox + (118.0 * k).round() as usize,
        oy + (16.0 * k).round() as usize,
        oy + (112.0 * k).round() as usize,
        LOGO_GREEN,
    );
}

/// The 8-bit colour an RGB565 constant expands to.
///
/// Lets a blend against a filled area be derived from the fill's own constant
/// instead of restating it as a literal, which is how the two drift apart.
const fn rgb565_to_rgb8(v: u16) -> (u8, u8, u8) {
    let r = ((v >> 11) & 0x1f) as u8;
    let g = ((v >> 5) & 0x3f) as u8;
    let b = (v & 0x1f) as u8;
    // Replicate the high bits into the low ones so full-scale stays full-scale.
    (
        (r << 3) | (r >> 2),
        (g << 2) | (g >> 4),
        (b << 3) | (b >> 2),
    )
}

fn draw_wordmark(buf: &mut [u8], w: usize, h: usize, s: f32) {
    let wm_h = sc(148.0, s);
    let wm_w = (wm_h as f32 * 554.0 / 140.0).round() as usize;
    let Some(img) = text_render::decode_image_rgba(WORDMARK_PNG, wm_w, wm_h) else {
        return;
    };
    // Centred on the panel, not placed at a design-space x. The old x was tuned
    // for the width the *previous* wordmark asset rendered to; the master this
    // now draws is a tighter crop, so the same x left it sitting short of centre
    // under a logo mark and above a tagline that are both centred.
    let ox = (w / 2).saturating_sub(img.width / 2);
    let oy = sc(392.0, s);
    // BAND_INK fills the top band with a near-black, and the wordmark master is
    // black ink on alpha -- blitting it as-is paints black on black. The master
    // is a monochrome mask, so the colour is ours to choose: tint it white and
    // blend against the band it sits on.
    blit_tinted(
        buf,
        w,
        h,
        &img,
        ox,
        oy,
        (255, 255, 255),
        rgb565_to_rgb8(BAND_INK),
    );
}

/// Composite a monochrome alpha mask against a solid background, tinting each
/// pixel with `fg`. Used to render the wordmark master (black on alpha) as
/// white text on the dark about-page band without shipping a second raster.
fn blit_tinted(
    buf: &mut [u8],
    w: usize,
    h: usize,
    img: &text_render::DecodedRgba,
    ox: usize,
    oy: usize,
    fg: (u8, u8, u8),
    bg: (u8, u8, u8),
) {
    for ry in 0..img.height {
        let py = oy + ry;
        if py >= h {
            break;
        }
        for rx in 0..img.width {
            let px = ox + rx;
            if px >= w {
                break;
            }
            let a = img.rgba[(ry * img.width + rx) * 4 + 3] as u32;
            if a == 0 {
                continue;
            }
            let mix =
                |f: u8, b: u8| -> u16 { ((f as u32 * a + b as u32 * (255 - a)) / 255) as u16 };
            let (r, g, b) = (mix(fg.0, bg.0), mix(fg.1, bg.1), mix(fg.2, bg.2));
            let v = ((r >> 3) << 11) | ((g >> 2) << 5) | (b >> 3);
            let off = (py * w + px) * 2;
            if off + 2 > buf.len() {
                continue;
            }
            buf[off] = (v & 0xff) as u8;
            buf[off + 1] = (v >> 8) as u8;
        }
    }
}

fn draw_version_pill(buf: &mut [u8], w: usize, h: usize, cx: usize, s: f32) {
    let label = format!("v{}", VERSION);
    let px = 26.0 * s;
    let tw = measure_text(&label, px);
    let pad_x = sc(28.0, s);
    let pad_y = sc(8.0, s);
    let pw = tw + pad_x * 2;
    let ph = text_render::line_height(px) as usize + pad_y * 2;
    let px_pos = cx.saturating_sub(pw / 2);
    let py_pos = sc(640.0, s);
    fill_rounded_rect(
        buf,
        w,
        h,
        px_pos,
        py_pos,
        pw,
        ph,
        BAND_INK,
        WHITE,
        sc(28.0, s),
    );
    text_render::blit_rgb565_color(
        buf,
        w,
        &label,
        px,
        px_pos + pad_x,
        py_pos + pad_y,
        WHITE,
        w,
        h,
    );
}

/// Body size of the library header's "Library" title on the design panel, so
/// this page reads as part of the same app rather than a separate design.
///
/// Scaled by `s` at use, unlike the header's own raw constant. Every position
/// on this page is design-space and shrinks with the panel; a type size that
/// did not shrink with them stopped fitting the gaps it was spaced for, and on
/// the 1072-wide panels the two "VOICE" lines overlapped by 4px. The page's
/// contract is that the whole composition scales together.
const INFO_VALUE_PX: f32 = crate::rendering::layout::BODY_PX * 0.92;

/// Type class of an information row, resolved to a size per panel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum InfoSize {
    Label,
    Value,
    Small,
}

fn info_px(size: InfoSize, s: f32) -> f32 {
    match size {
        InfoSize::Label => 22.0 * s,
        InfoSize::Value => INFO_VALUE_PX * s,
        InfoSize::Small => INFO_VALUE_PX * 0.85 * s,
    }
}

/// The left information column: design-space y, type class, text, colour.
///
/// A table rather than a run of draw calls so the spacing can be checked for
/// every panel in the fleet without painting a frame -- these rows are placed
/// at fixed y values but sized from the body font, so whether they collide is
/// arithmetic that has to be verified rather than eyeballed on one device.
///
/// "RUNNING ON <model>" moved to the device settings panel; the rows below are
/// respaced into the gap it left rather than leaving a hole.
const INFO_ROWS: &[(f32, InfoSize, &str, u16)] = &[
    (790.0, InfoSize::Label, "PRIVACY", LABEL_CLR),
    (
        834.0,
        InfoSize::Value,
        "Everything stays on this device",
        INK_TXT,
    ),
    (960.0, InfoSize::Label, "VOICE", LABEL_CLR),
    (1004.0, InfoSize::Value, "Plain text only, sent to", INK_TXT),
    (1052.0, InfoSize::Value, "Microsoft Edge TTS", INK_TXT),
    (1178.0, InfoSize::Label, "BUILT WITH", LABEL_CLR),
    (1222.0, InfoSize::Value, "Rust + Slint", INK_TXT),
    (1270.0, InfoSize::Small, "Free for personal use", MUTED),
    (1396.0, InfoSize::Label, "CONTACT", LABEL_CLR),
    (1440.0, InfoSize::Small, "KoThok@bitops.bd", INK_TXT),
    (
        1484.0,
        InfoSize::Small,
        "github.com/Nayeem170/KoThok",
        INK_TXT,
    ),
];

/// Left edge of the column, and of the author card it must not reach, in
/// design space. Shared with the tests so one cannot be moved without the
/// other noticing.
const INFO_COL_X: f32 = 64.0;
const RIGHT_COL_X: f32 = 712.0;

fn draw_info_column(buf: &mut [u8], w: usize, h: usize, s: f32) {
    let lx = sc(INFO_COL_X, s);
    for &(dy, size, text, colour) in INFO_ROWS {
        text_render::blit_rgb565_color(buf, w, text, info_px(size, s), lx, sc(dy, s), colour, w, h);
    }
}

fn draw_author_card(buf: &mut [u8], w: usize, h: usize, s: f32) {
    let card_x = sc(RIGHT_COL_X, s);
    let card_y = sc(790.0, s);
    fill_rect(
        buf,
        w,
        h,
        card_x,
        card_y,
        sc(488.0, s),
        sc(360.0, s),
        BAND_INK,
    );

    let tx = card_x + sc(32.0, s);
    let mut y = card_y + sc(34.0, s);
    let mut txt = |t: &str, px: f32, yy: usize, c: u16| {
        text_render::blit_rgb565_color(buf, w, t, px, tx, yy, c, w, h);
    };
    let lbl = 22.0 * s;
    txt("BUILT BY", lbl, y, LABEL_CLR);
    y += text_render::line_height(lbl) as usize + sc(18.0, s);

    let name_px = 44.0 * s;
    let name_lh = text_render::line_height(name_px) as usize;
    txt("Nayeem", name_px, y, WHITE);
    y += name_lh;
    txt("Bin Ahsan", name_px, y, WHITE);
    y += name_lh + sc(12.0, s);

    let role_px = 26.0 * s;
    txt("Software Engineer", role_px, y, CARD_ROLE);
    y += text_render::line_height(role_px) as usize + sc(26.0, s);
    txt("linkedin.com/in/nayeembinahsan", 23.0 * s, y, WHITE);
}

fn draw_qr_section(buf: &mut [u8], w: usize, h: usize, s: f32) {
    let card_x = sc(RIGHT_COL_X, s);
    let card_y = sc(1190.0, s);
    fill_rounded_rect(
        buf,
        w,
        h,
        card_x,
        card_y,
        sc(488.0, s),
        sc(252.0, s),
        WHITE,
        INK_TXT,
        0,
    );

    let qr_x = sc(740.0, s);
    let qr_y = sc(1228.0, s);
    let mod_px = (7.0 * s).round().max(3.0) as usize;
    draw_qr(buf, w, h, qr_x, qr_y, mod_px);

    let txt_x = sc(940.0, s);
    let mut txt = |t: &str, px: f32, dy: f32, c: u16| {
        text_render::blit_rgb565_color(buf, w, t, px, txt_x, sc(dy, s), c, w, h);
    };
    txt("WEBSITE", 22.0 * s, 1240.0, LABEL_CLR);
    txt("kothok.bitops.bd", 26.0 * s, 1284.0, INK_TXT);
    txt("Scan to open", 21.0 * s, 1330.0, MUTED);
}

fn draw_qr(buf: &mut [u8], w: usize, h: usize, ox: usize, oy: usize, mod_px: usize) {
    for (row, line) in QR_MATRIX.iter().enumerate() {
        for (col, ch) in line.chars().enumerate() {
            if ch == '1' {
                fill_rect(
                    buf,
                    w,
                    h,
                    ox + col * mod_px,
                    oy + row * mod_px,
                    mod_px,
                    mod_px,
                    INK_TXT,
                );
            }
        }
    }
}

fn draw_footer(buf: &mut [u8], w: usize, h: usize, cx: usize, s: f32) {
    fill_rect(
        buf,
        w,
        h,
        sc(64.0, s),
        sc(1602.0, s),
        sc(1136.0, s),
        sc(2.0, s).max(1),
        DIVIDER,
    );
    text_center(
        buf,
        w,
        h,
        "Star on GitHub: github.com/Nayeem170/KoThok",
        22.0 * s,
        cx,
        sc(1618.0, s),
        INK_TXT,
    );
    text_center(
        buf,
        w,
        h,
        "(c) 2026 Nayeem Bin Ahsan",
        20.0 * s,
        cx,
        sc(1650.0, s),
        FOOTER_CLR,
    );
}

#[allow(clippy::too_many_arguments)]
fn text_center(
    buf: &mut [u8],
    w: usize,
    h: usize,
    text: &str,
    px: f32,
    cx: usize,
    y: usize,
    color: u16,
) -> usize {
    let tw = measure_text(text, px);
    let x = cx.saturating_sub(tw / 2);
    text_render::blit_rgb565_color(buf, w, text, px, x, y, color, w, h);
    text_render::line_height(px) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every panel KoThok runs on, matching `splash::tests::FLEET`. The about
    /// page is a scaled design canvas, so "it looks right on the Libra" says
    /// nothing about the panel that is 600px wide.
    const FLEET: &[(&str, usize, usize)] = &[
        ("Touch/Mini", 600, 800),
        ("Glo/Aura/Nia", 758, 1024),
        ("Clara", 1072, 1448),
        ("Aura H2O", 1080, 1430),
        ("Libra", 1264, 1680),
        ("Elipsa", 1404, 1872),
        ("Forma/Sage", 1440, 1920),
    ];

    /// The information rows sit at fixed design-space y values but are sized
    /// from the body font, so nothing structural stops one from being drawn
    /// over the next. It happened: while `INFO_VALUE_PX` was used unscaled,
    /// "Plain text only, sent to" ran 4px into "Microsoft Edge TTS" on every
    /// 1072-wide panel, and was clear only on the panel it was designed on.
    #[test]
    fn info_rows_never_overlap_on_any_panel() {
        for &(name, w, _h) in FLEET {
            let s = w as f32 / DESIGN_W;
            for pair in INFO_ROWS.windows(2) {
                let (dy, size, text, _) = pair[0];
                let (next_dy, ..) = pair[1];
                let bottom = sc(dy, s) + text_render::line_height(info_px(size, s));
                assert!(
                    bottom <= sc(next_dy, s),
                    "{name}: {text:?} reaches {bottom}px, next row starts at {}px",
                    sc(next_dy, s)
                );
            }
        }
    }

    /// The author card and QR card share these rows' vertical band, so a row
    /// that grows past the gutter runs into artwork rather than into margin.
    #[test]
    fn info_rows_stay_clear_of_the_right_column() {
        for &(name, w, _h) in FLEET {
            let s = w as f32 / DESIGN_W;
            let lx = sc(INFO_COL_X, s);
            let limit = sc(RIGHT_COL_X, s);
            for &(_, size, text, _) in INFO_ROWS {
                let right = lx + measure_text(text, info_px(size, s));
                assert!(
                    right <= limit,
                    "{name}: {text:?} ends at {right}px, right column starts at {limit}px"
                );
            }
        }
    }

    /// The logo mark above the wordmark and the tagline below it are both
    /// centred on the panel, so the wordmark has to be too. It was placed at a
    /// fixed design-space x tuned for the width of a *previous* asset; swapping
    /// in the tighter master left it 17px short of centre on the Libra.
    #[test]
    fn wordmark_is_centred_under_the_logo_mark() {
        for &(name, w, h) in FLEET {
            let s = w as f32 / DESIGN_W;
            let wm_h = sc(148.0, s);
            let wm_w = (wm_h as f32 * 554.0 / 140.0).round() as usize;
            let img = text_render::decode_image_rgba(WORDMARK_PNG, wm_w, wm_h)
                .unwrap_or_else(|| panic!("{name}: wordmark failed to decode"));
            let ox = (w / 2).saturating_sub(img.width / 2);
            let centre = ox + img.width / 2;
            assert!(
                centre.abs_diff(w / 2) <= 1,
                "{name}: wordmark centre {centre}px vs panel centre {}px",
                w / 2
            );
            // A white-tinted mark blended against BAND_INK only reads as white
            // while it is on the band; past its bottom edge the page is white.
            let bottom = sc(392.0, s) + img.height;
            let band = sc(720.0, s);
            assert!(
                bottom <= band && band <= h,
                "{name}: wordmark reaches {bottom}px, band ends at {band}px"
            );
        }
    }

    /// The blend background is derived from `BAND_INK` rather than restated, so
    /// changing the band cannot leave the wordmark's anti-aliased edges blended
    /// against a colour that is no longer there.
    #[test]
    fn band_ink_expands_to_its_eight_bit_colour() {
        assert_eq!(rgb565_to_rgb8(BAND_INK), (16, 16, 16));
        assert_eq!(rgb565_to_rgb8(WHITE), (255, 255, 255));
        assert_eq!(rgb565_to_rgb8(0), (0, 0, 0));
    }
}
