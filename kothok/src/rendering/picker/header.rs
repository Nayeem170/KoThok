// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::rendering::density::{dp, dpf};
use crate::rendering::draw::{fill_rounded_rect, measure_text};
use crate::rendering::layout::BODY_PX;
use crate::rendering::text_render;

use super::layout::PICKER_HEADER_H;

const LOGO_PNG: &[u8] = include_bytes!("../../../ui/kothok-logo.png");
const EXIT_PNG: &[u8] = include_bytes!("../../../ui/components/assets/exit.png");
const WORDMARK_PNG: &[u8] = include_bytes!("../../../ui/kothok-wordmark.png");

const NAV_BORDER_COLOR: u16 = 0x1082;

const HEADER_LOGO_PX: usize = 76;
const HEADER_LOGO_X: usize = 23;
const HEADER_LOGO_Y: usize = 17;
pub const HEADER_BTN_PX: usize = 76;
const HEADER_BTN_Y: usize = 17;
const HEADER_ICON_PX: usize = 38;
const HEADER_TEXT_PX: f32 = BODY_PX * 0.92;
const HEADER_SEP_H: usize = 3;
const HEADER_SEP_COLOR: u16 = 0xD6BA;

pub fn header_exit_x(screen_w: usize) -> usize {
    screen_w - HEADER_BTN_PX - HEADER_LOGO_X
}

pub(super) fn paint_picker_nav_bar(
    buf_bytes: &mut [u8],
    screen_w: usize,
    screen_h: usize,
    nav_y: usize,
    nav_h: usize,
    center: &str,
    clock: &str,
    battery: i32,
) {
    for ry in 0..nav_h {
        let py = nav_y + ry;
        if py >= screen_h {
            break;
        }
        for rx in 0..screen_w {
            let off = (py * screen_w + rx) * 2;
            if off + 2 > buf_bytes.len() {
                break;
            }
            let v = if ry == 0 { NAV_BORDER_COLOR } else { 0xFFFF };
            buf_bytes[off] = (v & 0xff) as u8;
            buf_bytes[off + 1] = (v >> 8) as u8;
        }
    }
    let px = dpf(BODY_PX * 0.66);
    let lh = text_render::line_height(px);
    let cy = nav_y + (nav_h.saturating_sub(lh)) / 2;

    if !clock.is_empty() {
        text_render::blit_rgb565(
            buf_bytes,
            screen_w,
            clock,
            px,
            dp(24) as usize,
            cy,
            screen_w,
            screen_h,
        );
    }
    if battery > 0 {
        let bat_str = format!("{}%", battery);
        let bw = measure_text(&bat_str, px);
        let bx = screen_w.saturating_sub(bw + dp(24) as usize);
        text_render::blit_rgb565(
            buf_bytes, screen_w, &bat_str, px, bx, cy, screen_w, screen_h,
        );
    }
    if !center.is_empty() {
        let cw = measure_text(center, px);
        let cx = (screen_w / 2).saturating_sub(cw / 2);
        text_render::blit_rgb565(buf_bytes, screen_w, center, px, cx, cy, screen_w, screen_h);
    }
}

pub(super) fn paint_library_header(buf_bytes: &mut [u8], screen_w: usize, screen_h: usize) {
    let header_h = PICKER_HEADER_H as usize;
    for ry in 0..header_h {
        if ry >= screen_h {
            break;
        }
        for rx in 0..screen_w {
            let off = (ry * screen_w + rx) * 2;
            if off + 2 > buf_bytes.len() {
                break;
            }
            let v = 0xFFFF;
            buf_bytes[off] = (v & 0xff) as u8;
            buf_bytes[off + 1] = (v >> 8) as u8;
        }
    }
    if let Some(img) = text_render::decode_image_rgba(LOGO_PNG, HEADER_LOGO_PX, HEADER_LOGO_PX) {
        text_render::blit_rgb565_image_alpha(
            buf_bytes,
            screen_w,
            &img.rgba,
            img.width,
            img.height,
            HEADER_LOGO_X,
            HEADER_LOGO_Y,
            screen_w,
            screen_h,
        );
    }
    let lh = text_render::line_height(HEADER_TEXT_PX);
    let ty = (header_h.saturating_sub(lh)) / 2;
    let wordmark_h = 44usize;
    let wordmark_w = wordmark_h * 658 / 158;
    if let Some(img) = text_render::decode_image_rgba(WORDMARK_PNG, wordmark_w, wordmark_h) {
        let wx = HEADER_LOGO_X + HEADER_LOGO_PX + 16;
        let wy = (header_h.saturating_sub(img.height)) / 2;
        text_render::blit_rgb565_image_alpha(
            buf_bytes, screen_w, &img.rgba, img.width, img.height, wx, wy, screen_w, screen_h,
        );
    }
    let title = "Library";
    let tw = measure_text(title, HEADER_TEXT_PX);
    let tx = (screen_w / 2).saturating_sub(tw / 2);
    text_render::blit_rgb565(
        buf_bytes,
        screen_w,
        title,
        HEADER_TEXT_PX,
        tx,
        ty,
        screen_w,
        screen_h,
    );
    let exit_x = header_exit_x(screen_w);
    const EXIT_RED: u16 = crate::rendering::common::BRAND_RED_RGB565;
    fill_rounded_rect(
        buf_bytes,
        screen_w,
        screen_h,
        exit_x,
        HEADER_BTN_Y,
        HEADER_BTN_PX,
        HEADER_BTN_PX,
        EXIT_RED,
        EXIT_RED,
        HEADER_BTN_PX / 2,
    );
    if let Some(img) = text_render::decode_image_rgba(EXIT_PNG, HEADER_ICON_PX, HEADER_ICON_PX) {
        let icon_x = exit_x + (HEADER_BTN_PX - img.width) / 2;
        let icon_y = HEADER_BTN_Y + (HEADER_BTN_PX - img.height) / 2;
        text_render::blit_rgb565_image_alpha(
            buf_bytes, screen_w, &img.rgba, img.width, img.height, icon_x, icon_y, screen_w,
            screen_h,
        );
    }
    let sep_top = header_h.saturating_sub(HEADER_SEP_H);
    for sy in sep_top..header_h {
        if sy >= screen_h {
            break;
        }
        for rx in 0..screen_w {
            let off = (sy * screen_w + rx) * 2;
            if off + 2 > buf_bytes.len() {
                break;
            }
            buf_bytes[off] = (HEADER_SEP_COLOR & 0xff) as u8;
            buf_bytes[off + 1] = (HEADER_SEP_COLOR >> 8) as u8;
        }
    }
}
