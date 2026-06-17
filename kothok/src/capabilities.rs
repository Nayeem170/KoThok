use kobo_core::Capabilities;

use crate::device::{battery_pct, bt_name, bt_status, current_clock, wifi_name, wifi_status};

pub struct KoboCapabilities;

impl Capabilities for KoboCapabilities {
    fn network_available(&self) -> bool {
        wifi_status()
    }

    fn audio_sink_available(&self) -> bool {
        bt_status()
    }

    fn battery_pct(&self) -> i32 {
        battery_pct()
    }

    fn wifi_name(&self) -> Option<String> {
        wifi_name()
    }

    fn bt_name(&self) -> Option<String> {
        bt_name()
    }

    fn current_clock(&self) -> String {
        current_clock()
    }
}
