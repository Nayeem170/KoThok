# Offline TTS: Why It Is Not Available

KoThok requires an active WiFi connection for text-to-speech. There is no
offline fallback. This document records what was tried and why none of it
worked on the Kobo hardware.

## Hardware Constraint

The Kobo Libra Colour uses a low-power ARM CPU (~1 GHz, single-core). This
is sufficient for rendering and page-turn latency, but it cannot run neural
TTS inference in real time. The CPU is the bottleneck, not a software
limitation that optimisation can overcome.

## What Was Tried

### 1. espeak-ng (formant synthesizer)

espeak-ng is a lightweight rule-based TTS that runs in real time on any CPU.

- Built a static ARM binary via Docker/QEMU, deployed as a subprocess
- Audio was intelligible (full consonant synthesis, correct formants)
- Quality was unacceptable: robotic, mechanical voice that fatigues the
  listener over long sessions
- The pure-Rust port of espeak-ng was also tested but had broken consonant
  synthesis (0.6% spectral energy above 4 kHz vs 10-20% expected), making
  only 10-20% of words understandable

Verdict: runs in real time, but quality is too low for an audiobook reader.

### 2. Piper (neural TTS)

Piper uses VITS neural models via ONNX Runtime. Quality is near-human,
dramatically better than espeak-ng.

- Pre-compiled armv7l binary requires GLIBC 2.28+; the Kobo has GLIBC ~2.24
- Solved by bundling Ubuntu 22.04 glibc/libstdc++ and patchelf-ing the
  binary to use the bundled libraries
- Model loading (60 MB ONNX file) takes ~15 s on first sentence
- Inference runs at **5.5x real time**: every 3 s of speech takes ~16 s to
  synthesise
- This means 10-15 s gaps between sentences during playback

Verdict: quality is excellent, but the Kobo CPU is 5.5x too slow for
real-time neural inference. No model size or quality setting changes this
because all Piper models are architecturally similar.

### 3. Why Not a Smaller Model?

Piper "low" quality models are the same file size as "medium" (~60 MB). The
architecture is identical; only the training data differs. Even if a
significantly smaller model existed, the Kobo's CPU would still be too slow
for neural inference at any practical quality level.

## Conclusion

The Kobo's CPU cannot do real-time speech synthesis at acceptable quality.
Edge TTS (Microsoft's cloud neural voices) over WiFi is the only viable TTS
path for this device. When WiFi is unavailable, TTS is not available.

If offline TTS becomes a hard requirement in the future, the options are:

1. **Pre-generate audio on a faster device** (PC, phone) and transfer WAV
   files to the Kobo for playback
2. **Upgrade to a device with a faster CPU** (e.g., a phone running the
   mobile companion app)
3. **Accept espeak-ng quality** as a basic fallback for short snippets
   (menu navigation, status messages) where naturalness is less critical
