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

const WORDMARK_PNG: &[u8] = include_bytes!("../../ui/kothok-wordmark-ink.png");

#[inline]
fn sc(n: f32, s: f32) -> usize {
    (n * s).round().max(0.0) as usize
}

pub fn show_about(fb: &Fb, buffer: &mut [Rgb565Pixel], device_model: &str) {
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
    draw_info_column(buf, w, h, device_model, s);
    draw_author_card(buf, w, h, s);
    draw_qr_section(buf, w, h, s);
    draw_footer(buf, w, h, cx, s);

    if cfg!(feature = "ppm-dump") {
        dump_ppm(crate::data::config::PPM_DEBUG, buf, w, h);
    }
    fb.present(buf, w, h, true, 0, 0, WAVE_GC16);
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

fn draw_wordmark(buf: &mut [u8], w: usize, h: usize, s: f32) {
    let wm_w = sc(620.0, s);
    let wm_h = sc(148.0, s);
    if let Some(img) = text_render::decode_image_rgba(WORDMARK_PNG, wm_w, wm_h) {
        text_render::blit_rgb565_image_alpha(
            buf,
            w,
            &img.rgba,
            img.width,
            img.height,
            sc(322.0, s),
            sc(392.0, s),
            w,
            h,
        );
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

fn draw_info_column(buf: &mut [u8], w: usize, h: usize, device_model: &str, s: f32) {
    let lx = sc(64.0, s);
    let lbl = 22.0 * s;
    let val = 30.0 * s;
    let sml = 26.0 * s;
    let mut txt = |t: &str, px: f32, dy: f32, c: u16| {
        text_render::blit_rgb565_color(buf, w, t, px, lx, sc(dy, s), c, w, h);
    };
    txt("PRIVACY", lbl, 790.0, LABEL_CLR);
    txt("Everything stays on this device", val, 830.0, INK_TXT);
    txt("VOICE", lbl, 940.0, LABEL_CLR);
    txt("Plain text only, sent to", val, 980.0, INK_TXT);
    txt("Microsoft Edge TTS", val, 1020.0, INK_TXT);
    txt("BUILT WITH", lbl, 1140.0, LABEL_CLR);
    txt("Rust + Slint", val, 1180.0, INK_TXT);
    txt("Free for personal use", sml, 1220.0, MUTED);
    txt("RUNNING ON", lbl, 1340.0, LABEL_CLR);
    txt(device_model, val, 1380.0, INK_TXT);
    txt("CONTACT", lbl, 1470.0, LABEL_CLR);
    txt("KoThok@bitops.bd", sml, 1510.0, INK_TXT);
    txt("github.com/Nayeem170/KoThok", sml, 1550.0, INK_TXT);
}

fn draw_author_card(buf: &mut [u8], w: usize, h: usize, s: f32) {
    let card_x = sc(712.0, s);
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
    let card_x = sc(712.0, s);
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
        sc(1608.0, s),
        sc(1136.0, s),
        sc(2.0, s).max(1),
        DIVIDER,
    );
    text_center(
        buf,
        w,
        h,
        "(c) 2026 Nayeem Bin Ahsan",
        22.0 * s,
        cx,
        sc(1634.0, s),
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
