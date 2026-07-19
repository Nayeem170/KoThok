// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::rendering::common::{rgb565_as_bytes, ACCENT_BAR_RGB565, BRAND_RED_RGB565};
use crate::rendering::density::dpf;
use crate::rendering::draw::{fill_rounded_rect, measure_text};
use crate::rendering::fb::{dump_ppm, Fb, WAVE_GC16};
use crate::rendering::text_render;
use crate::VERSION;

use slint::platform::software_renderer::Rgb565Pixel;

const INK: u16 = 0x0000;
const MUTED: u16 = 0x8410;
const HINT: u16 = 0xB5B6;
const DIVIDER: u16 = 0xE71C;
const ACCENT: u16 = ACCENT_BAR_RGB565;

const LOGO_PNG: &[u8] = include_bytes!("../../ui/kothok-logo.png");

pub fn show_about(fb: &Fb, buffer: &mut [Rgb565Pixel], device_model: &str) {
    let w = crate::w();
    let h = crate::h();
    buffer.fill(Rgb565Pixel(0xFFFF));
    let buf = rgb565_as_bytes(buffer);

    let cx = w / 2;

    draw_close_button(buf, w, h);

    let logo_px = 140usize;
    let logo_y = 200usize;
    if let Some(img) = text_render::decode_image_rgba(LOGO_PNG, logo_px, logo_px) {
        text_render::blit_rgb565_image_alpha(
            buf,
            w,
            &img.rgba,
            img.width,
            img.height,
            cx - logo_px / 2,
            logo_y,
            w,
            h,
        );
    }

    let mut y = logo_y + logo_px + 32;
    y += text_center(buf, w, h, "KoThok", dpf(52.0), cx, y, INK);
    y += 8;
    y += text_center(
        buf,
        w,
        h,
        "Read | Listen | Anywhere",
        dpf(26.0),
        cx,
        y,
        MUTED,
    );
    y += 16;
    y += text_center(
        buf,
        w,
        h,
        &format!("v{}", VERSION),
        dpf(22.0),
        cx,
        y,
        ACCENT,
    );

    draw_h_line(buf, w, cx - 300, y + 48, 600, DIVIDER);
    y += 84;

    y += text_center_wrapped(
        buf, w, h,
        "All data stays on this device.\nVoice synthesis sends only plain text\nto Microsoft Edge TTS.",
        dpf(24.0), cx, y, 720, MUTED,
    );

    y += 36;
    y += text_center(
        buf,
        w,
        h,
        "Built with Rust + Slint",
        dpf(22.0),
        cx,
        y,
        MUTED,
    );
    y += 8;
    y += text_center(buf, w, h, "Free for personal use", dpf(22.0), cx, y, ACCENT);
    y += 28;
    y += text_center(
        buf,
        w,
        h,
        "(c) 2026 Nayeem Bin Ahsan",
        dpf(20.0),
        cx,
        y,
        HINT,
    );

    draw_h_line(buf, w, cx - 300, y + 40, 600, DIVIDER);
    y += 72;

    y += text_center(buf, w, h, "Built by", dpf(22.0), cx, y, MUTED);
    y += 8;
    y += text_center(buf, w, h, "Nayeem Bin Ahsan", dpf(30.0), cx, y, INK);
    y += 4;
    y += text_center(buf, w, h, "Software Engineer", dpf(22.0), cx, y, MUTED);
    y += 28;

    for line in &[
        "nayeemasis@hotmail.com",
        "github.com/Nayeem170/kothok",
        "linkedin.com/in/nayeembinahsan",
    ] {
        y += text_center(buf, w, h, line, dpf(23.0), cx, y, INK);
        y += 10;
    }

    y += 20;
    text_center(
        buf,
        w,
        h,
        &format!("Running on {}", device_model),
        dpf(20.0),
        cx,
        y,
        HINT,
    );

    if cfg!(feature = "ppm-dump") {
        dump_ppm(crate::data::config::PPM_DEBUG, buf, w, h);
    }
    fb.present(buf, w, h, true, 0, 0, WAVE_GC16);
}

fn draw_close_button(buf: &mut [u8], w: usize, h: usize) {
    let btn_px = 76usize;
    let pad = 23usize;
    let top = 17usize;
    let bx = w - pad - btn_px;
    fill_rounded_rect(
        buf,
        w,
        h,
        bx,
        top,
        btn_px,
        btn_px,
        BRAND_RED_RGB565,
        BRAND_RED_RGB565,
        btn_px / 2,
    );
    let px = dpf(38.0);
    let tw = measure_text("X", px);
    let tx = bx + (btn_px - tw) / 2;
    let ty = top + (btn_px - text_render::line_height(px) as usize) / 2;
    text_render::blit_rgb565_color(buf, w, "X", px, tx, ty, 0xFFFF, w, h);
}

fn draw_h_line(buf: &mut [u8], w: usize, x: usize, y: usize, width: usize, color: u16) {
    let h = crate::h();
    if y >= h {
        return;
    }
    for rx in 0..width {
        let px = x + rx;
        if px >= w {
            break;
        }
        let off = (y * w + px) * 2;
        if off + 2 > buf.len() {
            break;
        }
        buf[off] = (color & 0xff) as u8;
        buf[off + 1] = (color >> 8) as u8;
    }
}

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

fn text_center_wrapped(
    buf: &mut [u8],
    w: usize,
    h: usize,
    text: &str,
    px: f32,
    cx: usize,
    y: usize,
    _max_w: usize,
    color: u16,
) -> usize {
    let lh = text_render::line_height(px) as usize;
    let mut yy = y;
    for line in text.split('\n') {
        let tw = measure_text(line, px);
        let x = cx.saturating_sub(tw / 2);
        text_render::blit_rgb565_color(buf, w, line, px, x, yy, color, w, h);
        yy += lh;
    }
    yy - y
}
