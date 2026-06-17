pub mod paginate;
pub mod state;

pub use kobo_core::rendering::layout::{sentences_with_ranges, word_wrap_bytes};
pub use state::*;

use std::collections::HashMap;

use crate::audio::Utterance;

pub const PAD_LEFT: usize = 24;
pub const PAD_TOP: usize = 48;
pub const GUTTER_W: usize = 6;
pub const GUTTER_PAD: usize = 18;
pub use kobo_core::rendering::common::BODY_PX;
pub const HEADING_H: i32 = 84;
pub const HEADING_GAP: i32 = 18;
pub const PARA_GAP: i32 = 8;
pub const FOOTER_H: i32 = 92;
pub const FOOTER_H_F: f32 = 92.0;

struct ScreenLayout {
    text_w: usize,
    content_h: i32,
}

static LAYOUT: std::sync::OnceLock<ScreenLayout> = std::sync::OnceLock::new();

pub fn init_layout(fb_w: usize, fb_h: usize) {
    let side = PAD_LEFT + GUTTER_W + GUTTER_PAD;
    LAYOUT.get_or_init(|| ScreenLayout {
        text_w: fb_w.saturating_sub(2 * side),
        content_h: fb_h as i32 - PAD_TOP as i32 - FOOTER_H,
    });
    log::debug!(
        "layout: fb={}x{} → text_w={}, content_h={}",
        fb_w,
        fb_h,
        LAYOUT.get().unwrap().text_w,
        LAYOUT.get().unwrap().content_h
    );
}

pub fn text_w() -> usize {
    LAYOUT.get().map(|l| l.text_w).unwrap_or(976)
}

pub fn content_h() -> i32 {
    LAYOUT.get().map(|l| l.content_h).unwrap_or(1308)
}

pub struct ChapterState {
    pub all_rows: Vec<crate::Row>,
    pub row_heights: Vec<i32>,
    pub pages: Vec<(usize, usize)>,
    pub utterances: Vec<Utterance>,
    pub decoded_images: HashMap<usize, crate::rendering::text_render::DecodedImage>,
}

#[cfg(test)]
mod tests;
