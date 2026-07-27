// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
//! Waveform + update-mode policy for whole-screen transition presents
//! (panel open/close, mode switch, chapter overlay).
//!
//! # Why this is a setting and not a constant
//!
//! Which combination clears the Kaleido 3 colour filter without a visible
//! blink is a property of the panel and the hwtcon driver, not of the code.
//! It cannot be decided off-device, and the only way to test one is a manual
//! USB deploy. Keeping the choice in the config file means the candidates can
//! be walked in one deploy instead of one rebuild each. The default is the
//! combination the hardware documentation says should win; the others exist so
//! a losing default can be corrected on the device.
//!
//! # MTK waveform numbering
//!
//! The `WAVE_*` constants follow the NXP mxcfb enum. The MTK hwtcon driver on
//! the Libra/Clara Colour numbers its waveforms differently: **4 is
//! GLR16/REAGL and 6 is A2**, so on this hardware [`WAVE_A2`] has always been
//! driving REAGL, and the `WAVE_GLR16` (5) / `WAVE_GLD16` (6) constants name
//! the wrong waveforms - 5 is GLD16 and 6 is A2 here. Only
//! [`WAVE_REAGL_MTK`] is correct for this SoC, which is why it is the one used
//! below.
//!
//! [`WAVE_A2`]: crate::rendering::fb::WAVE_A2
//! [`WAVE_REAGL_MTK`]: crate::rendering::fb::WAVE_REAGL_MTK

use crate::rendering::fb::{WAVE_GC16, WAVE_REAGL_MTK};

pub const KEY_REAGL: &str = "reagl";
pub const KEY_GC16_FULL: &str = "gc16_full";
pub const KEY_GC16_PARTIAL: &str = "gc16_partial";
pub const KEY_TWO_PASS: &str = "two_pass";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelTransition {
    /// REAGL (hwtcon GLR16) + FULL. REAGL is the ghost-suppression waveform:
    /// FULL re-drives every pixel in the region, but the waveform carries no
    /// inversion pass, so it clears without GC16's dark blink. REAGL requires
    /// the FULL update mode - with PARTIAL the driver has no whole-region pass
    /// to run the deghost over.
    Reagl,
    /// GC16 + FULL. The only combination proven on this panel to physically
    /// clear the colour filter, at the cost of the dark inversion blink.
    Gc16Full,
    /// GC16 + PARTIAL. No blink, but leaves colour residue (green buttons,
    /// the vinyl disk ring) because PARTIAL runs no clearing pass.
    Gc16Partial,
    /// Two PARTIAL passes: flat white, wait for it to land, then the panel.
    /// The white pass is the clearing pass GC16+PARTIAL skips, so no inversion
    /// blink - at roughly double the refresh time.
    TwoPass,
}

impl PanelTransition {
    /// Waveform for the transition present. `TwoPass` uses this for both of
    /// its passes.
    pub fn waveform(self) -> u32 {
        match self {
            PanelTransition::Reagl => WAVE_REAGL_MTK,
            PanelTransition::Gc16Full | PanelTransition::Gc16Partial | PanelTransition::TwoPass => {
                WAVE_GC16
            }
        }
    }

    /// `update_mode = 1` (FULL). Only FULL re-drives unchanged pixels; it is
    /// also what makes GC16 invert, which is the blink.
    pub fn full(self) -> bool {
        matches!(self, PanelTransition::Reagl | PanelTransition::Gc16Full)
    }

    /// Whether the present needs a white clearing pass before the content.
    pub fn needs_white_pass(self) -> bool {
        matches!(self, PanelTransition::TwoPass)
    }

    pub fn as_key(self) -> &'static str {
        match self {
            PanelTransition::Reagl => KEY_REAGL,
            PanelTransition::Gc16Full => KEY_GC16_FULL,
            PanelTransition::Gc16Partial => KEY_GC16_PARTIAL,
            PanelTransition::TwoPass => KEY_TWO_PASS,
        }
    }

    /// Parse a config value. An unknown value keeps the default rather than
    /// failing the load - a typo in the file must not stop the reader booting.
    pub fn from_key(val: &str) -> PanelTransition {
        match val.trim() {
            KEY_GC16_FULL => PanelTransition::Gc16Full,
            KEY_GC16_PARTIAL => PanelTransition::Gc16Partial,
            KEY_TWO_PASS => PanelTransition::TwoPass,
            KEY_REAGL => PanelTransition::Reagl,
            _ => PanelTransition::default(),
        }
    }
}

impl Default for PanelTransition {
    fn default() -> Self {
        PanelTransition::Reagl
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_reagl() {
        assert_eq!(PanelTransition::default(), PanelTransition::Reagl);
    }

    #[test]
    fn every_mode_round_trips_through_its_key() {
        for m in [
            PanelTransition::Reagl,
            PanelTransition::Gc16Full,
            PanelTransition::Gc16Partial,
            PanelTransition::TwoPass,
        ] {
            assert_eq!(PanelTransition::from_key(m.as_key()), m, "{m:?}");
        }
    }

    #[test]
    fn unknown_or_empty_key_falls_back_to_the_default() {
        assert_eq!(
            PanelTransition::from_key("glr16"),
            PanelTransition::default()
        );
        assert_eq!(PanelTransition::from_key(""), PanelTransition::default());
    }

    #[test]
    fn keys_tolerate_surrounding_whitespace() {
        assert_eq!(
            PanelTransition::from_key("  gc16_full "),
            PanelTransition::Gc16Full
        );
    }

    #[test]
    fn reagl_drives_the_mtk_waveform_not_the_nxp_a2_number() {
        // On MTK, 4 is GLR16/REAGL. Guards against "fixing" this to the
        // NXP-numbered WAVE_GLR16 (5), which is GLD16 on this driver.
        assert_eq!(PanelTransition::Reagl.waveform(), WAVE_REAGL_MTK);
        assert_eq!(WAVE_REAGL_MTK, 4);
    }

    #[test]
    fn only_reagl_and_gc16_full_use_the_full_update_mode() {
        assert!(PanelTransition::Reagl.full(), "REAGL deghost needs FULL");
        assert!(PanelTransition::Gc16Full.full());
        assert!(!PanelTransition::Gc16Partial.full());
        assert!(
            !PanelTransition::TwoPass.full(),
            "two-pass avoids FULL - the white pass is what clears"
        );
    }

    #[test]
    fn only_two_pass_wants_a_white_clearing_pass() {
        assert!(PanelTransition::TwoPass.needs_white_pass());
        for m in [
            PanelTransition::Reagl,
            PanelTransition::Gc16Full,
            PanelTransition::Gc16Partial,
        ] {
            assert!(!m.needs_white_pass(), "{m:?}");
        }
    }

    #[test]
    fn gc16_modes_share_a_waveform_and_differ_only_in_update_mode() {
        assert_eq!(
            PanelTransition::Gc16Full.waveform(),
            PanelTransition::Gc16Partial.waveform()
        );
        assert_ne!(
            PanelTransition::Gc16Full.full(),
            PanelTransition::Gc16Partial.full()
        );
    }
}
