// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::VERSION;

const RELEASES_URL: &str = "https://api.github.com/repos/Nayeem170/KoThok/releases/latest";
/// Retry floor for a check that never completed. A successful check latches
/// `CHECKED` and never runs again this launch, so this only paces retries after
/// a network or parse failure.
const RETRY_INTERVAL_SECS: u64 = 3600;

static LATEST: OnceLock<String> = OnceLock::new();
static CHECKED: AtomicBool = AtomicBool::new(false);
static LAST_CHECK: AtomicU64 = AtomicU64::new(0);
static ENABLED: AtomicBool = AtomicBool::new(false);

/// Apply the `check_updates` setting. Called once at startup, and again when
/// the setting changes, so turning it off takes effect without a relaunch.
///
/// Defaults to disabled until this is called: the check reaches the network, so
/// it should not be able to fire because a config load failed.
pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

pub fn try_check_if_wifi() {
    if !ENABLED.load(Ordering::Relaxed) || CHECKED.load(Ordering::Relaxed) {
        return;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if now.saturating_sub(LAST_CHECK.load(Ordering::Relaxed)) < RETRY_INTERVAL_SECS {
        return;
    }
    if !crate::device::wifi_status() {
        return;
    }
    LAST_CHECK.store(now, Ordering::Relaxed);
    std::thread::spawn(fetch_latest);
}

fn fetch_latest() {
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(10))
        .timeout_read(Duration::from_secs(15))
        .build();

    let resp = match agent.get(RELEASES_URL).set("User-Agent", "KoThok").call() {
        Ok(r) => r,
        Err(e) => {
            log::warn!("update-check: {e}");
            return;
        }
    };

    let body: serde_json::Value = match resp
        .into_string()
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(v) => v,
        None => {
            log::warn!("update-check: parse error");
            return;
        }
    };

    let tag = body.get("tag_name").and_then(|t| t.as_str()).unwrap_or("");
    let remote_ver = tag.strip_prefix('v').unwrap_or(tag);

    if is_newer(remote_ver, VERSION) {
        log::info!("update-check: newer version available: {tag}");
        let _ = LATEST.set(tag.to_string());
    } else {
        log::info!("update-check: up to date ({VERSION})");
    }
    CHECKED.store(true, Ordering::Relaxed);
}

pub fn pending_update() -> Option<&'static str> {
    LATEST.get().map(|s| s.as_str())
}

/// Is `remote` a strictly higher version than `local`?
///
/// A component that does not parse fails the whole comparison rather than
/// being skipped. Dropping it would shift every later component one place
/// left, so `1.x.5` would read as `[1, 5]` and compare *newer* than `1.2.3` --
/// announcing an update that does not exist. An unreadable tag is not evidence
/// of a newer release, so the answer is "no".
fn is_newer(remote: &str, local: &str) -> bool {
    fn parse(s: &str) -> Option<Vec<u32>> {
        s.split('.')
            .map(|p| {
                // Trailing pre-release metadata (`3-beta`) is dropped, but the
                // numeric part in front of it must still be there.
                p.split('-').next()?.parse::<u32>().ok()
            })
            .collect()
    }
    let (Some(r), Some(l)) = (parse(remote), parse(local)) else {
        log::warn!("update-check: unparseable version pair ({remote}, {local})");
        return false;
    };
    for i in 0..r.len().max(l.len()) {
        let rv = r.get(i).copied().unwrap_or(0);
        let lv = l.get(i).copied().unwrap_or(0);
        if rv != lv {
            return rv > lv;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::is_newer;

    #[test]
    fn higher_component_is_newer() {
        assert!(is_newer("0.4.0", "0.3.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
        assert!(is_newer("0.3.10", "0.3.9"));
    }

    #[test]
    fn same_or_older_is_not_newer() {
        assert!(!is_newer("0.3.0", "0.3.0"));
        assert!(!is_newer("0.2.9", "0.3.0"));
    }

    #[test]
    fn missing_components_count_as_zero() {
        assert!(is_newer("0.4", "0.3.9"));
        assert!(!is_newer("0.3", "0.3.0"));
        assert!(is_newer("0.3.1", "0.3"));
    }

    #[test]
    fn pre_release_suffix_is_ignored() {
        assert!(is_newer("0.4.0-beta", "0.3.0"));
        assert!(!is_newer("0.3.0-beta", "0.3.0"));
    }

    /// The regression this function was rewritten for: a dropped component
    /// used to shift the rest left and fake an update.
    #[test]
    fn unparseable_version_is_never_newer() {
        assert!(!is_newer("1.x.5", "1.2.3"));
        assert!(!is_newer("", "0.3.0"));
        assert!(!is_newer("latest", "0.3.0"));
        assert!(!is_newer("0.4.0", "not-a-version"));
    }
}
