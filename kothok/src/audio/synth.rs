use kobo_core::audio::synthesize_prepared;
use log::debug;

use crate::meta::{LANG_AUTO, LANG_BN_BD};

pub(crate) fn voice_for_text(text: &str, voice: &str, bn_voice: &str) -> (String, String) {
    let script = crate::rendering::text_render::detect_script(text);
    let lang = script.lang_tag();
    debug!("voice_detect: script={script:?} lang={lang}");
    (pick_voice(&lang, voice, bn_voice), lang.to_string())
}

pub(crate) fn voice_for_text_explicit(
    text: &str,
    voice: &str,
    bn_voice: &str,
    lang: &str,
) -> (String, String) {
    if lang == LANG_AUTO {
        return voice_for_text(text, voice, bn_voice);
    }
    (pick_voice(lang, voice, bn_voice), lang.to_string())
}

fn pick_voice(lang: &str, voice: &str, bn_voice: &str) -> String {
    if lang == LANG_BN_BD {
        return bn_voice.to_string();
    }
    let voices = crate::panel::voices_for_lang(lang);
    voices
        .iter()
        .find(|v| v.id() == voice)
        .or_else(|| voices.first())
        .map(|v| v.id().to_string())
        .unwrap_or_else(|| voice.to_string())
}

pub(crate) async fn synth_prepare(
    utt_idx: usize,
    text: String,
    voice: String,
    rate: String,
    lang: String,
) -> Result<kobo_core::audio::Prepared, String> {
    let text = normalize_tts_text(&text);
    synthesize_prepared(utt_idx, &text, &voice, &rate, &lang).await
}

fn normalize_tts_text(text: &str) -> String {
    text.replace('\u{0964}', ". ")
        .replace('\u{0965}', ". ")
        .replace("\u{09AF}\u{09BC}", "\u{09DF}")
        .replace(['\u{201C}', '\u{201D}', '"', '\u{2018}', '\u{2019}'], "")
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn voice_for_english_text_uses_base_voice() {
        let (v, lang) = voice_for_text("Hello world", "en-US-EmmaNeural", "bn-BD-NabanitaNeural");
        assert_eq!(v, "en-US-EmmaNeural");
        assert_eq!(lang, "en-US");
    }

    #[test]
    fn voice_for_bangla_text_uses_bn_voice() {
        let (v, lang) = voice_for_text(
            "আমি বাংলায় কথা বলি",
            "en-US-EmmaNeural",
            "bn-BD-NabanitaNeural",
        );
        assert_eq!(v, "bn-BD-NabanitaNeural");
        assert_eq!(lang, "bn-BD");
    }

    #[test]
    fn voice_explicit_lang_overrides_detection() {
        let (v, lang) =
            voice_for_text_explicit("আমি", "en-US-EmmaNeural", "bn-BD-NabanitaNeural", "en-US");
        assert_eq!(v, "en-US-EmmaNeural");
        assert_eq!(lang, "en-US");
    }

    #[test]
    fn voice_explicit_auto_delegates_to_detection() {
        let (v, lang) =
            voice_for_text_explicit("Hello", "en-US-EmmaNeural", "bn-BD-NabanitaNeural", "auto");
        assert_eq!(v, "en-US-EmmaNeural");
        assert_eq!(lang, "en-US");
    }

    #[test]
    fn voice_for_arabic_picks_lang_voice() {
        let (v, lang) = voice_for_text("مرحبا بالعالم", "en-US-EmmaNeural", "bn-BD-NabanitaNeural");
        assert_eq!(lang, "ar-SA");
        let voices = crate::panel::voices_for_lang("ar-SA");
        let expected = voices
            .first()
            .expect("ar voice")
            .id()
            .to_string();
        assert_eq!(v, expected);
    }
}
