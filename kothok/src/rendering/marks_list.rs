// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
#![allow(dead_code)]
use slint::platform::software_renderer::Rgb565Pixel;

use crate::data::mark::{Mark, MarkKind};
use crate::rendering::chapter_list::{
    paint_scrollbar, CH_LIST_BOTTOM_PAD, CH_LIST_TOP, CH_ROW_H, CH_ROW_PITCH, CH_ROW_X,
    SB_ROW_SHRINK,
};
use crate::rendering::common::rgb565_as_bytes;
use crate::rendering::draw::{fill_rounded_rect, truncate_to_width};
use crate::rendering::text_render;
use crate::rendering::word_list::{INK, TAB_BORDER};

const WHITE: u16 = 0xFFFF;
const BORDER: u16 = 0x94B2;
const MUTED: u16 = 0x3186;
const KIND_BAR_PX: usize = 3;
const DELETE_LABEL: &str = "Delete";
const DELETE_RESERVE_PX: usize = 80;

pub fn paint_marks_list(
    buf: &mut [Rgb565Pixel],
    marks: &[Mark],
    marks_scroll: i32,
    body_px: f32,
    armed_mark_idx: usize,
    dragging: bool,
) {
    let w = crate::w();
    let h = crate::h();
    let list_bottom = h as i32 - CH_LIST_BOTTOM_PAD;
    for y in CH_LIST_TOP..list_bottom {
        let off = (y as usize) * w;
        buf[off..off + w].fill(Rgb565Pixel(0xFFFF));
    }
    let buf_bytes = rgb565_as_bytes(buf);
    if marks.is_empty() {
        paint_empty_message(buf, w, h, "No bookmarks or highlights yet");
        return;
    }
    let lh = text_render::line_height(body_px) as i32;
    let row_w = (w as i32 - 2 * CH_ROW_X - SB_ROW_SHRINK) as usize;
    for (i, m) in marks.iter().enumerate() {
        let y = CH_LIST_TOP + (i as i32) * CH_ROW_PITCH - marks_scroll;
        if y < CH_LIST_TOP || y + CH_ROW_H > list_bottom {
            continue;
        }
        let is_sel = armed_mark_idx != usize::MAX && i == armed_mark_idx;
        let (fill, border) = if is_sel {
            (INK, TAB_BORDER)
        } else {
            (WHITE, BORDER)
        };
        fill_rounded_rect(
            buf_bytes,
            w,
            h,
            CH_ROW_X as usize,
            y as usize,
            row_w,
            CH_ROW_H as usize,
            fill,
            border,
            8,
        );
        let ch_fg: u16 = if is_sel { WHITE } else { MUTED };
        let is_armed = armed_mark_idx != usize::MAX && i == armed_mark_idx;
        let excerpt_max_w = if is_armed {
            row_w.saturating_sub(DELETE_RESERVE_PX)
        } else {
            row_w
        };
        let truncated = truncate_to_width(&m.excerpt, body_px, excerpt_max_w);
        let text_y = (y + (CH_ROW_H - lh) / 2).max(0) as usize;
        text_render::blit_rgb565_color(
            buf_bytes,
            w,
            &truncated,
            body_px,
            CH_ROW_X as usize + KIND_BAR_PX + 4,
            text_y,
            ch_fg,
            w,
            h,
        );
        let ch_label = format!("Ch {} - p {}", m.chapter + 1, m.page_hint + 1);
        let ch_max_w = w - CH_ROW_X as usize - 20 - SB_ROW_SHRINK as usize;
        let ch_trunc = crate::rendering::draw::truncate_to_width(&ch_label, 18.0, ch_max_w);
        text_render::blit_rgb565_color(
            buf_bytes,
            w,
            &ch_trunc,
            18.0,
            CH_ROW_X as usize + KIND_BAR_PX + 4,
            text_y + lh as usize,
            ch_fg,
            w,
            h,
        );
        if is_armed {
            let del_w = crate::rendering::draw::measure_text(DELETE_LABEL, body_px);
            let del_x = w - SB_ROW_SHRINK as usize - 10 - del_w.min(DELETE_RESERVE_PX);
            let min_x = (CH_ROW_X as usize + KIND_BAR_PX + 4)
                .max(row_w.saturating_sub(DELETE_RESERVE_PX) + CH_ROW_X as usize);
            text_render::blit_rgb565_color(
                buf_bytes,
                w,
                DELETE_LABEL,
                body_px,
                del_x.max(min_x),
                (y + (CH_ROW_H - lh) / 2).max(0) as usize,
                WHITE,
                w,
                h,
            );
        }
        paint_kind_marker(
            buf_bytes,
            w,
            h,
            m,
            y as usize,
            CH_ROW_H as usize,
            CH_ROW_X as usize,
            is_sel,
        );
    }
    paint_scrollbar(buf_bytes, w, h, marks.len(), marks_scroll, dragging);
}

fn paint_kind_marker(
    buf_bytes: &mut [u8],
    w: usize,
    h: usize,
    m: &Mark,
    row_y: usize,
    row_h: usize,
    row_x: usize,
    is_sel: bool,
) {
    if m.kind != MarkKind::Highlight {
        return;
    }
    let bar_color: u16 = if is_sel { WHITE } else { INK };
    let bar_bytes = bar_color.to_le_bytes();
    let inset = 4;
    for ry in 0..row_h {
        let y = row_y + ry;
        if y >= h {
            break;
        }
        for dx in 0..KIND_BAR_PX {
            let px = row_x + inset + dx;
            if px < w {
                let idx = (y * w + px) * 2;
                if idx + 1 < buf_bytes.len() {
                    buf_bytes[idx] = bar_bytes[0];
                    buf_bytes[idx + 1] = bar_bytes[1];
                }
            }
        }
    }
}

fn paint_empty_message(buf: &mut [Rgb565Pixel], w: usize, h: usize, msg: &str) {
    let lh = 24.0;
    let text_w = crate::rendering::draw::measure_text(msg, lh);
    let x = (w.saturating_sub(text_w)) / 2;
    let y = ((CH_LIST_TOP as usize + (h as i32 - CH_LIST_BOTTOM_PAD) as usize) / 2)
        .max(CH_LIST_TOP as usize);
    let buf_bytes = rgb565_as_bytes(buf);
    text_render::blit_rgb565_color(buf_bytes, w, msg, lh, x, y, INK, w, h);
}

pub fn marks_list_hit_test(tap_y: i32, scroll: i32, count: usize) -> Option<usize> {
    for i in 0..count {
        let y = CH_LIST_TOP + (i as i32) * CH_ROW_PITCH - scroll;
        if tap_y >= y && tap_y < y + CH_ROW_H {
            return Some(i);
        }
    }
    None
}
