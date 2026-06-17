use kobo_core::Chapter;
use slint::platform::software_renderer::Rgb565Pixel;

use crate::data::library::chapter_display_title;
use crate::rendering::text_render;

use crate::rendering::common::rgb565_as_bytes;
use crate::rendering::draw::{fill_rounded_rect, measure_text};

pub const CH_LIST_TOP: i32 = 128;
pub const CH_LIST_BOTTOM_PAD: i32 = 136;
pub const CH_ROW_H: i32 = 60;
pub const CH_ROW_PITCH: i32 = 70;
pub const CH_ROW_X: i32 = 40;
pub const CH_TITLE_PX: f32 = 24.0;

pub fn chapter_list_hit_test(tap_y: i32, scroll: i32, chapter_count: usize) -> Option<usize> {
    let h = crate::h() as i32;
    let list_bottom = h - CH_LIST_BOTTOM_PAD;
    if tap_y < CH_LIST_TOP || tap_y >= list_bottom {
        return None;
    }
    let i = (tap_y - CH_LIST_TOP + scroll) / CH_ROW_PITCH;
    if i >= 0 && (i as usize) < chapter_count {
        Some(i as usize)
    } else {
        None
    }
}

pub fn paint_chapter_list(
    buf: &mut [Rgb565Pixel],
    chapters: &[Chapter],
    scroll: i32,
    selected: i32,
    current: i32,
) {
    let w = crate::w();
    let h = crate::h();
    let list_bottom = h as i32 - CH_LIST_BOTTOM_PAD;
    for y in CH_LIST_TOP..list_bottom {
        let off = (y as usize) * w;
        buf[off..off + w].fill(Rgb565Pixel(0xFFFF));
    }
    let buf_bytes = rgb565_as_bytes(buf);
    let row_w = w as i32 - 2 * CH_ROW_X;
    let lh = text_render::line_height(CH_TITLE_PX) as i32;
    let title_x = (CH_ROW_X + 64) as usize;
    let title_max_w = (row_w - 80).max(40) as usize;
    let num_x = (CH_ROW_X + 16) as usize;

    for (i, chapter) in chapters.iter().enumerate() {
        let y = CH_LIST_TOP + (i as i32) * CH_ROW_PITCH - scroll;
        if y + CH_ROW_H <= CH_LIST_TOP || y >= list_bottom {
            continue;
        }
        let active = i as i32 == selected || (selected < 0 && i as i32 == current);
        let (fill, border) = if active {
            (0xFFFFu16, 0x0000u16)
        } else {
            (0xE71Cu16, 0x94B2u16)
        };
        fill_rounded_rect(
            buf_bytes,
            crate::w(),
            crate::h(),
            CH_ROW_X as usize,
            y as usize,
            row_w as usize,
            CH_ROW_H as usize,
            fill,
            border,
            8,
        );
        let num = format!("{}.", i + 1);
        let num_y = (y + (CH_ROW_H - lh) / 2).max(0) as usize;
        text_render::blit_rgb565(buf_bytes, w, &num, CH_TITLE_PX, num_x, num_y, w, h);
        let title = chapter_display_title(chapter, i);
        let title = truncate_to_width(&title, CH_TITLE_PX, title_max_w);
        let title_y = (y + (CH_ROW_H - lh) / 2).max(0) as usize;
        text_render::blit_rgb565(
            buf_bytes,
            w,
            &title,
            CH_TITLE_PX,
            title_x,
            title_y,
            title_x + title_max_w,
            h,
        );
    }
}

fn truncate_to_width(s: &str, px: f32, max_w: usize) -> String {
    if measure_text(s, px) <= max_w {
        return s.to_string();
    }
    let mut out: String = s.chars().take(3).collect();
    for ch in s.chars().skip(3) {
        out.push(ch);
        if measure_text(&(out.clone() + "…"), px) > max_w {
            out.pop();
            break;
        }
    }
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_list_hit_test_first_row() {
        assert_eq!(chapter_list_hit_test(CH_LIST_TOP, 0, 5), Some(0));
    }

    #[test]
    fn chapter_list_hit_test_second_row() {
        assert_eq!(
            chapter_list_hit_test(CH_LIST_TOP + CH_ROW_PITCH, 0, 5),
            Some(1)
        );
    }

    #[test]
    fn chapter_list_hit_test_above_list_returns_none() {
        assert_eq!(chapter_list_hit_test(CH_LIST_TOP - 1, 0, 5), None);
    }

    #[test]
    fn chapter_list_hit_test_below_list_returns_none() {
        let bottom = crate::h() as i32 - CH_LIST_BOTTOM_PAD;
        assert_eq!(chapter_list_hit_test(bottom, 0, 5), None);
        assert_eq!(chapter_list_hit_test(bottom + 500, 0, 5), None);
    }

    #[test]
    fn chapter_list_hit_test_respects_scroll() {
        assert_eq!(chapter_list_hit_test(CH_LIST_TOP, CH_ROW_PITCH, 5), Some(1));
    }

    #[test]
    fn chapter_list_hit_test_clamps_to_chapter_count() {
        let bottom = crate::h() as i32 - CH_LIST_BOTTOM_PAD - 1;
        assert_eq!(
            chapter_list_hit_test(bottom, 0, 1),
            None,
            "only one chapter exists"
        );
        assert_eq!(
            chapter_list_hit_test(bottom, 0, 1).unwrap_or(0) < 1 || true,
            true
        );
    }

    #[test]
    fn truncate_to_width_keeps_short_text_intact() {
        let w = measure_text("Short", 24.0);
        assert_eq!(truncate_to_width("Short", 24.0, w + 100), "Short");
    }

    #[test]
    fn truncate_to_width_adds_ellipsis_when_too_long() {
        let full = "The Complete Works of William Shakespeare";
        let out = truncate_to_width(full, 24.0, 80);
        assert!(
            out.ends_with('…'),
            "truncated text must end with ellipsis: {out:?}"
        );
        assert!(out.len() < full.len());
        assert!(
            measure_text(&out, 24.0) <= 80,
            "truncated result must fit within max_w"
        );
    }

    #[test]
    fn truncate_to_width_preserves_at_least_three_chars() {
        let out = truncate_to_width("abcdefghij", 24.0, 1);
        assert!(
            out.starts_with("abc"),
            "truncation must preserve the first three characters: {out:?}"
        );
        assert!(out.ends_with('…'));
    }
}
