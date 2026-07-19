// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use log::debug;
use slint::SharedString;

use crate::audio::Cmd;
use crate::data::config::{save_config, AppConfig};
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
) {
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
        // best-effort: audio worker may not be running yet
        let _ = if is_bn {
            tx.send(Cmd::BnVoice(want_voice.clone()))
        } else {
            tx.send(Cmd::Voice(want_voice.clone()))
        };
    }
    if voice_changed || lang_changed {
        save_config(cfg);
    }
    debug!(
        "voice-recall: mapped={} saved={} want={} changed={}",
        mapped,
        cfg.voices
            .get(mapped)
            .map(|s| s.as_str())
            .unwrap_or("(none)"),
        want_voice,
        voice_changed || lang_changed
    );
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
