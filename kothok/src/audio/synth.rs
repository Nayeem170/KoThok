use kobo_core::audio::synthesize_prepared;
use kobo_core::audio::{EdgeTts, Player, TtsEvent, TARGET_RATE};
use log::debug;

use crate::meta::{LANG_AUTO, LANG_BN_BD, LANG_EN_US};

pub(crate) fn voice_for_text(text: &str, voice: &str, bn_voice: &str) -> (String, String) {
    let script = crate::rendering::text_render::detect_script(text);
    let lang = script.lang_tag();
    debug!("voice_detect: script={script:?} lang={lang}");

    let chosen = match lang {
        LANG_EN_US => voice.to_string(),
        LANG_BN_BD => bn_voice.to_string(),
        other => crate::panel::voices_for_lang(other)
            .first()
            .map(|v| v.id.to_string())
            .unwrap_or_else(|| voice.to_string()),
    };
    (chosen, lang.to_string())
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
    match lang {
        LANG_EN_US => (voice.to_string(), LANG_EN_US.to_string()),
        LANG_BN_BD => (bn_voice.to_string(), LANG_BN_BD.to_string()),
        other => {
            let v = crate::panel::voices_for_lang(other)
                .first()
                .map(|v| v.id.to_string())
                .unwrap_or_else(|| voice.to_string());
            (v, other.to_string())
        }
    }
}

pub(crate) async fn synth_prepare(
    utt_idx: usize,
    text: String,
    voice: String,
    rate: String,
    lang: String,
) -> Result<kobo_core::audio::Prepared, String> {
    synthesize_prepared(utt_idx, &text, &voice, &rate, &lang).await
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
        let expected = crate::panel::voices_for_lang("ar-SA")
            .first()
            .expect("ar voice")
            .id;
        assert_eq!(v, expected);
    }
}
