// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::SharedString;

use crate::audio::glue::best_effort_send;
use crate::audio::Cmd;
use crate::data::config::AppConfig;
use crate::Reader;

pub const LANG_AUTO: &str = "auto";
pub const LANG_EN_US: &str = "en-US";
pub const LANG_BN_BD: &str = "bn-BD";

pub(crate) const VOICE: &str = crate::panel::DEFAULT_VOICE_EN;
pub(crate) const BN_VOICE: &str = crate::panel::DEFAULT_VOICE_BN;

pub fn is_rtl(lang: Option<&str>) -> bool {
    kobo_core::rendering::common::lang_is_rtl(lang)
}

pub fn apply_book_voice(
    cfg: &mut AppConfig,
    book_lang: Option<&str>,
    reader: &Reader,
    cmd_tx: Option<&std::sync::mpsc::Sender<Cmd>>,
) -> bool {
    let lang = book_lang.unwrap_or(LANG_EN_US);
    let mapped = crate::panel::normalize_lang(lang);
    let voices = crate::panel::voices_for_lang(mapped);
    let is_bn = mapped == LANG_BN_BD;
    let want_voice = cfg
        .voices
        .get(mapped)
        .cloned()
        .or_else(|| voices.first().map(|v| v.id().to_string()))
        .unwrap_or_else(|| {
            if is_bn {
                BN_VOICE.to_string()
            } else {
                VOICE.to_string()
            }
        });
    let lang_changed = cfg.tts_lang != mapped;
    let voice_changed = cfg.tts_voice != want_voice;
    cfg.tts_lang = mapped.to_string();
    cfg.tts_voice = want_voice.clone();
    reader.set_tts_lang(SharedString::from(mapped));
    reader.set_tts_voice(SharedString::from(&want_voice));
    reader.set_tts_voice_label(SharedString::from(crate::panel::voice_label(&want_voice)));
    if let Some(tx) = cmd_tx {
        let cmd = if is_bn {
            Cmd::BnVoice(want_voice.clone())
        } else {
            Cmd::Voice(want_voice.clone())
        };
        best_effort_send(tx, cmd);
    }
    voice_changed || lang_changed
}

pub const SAMPLE_CHAPTER: &str = r#"<html><body>
<h1>The Quick Brown Fox</h1>
<p>The quick brown fox jumps over the lazy dog. This is sample chapter text for
the Read Aloud reader on the Kobo Libra Colour. Each sentence is highlighted
with a left accent bar as the playback clock advances.</p>
<figure><img src="fox.png" alt="A fox"/>
  <figcaption>Fig. 1 - the fox leaps.</figcaption></figure>
<p>Highlight and audio read one shared clock, so they cannot drift. Use the
previous and next buttons to jump by sentence. Drag the seek bar to scrub.</p>
<p>The real Player drives the clock from the A2DP audio sink; here we render a
static page to validate legibility on the Kaleido 3 panel.</p>
</body></html>"#;

pub(crate) fn has_bangla(s: &str) -> bool {
    crate::rendering::text_render::detect_script(s)
        == crate::rendering::text_render::Script::Bengali
}

pub(crate) fn clean_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

const PANEL_COVER_W: usize = 200;
const PANEL_PAD: usize = 24;
const PANEL_COL_GAP: usize = 20;
/// Width of the full-width text column in the panel: everything left of the
/// right edge once the cover and the padding are taken out. Correct for rows
/// that own the whole column, which is the chapter name and nothing else.
fn panel_text_w() -> usize {
    crate::w()
        .saturating_sub(PANEL_COVER_W + 2 * PANEL_PAD + PANEL_COL_GAP)
        .max(120)
}

/// The page badge in `control_panel.slint` and the gap before it.
const PANEL_BADGE_W: usize = 160;
const PANEL_BADGE_GAP: usize = 12;

/// Width available to the **title and author**, which share their row with the
/// fixed-width page badge.
///
/// Rendering those at `panel_text_w()` was wrong by exactly the badge: a
/// `text_image` raster is as wide as its longest line, so a long title produced
/// a picture ~170px wider than the slot it was placed in. That widened the row,
/// which widened the column, which pushed the badge off the right edge and
/// cropped the progress bar underneath it. Latin books mostly escaped it
/// because they take the plain `Text` path, which wraps.
fn panel_title_w() -> usize {
    panel_text_w()
        .saturating_sub(PANEL_BADGE_W + PANEL_BADGE_GAP)
        .max(120)
}

/// Type size of the panel's header line, matching the `font-size: 33px` in
/// `control_panel.slint`.
const PANEL_HEAD_PX: f32 = 33.0;
/// Everything the panel header spends on things that are not the title: the
/// two 23px edge insets and the 76px close button, with room to spare so a long
/// title elides instead of colliding.
const PANEL_HEAD_RESERVE: usize = 320;
/// Baked into the header raster rather than drawn as a sibling Text -- see
/// [`set_book_meta`].
const PANEL_HEAD_SUFFIX: &str = "Settings";
/// Gap between the elided title and the "Settings" suffix, in px.
const PANEL_HEAD_GAP: usize = 10;
/// Slack past the measured extent of a run, in px. `measure_text` sums shaped
/// advances and the blit lands within a pixel or two of that; this covers the
/// difference so a glyph's last column is never the one outside the buffer.
const PANEL_HEAD_PAD: usize = 4;

/// Where each part of the header raster goes, and how wide the raster is.
///
/// Split out from the drawing so the invariant can be checked without a
/// framebuffer: `suffix_x + suffix width <= img_w` means the suffix is inside
/// the buffer, for every title, on every panel.
struct PanelHead {
    /// The title as it will be drawn: elided, possibly empty.
    title: String,
    /// x of the suffix within the raster.
    suffix_x: usize,
    /// Width of the raster.
    img_w: usize,
}

fn panel_head_layout(title: &str, head_w: usize) -> PanelHead {
    use crate::rendering::draw::measure_text;
    // Titles come out of EPUB metadata, where a line break or a run of spaces
    // inside the element is ordinary. Left in, they reach the renderer as real
    // whitespace and take horizontal room nothing accounted for.
    let title = clean_ws(title);
    let suffix_w = panel_head_suffix_w();
    let budget = head_w.saturating_sub(suffix_w + PANEL_HEAD_GAP);
    let fitted = elide_to_width(&title, budget);
    let suffix_x = if fitted.is_empty() {
        0
    } else {
        measure_text(&fitted, PANEL_HEAD_PX) + PANEL_HEAD_PAD + PANEL_HEAD_GAP
    };
    PanelHead {
        title: fitted,
        suffix_x,
        img_w: (suffix_x + suffix_w).max(1),
    }
}

fn panel_head_suffix_w() -> usize {
    crate::rendering::draw::measure_text(PANEL_HEAD_SUFFIX, PANEL_HEAD_PX) + PANEL_HEAD_PAD
}

/// The panel header raster: the book's title, then "Settings".
///
/// Composed here rather than by handing `text_image` one string, because that
/// path could not keep the suffix. `text_image` wraps and then keeps only the
/// first line, and the suffix is at the end, so any title that wrapped rendered
/// as the title alone with the word naming the screen silently gone.
///
/// Reserving width for the suffix and eliding the title first is not enough on
/// its own: it predicts what the wrapper will do, and the prediction is wrong
/// in two ways at once. `word_wrap_char_based_styled` opens with
/// `max_w.saturating_sub(12)`, twelve pixels `measure_text` knows nothing
/// about; and Bangla does not use word spacing, so it takes the character-based
/// wrap and may break at *any* character -- including immediately before
/// "Settings".
///
/// So the suffix is not left to a wrap decision at all. It is blitted at an x
/// this function computes, into a buffer this function sized to contain it.
/// Only the title can lose characters, and it loses them against its own
/// budget. There is no input for which the suffix does not fit, because its
/// position is not a function of the title's length -- the buffer is.
fn render_panel_head(title: &str, head_w: usize) -> (slint::Image, u32) {
    use crate::rendering::common::rgb565_as_bytes;
    use crate::rendering::text_render;
    use slint::platform::software_renderer::Rgb565Pixel;

    // Titles come out of EPUB metadata, where a line break or a run of spaces
    // inside the element is ordinary. Left in, they reach the wrapper as real
    // whitespace and can break the line on their own.
    let PanelHead {
        title: fitted,
        suffix_x,
        img_w,
    } = panel_head_layout(title, head_w);

    // blit_rgb565 takes the top of the run's own line box, not the baseline.
    // Two scripts on the same oy sit on two different baselines because each
    // face has its own ascent. Sink each run by (max_ascent - own_ascent) so the
    // baselines coincide, and grow the buffer by the same amount so the sunk
    // run is not clipped.
    let title_asc = text_render::ascent_for(text_render::detect_script(&fitted), PANEL_HEAD_PX);
    let suffix_asc =
        text_render::ascent_for(text_render::detect_script(PANEL_HEAD_SUFFIX), PANEL_HEAD_PX);
    let baseline = title_asc.max(suffix_asc);
    let h = text_render::line_height(PANEL_HEAD_PX).max(1);
    let h = h + baseline - title_asc.min(suffix_asc);

    let mut buf = vec![Rgb565Pixel(0xFFFF); img_w * h];
    let bytes = rgb565_as_bytes(&mut buf);
    if !fitted.is_empty() {
        text_render::blit_rgb565(
            bytes,
            img_w,
            &fitted,
            PANEL_HEAD_PX,
            0,
            baseline - title_asc,
            img_w,
            h,
        );
    }
    text_render::blit_rgb565(
        bytes,
        img_w,
        PANEL_HEAD_SUFFIX,
        PANEL_HEAD_PX,
        suffix_x,
        baseline - suffix_asc,
        img_w,
        h,
    );

    let mut rgb: Vec<u8> = Vec::with_capacity(img_w * h * 3);
    for p in &buf {
        let v = p.0;
        rgb.push((((v >> 11) & 0x1f) << 3) as u8);
        rgb.push((((v >> 5) & 0x3f) << 2) as u8);
        rgb.push(((v & 0x1f) << 3) as u8);
    }
    let pb = slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(
        &rgb,
        img_w as u32,
        h as u32,
    );
    (slint::Image::from_rgb8(pb), h as u32)
}

/// The longest prefix of `s` that draws within `max_w`, ellipsised.
///
/// Per character rather than per word: `draw::truncate_to_width` cuts on word
/// boundaries and so cannot shorten a string that has none -- handed a single
/// long token it returns it whole, still over the limit. Bangla written without
/// spaces is exactly that case.
fn elide_to_width(s: &str, max_w: usize) -> String {
    use crate::rendering::draw::measure_text;
    let fits = |t: &str| measure_text(t, PANEL_HEAD_PX) + PANEL_HEAD_PAD <= max_w;
    if max_w == 0 {
        return String::new();
    }
    if fits(s) {
        return s.to_string();
    }
    let mut best = String::new();
    for (i, _) in s.char_indices().skip(1) {
        let candidate = format!("{}...", s[..i].trim_end());
        if fits(&candidate) {
            best = candidate;
        } else {
            break;
        }
    }
    best
}

pub fn set_book_meta(reader: &Reader, title: &str, author: &str) {
    reader.set_book_title(SharedString::from(title));
    reader.set_book_author(SharedString::from(author));

    const HEADER_TITLE_FONT_PX: f32 = 33.0;
    let header_w = (crate::w() as i32 - 686).max(100) as f32;
    let avg_char_w = HEADER_TITLE_FONT_PX * 0.6;
    let est_w = title.len() as f32 * avg_char_w;
    let overflow = !title.is_empty() && est_w > header_w;
    reader.set_title_overflow(overflow);

    let img_w = panel_title_w();
    if !title.is_empty() && has_bangla(title) {
        let (img, h) = crate::rendering::render::text_image(title, 24.0, img_w, 2);
        reader.set_book_title_img(img);
        reader.set_book_title_img_h(h as i32);
    } else {
        reader.set_book_title_img(slint::Image::default());
        reader.set_book_title_img_h(0);
    }

    // The settings header is rasterised for **every** book, Latin included --
    // not just the ones Slint cannot draw.
    //
    // Rendering Bangla as a picture and Latin as a Slint `Text` put two
    // different engines on the same line, each with its own font and its own
    // baseline inside its line box. Two boxes can be centred on each other but
    // two baselines cannot, so the Bangla title always sat off the line the
    // English one sat on, and matching the geometry did not (and could not)
    // fix it. One engine for both means the two cannot disagree.
    //
    // The " Settings" suffix rides inside the same raster for the same reason.
    if title.is_empty() {
        reader.set_book_title_head_img(slint::Image::default());
        reader.set_book_title_head_img_h(0);
    } else {
        let head_w = crate::w().saturating_sub(PANEL_HEAD_RESERVE).max(120);
        let (head_img, head_h) = render_panel_head(title, head_w);
        reader.set_book_title_head_img(head_img);
        reader.set_book_title_head_img_h(head_h as i32);
    }
    if !author.is_empty() && has_bangla(author) {
        let (img, h) = crate::rendering::render::text_image(author, 20.0, img_w, 1);
        reader.set_book_author_img(img);
        reader.set_book_author_img_h(h as i32);
    } else {
        reader.set_book_author_img(slint::Image::default());
        reader.set_book_author_img_h(0);
    }
}

/// Side padding around the audio screen's chapter caption. Must match the
/// `x: 140px; width: root.width - 280px` in `audio_player.slint`.
const AUDIO_CAPTION_PAD: usize = 280;
/// Type size of that caption, matching the `font-size: 40px` it stands in for.
const AUDIO_CAPTION_PX: f32 = 40.0;

pub(crate) fn set_chapter_name(reader: &Reader, name: &str) {
    let name = clean_ws(name);
    reader.set_chapter_name(SharedString::from(&name));
    if !name.is_empty() && has_bangla(&name) {
        let (img, h) = crate::rendering::render::text_image(&name, 22.0, panel_text_w(), 1);
        reader.set_chapter_name_img(img);
        reader.set_chapter_name_img_h(h as i32);
        // The audio screen sets the same name at nearly twice the size, and it
        // gets its own raster rather than scaling this one up. The control
        // panel binds its Image to the picture's *natural* height, so raising
        // the shared render to suit the audio screen would silently enlarge the
        // caption in the panel; scaling a 22px raster up to 40px instead just
        // renders it soft. Rendering twice costs one extra text_image per
        // chapter change, and only for Bangla.
        let (hero, hero_h) = crate::rendering::render::text_image(
            &name,
            AUDIO_CAPTION_PX,
            crate::w().saturating_sub(AUDIO_CAPTION_PAD).max(120),
            1,
        );
        reader.set_chapter_name_hero_img(hero);
        reader.set_chapter_name_hero_img_h(hero_h as i32);
    } else {
        reader.set_chapter_name_img(slint::Image::default());
        reader.set_chapter_name_img_h(0);
        reader.set_chapter_name_hero_img(slint::Image::default());
        reader.set_chapter_name_hero_img_h(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE ON BANGLA IN THESE TESTS
    //
    // Faces are loaded from `/mnt/onboard/.adds/fonts`, which does not exist
    // off-device, so a Bengali string measured here is measured as `.notdef`
    // boxes and its width bears no relation to the device's. That is why
    // earlier attempts at this passed their tests and still cropped on the
    // Kobo: they predicted a width, and the prediction was made against the
    // wrong glyphs.
    //
    // What these tests check is therefore not a width but an identity --
    // `img_w == suffix_x + suffix_w` -- which holds for whatever the font
    // turns out to be, because `panel_head_layout` measures at runtime and
    // sizes the buffer from the result rather than the other way round.

    /// Titles long enough to have wrapped the old single-line raster, in both
    /// scripts, plus unbreakable single tokens that word-boundary truncation
    /// cannot shorten, plus the whitespace EPUB metadata really carries.
    const HARD_TITLES: &[&str] = &[
        "",
        "Aleph",
        "The Book of Tomorrow",
        "A Short History of Nearly Everything That Has Ever Happened Anywhere",
        "বাংলাদেশের মুক্তিযুদ্ধ",
        "বাংলাদেশের মুক্তিযুদ্ধের ইতিহাস এবং সংগ্রামের কাহিনী একত্রে",
        "Supercalifragilisticexpialidociousandthensomemoreletters",
        "অতিদীর্ঘএকটিশব্দযাকখনোভাঙানোযাবেনা",
        "  Spaced   Out  ",
        "Broken\nAcross\nLines",
        "Tab\tSeparated\tTitle",
    ];

    /// "Settings" names the screen. Its position is computed, not wrapped, so
    /// there is no title for which it lands outside the raster -- the buffer is
    /// sized from where it goes.
    #[test]
    fn settings_always_fits_inside_the_raster() {
        let suffix_w = panel_head_suffix_w();
        for &screen_w in &[600usize, 758, 1072, 1080, 1264, 1404, 1440] {
            let head_w = screen_w.saturating_sub(PANEL_HEAD_RESERVE).max(120);
            for &title in HARD_TITLES {
                let head = panel_head_layout(title, head_w);
                assert!(
                    head.suffix_x + suffix_w <= head.img_w,
                    "w={screen_w} {title:?}: suffix at {} + {suffix_w} exceeds raster {}",
                    head.suffix_x,
                    head.img_w
                );
                assert!(
                    head.img_w <= head_w,
                    "w={screen_w} {title:?}: raster {} wider than the {head_w}px slot",
                    head.img_w
                );
            }
        }
    }

    /// The title is what gives way, and only as far as it must.
    #[test]
    fn short_titles_are_left_alone() {
        let head_w = 1264usize - PANEL_HEAD_RESERVE;
        assert_eq!(panel_head_layout("Aleph", head_w).title, "Aleph");
    }

    /// Whitespace inside EPUB metadata is collapsed before it can take room or
    /// break a line.
    #[test]
    fn metadata_whitespace_is_collapsed() {
        let head_w = 1264usize - PANEL_HEAD_RESERVE;
        assert_eq!(
            panel_head_layout("Broken\nAcross\nLines", head_w).title,
            "Broken Across Lines"
        );
        assert_eq!(
            panel_head_layout("  Spaced   Out  ", head_w).title,
            "Spaced Out"
        );
    }

    /// A token with no space in it still has to shorten.
    #[test]
    fn unbreakable_titles_still_elide() {
        let head_w = 600usize - PANEL_HEAD_RESERVE;
        let t = panel_head_layout("Supercalifragilisticexpialidociousandthensome", head_w).title;
        assert!(t.ends_with("..."), "got {t:?}");
        assert!(t.len() < 45, "got {t:?}");
    }
}
