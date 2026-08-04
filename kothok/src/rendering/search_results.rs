// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::Rgb565Pixel;

use crate::data::word_index::{WordHit, MAX_SEARCH_RESULTS};
use crate::rendering::chapter_list::{
    paint_scrollbar, CH_LIST_BOTTOM_PAD, CH_LIST_TOP, CH_ROW_H, CH_ROW_PITCH, CH_ROW_X,
    SB_ROW_SHRINK,
};
use crate::rendering::common::rgb565_as_bytes;
use crate::rendering::draw::fill_rounded_rect;
use crate::rendering::text_render;
use crate::rendering::word_list::{INK, TAB_BORDER};

const WHITE: u16 = 0xFFFF;
const BORDER: u16 = 0x94B2;
const SNIPPET_CHARS: usize = 70;
const SNIPPET_LEAD: usize = 15;
const MUTED: u16 = 0x3186;

/// Type size of the chapter line above each quote.
const CHAPTER_PX: f32 = 20.0;

/// Ceiling on the quote's type size.
///
/// A result row is a fixed `CH_ROW_H` tall and already spends its top on the
/// chapter line, so the reader's body size (20-60) cannot be honoured here.
/// A line box is about 1.36x its type size on Noto, so at the default 36 the
/// quote alone would be 49px and run past the row into the one below it.
/// `search_result_rows_fit_their_row_box` pins the arithmetic.
const SNIPPET_PX_MAX: f32 = 22.0;

/// Padding above the chapter line, and the gap between the two lines.
const ROW_PAD_TOP: i32 = 6;
const ROW_LINE_GAP: i32 = 4;

pub fn paint_search_results(
    buf: &mut [Rgb565Pixel],
    _word: &str,
    hits: &[WordHit],
    chapters: &[kobo_core::Chapter],
    scroll: i32,
    body_px: f32,
    total_hits: usize,
    selected: usize,
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
    let display_count = hits.len().min(MAX_SEARCH_RESULTS);
    let more_row = if total_hits > MAX_SEARCH_RESULTS {
        1
    } else {
        0
    };
    let scroll_item_count = display_count + more_row;
    for (i, hit) in hits.iter().take(display_count).enumerate() {
        let y = CH_LIST_TOP + (i as i32) * CH_ROW_PITCH - scroll;
        if y < CH_LIST_TOP || y + CH_ROW_H > list_bottom {
            continue;
        }
        let is_sel = selected != usize::MAX && i == selected;
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
            (w as i32 - 2 * CH_ROW_X - SB_ROW_SHRINK) as usize,
            CH_ROW_H as usize,
            fill,
            border,
            8,
        );
        let ch_label = chapter_label(chapters, hit.chapter as usize);
        let ch_font = CHAPTER_PX;
        let ch_lh = text_render::line_height(ch_font) as i32;
        let ch_y = (y + ROW_PAD_TOP).max(0) as usize;
        let ch_fg: u16 = if is_sel { WHITE } else { MUTED };
        let ch_max_w = w - 2 * CH_ROW_X as usize - 20 - SB_ROW_SHRINK as usize;
        let ch_trunc = crate::rendering::draw::truncate_to_width(&ch_label, ch_font, ch_max_w);
        text_render::blit_rgb565_color(
            buf_bytes,
            w,
            &ch_trunc,
            ch_font,
            (CH_ROW_X + 10) as usize,
            ch_y,
            ch_fg,
            CH_ROW_X as usize + 10 + ch_max_w,
            h,
        );
        let snippet = build_snippet(chapters, hit);
        let snip_x = (CH_ROW_X + 10) as usize;
        let snip_max = w - 2 * CH_ROW_X as usize - 20 - SB_ROW_SHRINK as usize;
        let snip_y = (y + ROW_PAD_TOP + ch_lh + ROW_LINE_GAP).max(0) as usize;
        let fg: u16 = if is_sel { WHITE } else { INK };
        text_render::blit_rgb565_color(
            buf_bytes,
            w,
            &snippet,
            snippet_px(body_px),
            snip_x,
            snip_y,
            fg,
            snip_x + snip_max,
            h,
        );
    }
    if total_hits > MAX_SEARCH_RESULTS {
        let more_y = CH_LIST_TOP + (display_count as i32) * CH_ROW_PITCH - scroll;
        if more_y >= CH_LIST_TOP && more_y < list_bottom {
            let msg = format!("and {} more...", total_hits - MAX_SEARCH_RESULTS);
            let msg_y =
                (more_y + (CH_ROW_H - text_render::line_height(18.0) as i32) / 2).max(0) as usize;
            text_render::blit_rgb565(
                buf_bytes,
                w,
                &msg,
                18.0,
                CH_ROW_X as usize + 20,
                msg_y,
                w,
                h,
            );
        }
    }
    paint_scrollbar(buf_bytes, w, h, scroll_item_count, scroll, dragging);
}

/// The quote's type size for a reader body size of `body_px`.
///
/// Tracks the reader's choice while it fits, then stops. Rows are a fixed
/// height, so past the ceiling the only thing a larger size buys is a quote
/// drawn over the next result.
fn snippet_px(body_px: f32) -> f32 {
    body_px.min(SNIPPET_PX_MAX)
}

/// Words of the chapter's own opening carried into the label after its title.
const CHAPTER_LEAD_WORDS: usize = 10;

/// The chapter line of a result row.
///
/// `display_title` alone is not enough to identify a chapter. It returns the
/// declared NCX title, else the first heading, else the first useful line, else
/// `"Chapter N"` -- and since `toc_rows` merges the spine in, most rows of a
/// book with a one-entry NCX have no declared title and land on `"Chapter N"`.
/// A bare position label says nothing about which chapter a hit came from,
/// which is the one question this line exists to answer.
///
/// So the label is always title *and* opening words, not one or the other. The
/// opening is skipped past the title when the two start the same, which they do
/// whenever the title came from the chapter's own first heading.
fn chapter_label(chapters: &[kobo_core::Chapter], idx: usize) -> String {
    let Some(ch) = chapters.get(idx) else {
        return format!("Ch {}", idx + 1);
    };
    let title = crate::data::library::chapter_display_title(ch, idx);
    let lead = chapter_lead(&ch.body, &title);
    if lead.is_empty() {
        title
    } else {
        format!("{title} - {lead}")
    }
}

/// The first `CHAPTER_LEAD_WORDS` words of `body`, past `title` if the body
/// opens with it.
fn chapter_lead(body: &str, title: &str) -> String {
    let rest = body.trim_start();
    let rest = rest.strip_prefix(title.trim()).unwrap_or(rest);
    let words: Vec<&str> = rest.split_whitespace().take(CHAPTER_LEAD_WORDS).collect();
    words.join(" ")
}

fn build_snippet(chapters: &[kobo_core::Chapter], hit: &WordHit) -> String {
    let body = chapters
        .get(hit.chapter as usize)
        .map(|c| c.body.as_str())
        .unwrap_or("");
    if body.is_empty() {
        return String::new();
    }
    let hit_pos = if body.is_char_boundary(hit.byte_offset as usize) {
        hit.byte_offset as usize
    } else {
        body.floor_char_boundary(hit.byte_offset as usize)
    };
    if hit_pos >= body.len() {
        return String::new();
    }
    let start = body[..hit_pos]
        .char_indices()
        .rev()
        .nth(SNIPPET_LEAD.saturating_sub(1))
        .map(|(i, _)| i)
        .unwrap_or(0);
    let window = &body[start..];
    let end = window
        .char_indices()
        .nth(SNIPPET_CHARS)
        .map_or(window.len(), |(i, _)| i);
    let snippet = &window[..end];
    let mut out = String::with_capacity(SNIPPET_CHARS + 8);
    if start > 0 {
        out.push_str("...");
    }
    out.push_str(snippet);
    if end < window.len() {
        out.push_str("...");
    }
    out
}

pub fn results_hit_test(tap_y: i32, scroll: i32, result_count: usize) -> Option<usize> {
    let h = crate::h() as i32;
    let list_bottom = h - CH_LIST_BOTTOM_PAD;
    if tap_y < CH_LIST_TOP || tap_y >= list_bottom {
        return None;
    }
    let i = (tap_y - CH_LIST_TOP + scroll) / CH_ROW_PITCH;
    if i >= 0 && (i as usize) < result_count {
        Some(i as usize)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn results_hit_test_first_row() {
        assert_eq!(results_hit_test(CH_LIST_TOP, 0, 5), Some(0));
    }

    #[test]
    fn results_hit_test_above_returns_none() {
        assert_eq!(results_hit_test(CH_LIST_TOP - 1, 0, 5), None);
    }

    #[test]
    fn results_hit_test_respects_scroll() {
        assert_eq!(results_hit_test(CH_LIST_TOP, CH_ROW_PITCH, 5), Some(1));
    }

    /// Both lines of a result must sit inside the row box for every body size
    /// the reader can pick. The quote used to be drawn at `body_px` directly,
    /// so at the default 36 it already ran 10px past the row and at 60 it was
    /// painted over the next result entirely.
    #[test]
    fn search_result_rows_fit_their_row_box() {
        let ch_lh = text_render::line_height(CHAPTER_PX) as i32;
        for body_px in 20..=60 {
            let snip_lh = text_render::line_height(snippet_px(body_px as f32)) as i32;
            let bottom = ROW_PAD_TOP + ch_lh + ROW_LINE_GAP + snip_lh;
            assert!(
                bottom <= CH_ROW_H,
                "body_px {body_px}: two lines reach {bottom}px in a {CH_ROW_H}px row"
            );
        }
    }

    /// A chapter with no declared title used to render as a bare "Chapter 3",
    /// which does not identify anything. The label must carry words from the
    /// chapter itself.
    #[test]
    fn chapter_lead_adds_words_from_the_chapter() {
        let lead = chapter_lead(
            "The morning the letter arrived, Tamara was still asleep upstairs in the old house.",
            "Chapter 3",
        );
        assert_eq!(lead.split_whitespace().count(), CHAPTER_LEAD_WORDS);
        assert!(lead.starts_with("The morning the letter"), "got {lead:?}");
    }

    /// When the title came from the chapter's own first heading the body starts
    /// with it, and repeating it would spend the line saying the same thing
    /// twice.
    #[test]
    fn chapter_lead_skips_a_repeated_title() {
        let lead = chapter_lead(
            "Prologue The house had been empty for years before they came.",
            "Prologue",
        );
        assert!(!lead.starts_with("Prologue"), "title repeated: {lead:?}");
        assert!(lead.starts_with("The house"), "got {lead:?}");
    }

    /// An empty chapter must not produce a dangling separator.
    #[test]
    fn chapter_lead_of_empty_body_is_empty() {
        assert_eq!(chapter_lead("", "Chapter 1"), "");
        assert_eq!(chapter_lead("   ", "Chapter 1"), "");
    }

    /// The cap has to bind somewhere below the default, or it is not doing
    /// anything -- and the reader's smaller sizes must still be honoured.
    #[test]
    fn snippet_size_tracks_the_reader_until_it_stops_fitting() {
        assert_eq!(snippet_px(20.0), 20.0, "small sizes pass through");
        assert_eq!(snippet_px(60.0), SNIPPET_PX_MAX, "large sizes clamp");
        assert!(
            SNIPPET_PX_MAX < 36.0,
            "the cap must bind at the default size"
        );
    }
}
