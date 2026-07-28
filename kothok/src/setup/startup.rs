// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan

/// What the reader should land in on launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupTarget {
    /// Open the onboarding guide at chapter 0, page 0. The caller must override
    /// `initial_path` with `GUIDE_PATH`, set `onboarding_version = BUILD_TAG`
    /// in config, and save before the book opens so a crash mid-guide does not
    /// re-trap the reader.
    Onboarding,
    /// Show the library picker.
    Picker,
    /// Open a book (CLI path, last-read, or first available).
    Book,
}

/// Pure routing decision extracted from the old inline
/// `cli_path.is_none() && all_books.len() >= 2` branch.
///
/// Priority:
/// 1. CLI path - explicit user intent always wins.
/// 2. Onboarding - version mismatch + guide installed.
/// 3. Library picker - 2+ books on the shelf.
/// 4. Book - 0 or 1 books, open whatever is there.
pub fn startup_target(
    cli_path: &Option<String>,
    book_count: usize,
    seen_version: &str,
    build_tag: &str,
    guide_exists: bool,
) -> StartupTarget {
    if cli_path.is_some() {
        return StartupTarget::Book;
    }
    if guide_exists && seen_version != build_tag {
        return StartupTarget::Onboarding;
    }
    if book_count >= 2 {
        StartupTarget::Picker
    } else {
        StartupTarget::Book
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_path_always_wins() {
        let t = startup_target(&Some("x.epub".into()), 5, "v0.2.0", "v0.3.0", true);
        assert_eq!(t, StartupTarget::Book);
    }

    #[test]
    fn fresh_install_opens_guide() {
        let t = startup_target(&None, 3, "", "v0.2.0", true);
        assert_eq!(t, StartupTarget::Onboarding);
    }

    #[test]
    fn version_change_opens_guide() {
        let t = startup_target(&None, 3, "v0.2.0", "v0.3.0", true);
        assert_eq!(t, StartupTarget::Onboarding);
    }

    #[test]
    fn same_version_normal_routing_picker() {
        let t = startup_target(&None, 3, "v0.2.0", "v0.2.0", true);
        assert_eq!(t, StartupTarget::Picker);
    }

    #[test]
    fn same_version_normal_routing_book() {
        let t = startup_target(&None, 1, "v0.2.0", "v0.2.0", true);
        assert_eq!(t, StartupTarget::Book);
    }

    #[test]
    fn guide_missing_falls_through() {
        let t = startup_target(&None, 3, "", "v0.2.0", false);
        assert_eq!(t, StartupTarget::Picker);
    }

    #[test]
    fn downgrade_also_opens_guide() {
        let t = startup_target(&None, 3, "v0.3.0", "v0.2.0", true);
        assert_eq!(t, StartupTarget::Onboarding);
    }

    #[test]
    fn zero_books_no_guide_opens_book() {
        let t = startup_target(&None, 0, "v0.2.0", "v0.2.0", false);
        assert_eq!(t, StartupTarget::Book);
    }
}
