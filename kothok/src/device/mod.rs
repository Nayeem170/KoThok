// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
pub mod font_download;
pub mod hw;

pub use kobo_core::device::{
    battery, bt, clock, fonts, input, media_keys, power, registry, touch, wake, wifi,
};

pub use battery::*;
pub use bt::*;
pub use clock::*;
pub use wifi::*;

/// Identity of the binary that is actually running, as "7572k/183044".
///
/// `BUILD_TAG` is `concat!("v", CARGO_PKG_VERSION)`, which is the same string
/// for every build of a version. The settings panel showed it next to
/// `VERSION` -- the same number twice -- so nothing on the device could tell a
/// fresh deploy from a stale one. Four already-fixed defects were reported as
/// unfixed for exactly that reason: there was no way to see which binary was
/// answering.
///
/// Both numbers are read from the running executable rather than from a file
/// written beside it, because a stamp written by the deploy script would stay
/// current even when the binary it describes did not land. Size catches a
/// changed build; mtime catches a rebuild that happened to land on the same
/// size. `deploy.ps1` prints the same pair for the binary it copied, so the two
/// can be compared directly.
pub fn build_stamp() -> String {
    static STAMP: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    STAMP
        .get_or_init(|| {
            let meta = std::env::current_exe().and_then(std::fs::metadata);
            let Ok(meta) = meta else {
                return "unreadable".to_string();
            };
            let secs = meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            // Truncated to six digits: enough to separate any two builds made
            // within eleven days, short enough to sit on one line. The kc: suffix
            // is the compile-time kobo-core version (read from Cargo.lock by
            // build.rs) so the stamp self-describes which dependency is compiled in.
            format!(
                "{}k/{} kc:{}",
                meta.len() / 1024,
                secs % 1_000_000,
                env!("KOBO_CORE_REV")
            )
        })
        .clone()
}

/// Free and total space on the book partition, as a short human label
/// ("12.1 GB / 28.9 GB").
///
/// Reported for the partition the reader's own books and caches live on, not
/// the root filesystem: that is the number that decides whether another book
/// fits, and it is the one the About page quoted.
pub fn free_space_label() -> Option<String> {
    let path = std::ffi::CString::new(crate::data::config::BOOK_DIR).ok()?;
    // SAFETY: `statvfs` writes the caller-owned struct through the out-pointer
    // and reads a NUL-terminated path that `CString` guarantees. Nothing is
    // retained past the call.
    let stat = unsafe {
        let mut s: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(path.as_ptr(), &mut s) != 0 {
            return None;
        }
        s
    };
    let free_bytes = stat.f_bavail as u64 * stat.f_frsize as u64;
    let total_bytes = stat.f_blocks as u64 * stat.f_frsize as u64;
    Some(storage_label(free_bytes, total_bytes))
}

fn storage_label(free_bytes: u64, total_bytes: u64) -> String {
    format!("{} / {}", size_label(free_bytes), size_label(total_bytes))
}

fn size_label(bytes: u64) -> String {
    let gb = bytes as f64 / 1_000_000_000.0;
    if gb >= 1.0 {
        format!("{gb:.1} GB")
    } else {
        format!("{:.0} MB", bytes as f64 / 1_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::storage_label;

    #[test]
    fn both_sides_in_gb() {
        assert_eq!(
            storage_label(12_100_000_000, 28_900_000_000),
            "12.1 GB / 28.9 GB"
        );
    }

    #[test]
    fn free_below_one_gb_uses_mb_total_stays_gb() {
        assert_eq!(
            storage_label(820_000_000, 28_900_000_000),
            "820 MB / 28.9 GB"
        );
    }

    #[test]
    fn zero_free_space() {
        assert_eq!(storage_label(0, 28_900_000_000), "0 MB / 28.9 GB");
    }
}
