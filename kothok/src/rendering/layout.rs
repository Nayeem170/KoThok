// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
pub mod paginate;
pub mod state;

pub use kobo_core::rendering::layout::{
    sentences_with_ranges, word_wrap_bytes, word_wrap_char_based, word_wrap_char_based_styled,
    word_wrap_indent,
};
pub use state::*;

use std::collections::HashMap;

use crate::audio::Utterance;

pub const PAD_LEFT: usize = 24;
/// Top of the text area == the header height, shared by every page's header
/// (see `content.slint`). Changing it repaginates: `content_h` shrinks and page
/// indices shift, so saved positions land a page or so off once.
pub const PAD_TOP: usize = 110;
pub const GUTTER_W: usize = 6;
pub const GUTTER_PAD: usize = 18;
pub use kobo_core::rendering::common::BODY_PX;
pub const HEADING_H: i32 = 84;
pub const HEADING_GAP: i32 = 18;
pub const PARA_GAP: i32 = 8;
pub const FOOTER_H: i32 = 92;
pub const FOOTER_H_F: f32 = 92.0;

/// Heading size as a fraction of the body font size. Shared by the initial
/// reader setup (`setup.rs`) and the live font reflow (`panel/callbacks/font.rs`)
/// so both agree on the head/body ratio after a font change.
pub const HEADING_SCALE: f32 = 0.78;
/// Line height as a multiple of the body font size. Same shared use as
/// `HEADING_SCALE`; changing it repaginates every chapter.
pub const LINE_HEIGHT_SCALE: f32 = 1.4;

/// `Row::tag` flags for body rows (`kind == 0`), where `tag` is otherwise unused.
/// They tell `overlay_text` how to place the line: justified (fill the column by
/// widening word gaps) and/or first-line indented.
pub const ROW_FLAG_JUSTIFY: i32 = 1;
pub const ROW_FLAG_INDENT: i32 = 2;
/// Set the row in the monospace face. Code only -- see `push_body_rows`.
pub const ROW_FLAG_MONO: i32 = 4;

/// Above the flag bits, `tag` also carries the row's **block** indent in px --
/// the whole-block inset a code listing's nesting level maps to, as opposed to
/// `ROW_FLAG_INDENT`'s first-line-only prose indent. Packed into the same field
/// because `Row` is a Slint struct: widening it means touching the `.slint`
/// type and every construction site, for a value that only ever needs 8 bits.
const ROW_INDENT_SHIFT: u32 = 8;
/// Widest block indent representable in the packed field.
pub const MAX_BLOCK_INDENT_PX: usize = 255;

pub fn pack_block_indent(px: usize) -> i32 {
    (px.min(MAX_BLOCK_INDENT_PX) as i32) << ROW_INDENT_SHIFT
}

/// Block indent of a body row, in px. 0 for every other row kind.
pub fn block_indent_px(row: &crate::Row) -> usize {
    if row.kind != 0 {
        return 0;
    }
    ((row.tag >> ROW_INDENT_SHIFT) as usize) & MAX_BLOCK_INDENT_PX
}

pub use kobo_core::rendering::layout::{block_indent_for, ScreenLayout};

pub fn screen_layout() -> ScreenLayout {
    ScreenLayout {
        text_w: text_w(),
        content_h: content_h(),
        heading_h: HEADING_H,
        heading_gap: HEADING_GAP,
        para_gap: PARA_GAP,
    }
}

/// First-line paragraph indent, ~1.4em of the body size. Used both when wrapping
/// (the first line is narrower) and when rendering (the line starts inset).
pub fn first_line_indent(body_px: f32) -> usize {
    (body_px * 1.4) as usize
}

struct AppScreenLayout {
    text_w: usize,
    content_h: i32,
}

static LAYOUT: std::sync::OnceLock<AppScreenLayout> = std::sync::OnceLock::new();

pub fn init_layout(fb_w: usize, fb_h: usize) {
    let side = PAD_LEFT + GUTTER_W + GUTTER_PAD;
    LAYOUT.get_or_init(|| AppScreenLayout {
        text_w: fb_w.saturating_sub(2 * side),
        content_h: fb_h as i32 - PAD_TOP as i32 - FOOTER_H,
    });
    log::debug!(
        "layout: fb={}x{} -> text_w={}, content_h={}",
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
    /// Bold/italic spans over `body` offsets -- the same offsets rows carry in
    /// `start`/`end`, so the renderer resolves a character's style by lookup
    /// rather than the `Row` struct having to carry it.
    pub style_runs: Vec<kobo_core::html_text::StyleRun>,
}

impl ChapterState {
    /// Index of the page whose rows cover `offset`, if any.
    pub fn page_for_offset(&self, offset: usize) -> Option<usize> {
        self.pages.iter().position(|(s, e)| {
            self.all_rows.get(*s..*e).is_some_and(|rows| {
                rows.iter()
                    .any(|r| r.start < r.end && offset >= r.start as usize && offset < r.end as usize)
            })
        })
    }
}

#[cfg(test)]
pub use kobo_core::rendering::text_render::style_at;

#[cfg(test)]
mod tests;
