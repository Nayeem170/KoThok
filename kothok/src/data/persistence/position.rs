// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::fs;
use std::path::Path;

use crate::ViewMode;

pub use crate::data::config::POSITIONS_FILE;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bookmark {
    pub chapter: usize,
    pub page: usize,
    pub offset: usize,
}

pub struct ReadingPosition {
    pub chapter: usize,
    pub page: usize,
    pub cur_start: usize,
    pub cur_end: usize,
    pub view_mode: ViewMode,
    pub bookmark: Option<Bookmark>,
    /// Fraction of the book read, 0..1, as the reader knew it when this
    /// position was saved.
    ///
    /// Stored rather than recomputed because the library cannot recompute it
    /// reliably: deriving progress needs the per-chapter page offsets, and that
    /// cache is keyed by font size and discarded whenever a layout change
    /// repaginates. Every such invalidation used to drop a part-read book back
    /// to "0 %" until it was opened again. The reader always knows the true
    /// figure at save time, so it writes it down.
    pub progress: f32,
}

fn format_bookmark(bm: Option<Bookmark>) -> String {
    match bm {
        Some(b) => format!("{}:{}:{}", b.chapter, b.page, b.offset),
        None => "0:0:0".into(),
    }
}

fn format_mode(mode: ViewMode) -> char {
    match mode {
        ViewMode::Reading => 'r',
        ViewMode::Audio => 'a',
    }
}

pub fn save_position(file: &Path, book_path: &str, pos: &ReadingPosition) {
    let mut lines: Vec<String> = fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with(book_path))
        .map(String::from)
        .collect();
    lines.push(format!(
        "{}|{}|{}|{}|{}|{}|{}|{:.4}",
        book_path,
        pos.chapter,
        pos.page,
        pos.cur_start,
        pos.cur_end,
        format_bookmark(pos.bookmark),
        format_mode(pos.view_mode),
        pos.progress.clamp(0.0, 1.0),
    ));
    // best-effort: a failed position write only loses resume state, not the read
    let _ = fs::write(file, lines.join("\n"));
}

fn parse_bookmark(s: &str) -> Option<Bookmark> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() == 3 {
        let ch = parts[0].parse().ok()?;
        let pg = parts[1].parse().ok()?;
        let off = parts[2].parse().ok()?;
        if ch == 0 && pg == 0 && off == 0 {
            return None;
        }
        return Some(Bookmark {
            chapter: ch,
            page: pg,
            offset: off,
        });
    }
    None
}

fn parse_mode(s: Option<&str>) -> ViewMode {
    match s {
        Some("a") => ViewMode::Audio,
        _ => ViewMode::Reading,
    }
}

pub fn load_position(file: &Path, book_path: &str) -> Option<ReadingPosition> {
    let data = fs::read_to_string(file).ok()?;
    for line in data.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() >= 5 && parts[0] == book_path {
            let ch = parts[1].parse().ok()?;
            let pg = parts[2].parse().ok()?;
            let cs = parts[3].parse().ok()?;
            let ce = parts[4].parse().ok()?;
            let bookmark = parts.get(5).and_then(|s| parse_bookmark(s));
            let view_mode = parse_mode(parts.get(6).copied());
            // Field 8 is newer than the format; lines written before it exists
            // simply have no stored progress, and the caller falls back to
            // deriving it from the offset cache.
            let progress = parts
                .get(7)
                .and_then(|s| s.trim().parse::<f32>().ok())
                .unwrap_or(0.0)
                .clamp(0.0, 1.0);
            return Some(ReadingPosition {
                chapter: ch,
                page: pg,
                cur_start: cs,
                cur_end: ce,
                view_mode,
                bookmark,
                progress,
            });
        }
    }
    None
}

pub fn last_book_path(file: &Path) -> Option<String> {
    let data = fs::read_to_string(file).ok()?;
    data.lines()
        .next_back()
        .map(|line| line.split('|').next().unwrap_or_default().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn position_roundtrip() {
        let dir = std::env::temp_dir().join("kothok_test_pos_roundtrip");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("positions");
        save_position(
            &file,
            "/mnt/onboard/Book.epub",
            &ReadingPosition {
                chapter: 3,
                page: 7,
                cur_start: 150,
                cur_end: 200,
                view_mode: ViewMode::Reading,
                bookmark: None,
                progress: 0.0,
            },
        );
        let pos = load_position(&file, "/mnt/onboard/Book.epub").unwrap();
        assert_eq!(pos.chapter, 3);
        assert_eq!(pos.page, 7);
        assert_eq!(pos.cur_start, 150);
        assert_eq!(pos.cur_end, 200);
        assert_eq!(pos.view_mode, ViewMode::Reading);
        assert!(pos.bookmark.is_none());
    }

    #[test]
    fn position_save_overwrites_previous() {
        let dir = std::env::temp_dir().join("kothok_test_pos_overwrite");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("positions");
        save_position(
            &file,
            "/mnt/onboard/Book.epub",
            &ReadingPosition {
                chapter: 1,
                page: 2,
                cur_start: 10,
                cur_end: 20,
                view_mode: ViewMode::Reading,
                bookmark: None,
                progress: 0.0,
            },
        );
        save_position(
            &file,
            "/mnt/onboard/Book.epub",
            &ReadingPosition {
                chapter: 5,
                page: 9,
                cur_start: 100,
                cur_end: 200,
                view_mode: ViewMode::Audio,
                bookmark: Some(Bookmark {
                    chapter: 5,
                    page: 3,
                    offset: 42,
                }),
                progress: 0.0,
            },
        );
        let pos = load_position(&file, "/mnt/onboard/Book.epub").unwrap();
        assert_eq!(pos.chapter, 5);
        assert_eq!(pos.page, 9);
        assert_eq!(pos.view_mode, ViewMode::Audio);
        assert_eq!(
            pos.bookmark,
            Some(Bookmark {
                chapter: 5,
                page: 3,
                offset: 42
            })
        );
    }

    #[test]
    fn position_multiple_books() {
        let dir = std::env::temp_dir().join("kothok_test_pos_multi");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("positions");
        save_position(
            &file,
            "/mnt/onboard/A.epub",
            &ReadingPosition {
                chapter: 1,
                page: 2,
                cur_start: 10,
                cur_end: 20,
                view_mode: ViewMode::Reading,
                bookmark: None,
                progress: 0.0,
            },
        );
        save_position(
            &file,
            "/mnt/onboard/B.epub",
            &ReadingPosition {
                chapter: 3,
                page: 4,
                cur_start: 30,
                cur_end: 40,
                view_mode: ViewMode::Reading,
                bookmark: None,
                progress: 0.0,
            },
        );
        let a = load_position(&file, "/mnt/onboard/A.epub").unwrap();
        assert_eq!(a.chapter, 1);
        let b = load_position(&file, "/mnt/onboard/B.epub").unwrap();
        assert_eq!(b.chapter, 3);
        assert!(load_position(&file, "/mnt/onboard/C.epub").is_none());
    }

    #[test]
    fn last_book_path_returns_last_saved() {
        let dir = std::env::temp_dir().join("kothok_test_last_book");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("positions");
        save_position(
            &file,
            "/mnt/onboard/First.epub",
            &ReadingPosition {
                chapter: 0,
                page: 0,
                cur_start: 0,
                cur_end: 0,
                view_mode: ViewMode::Reading,
                bookmark: None,
                progress: 0.0,
            },
        );
        save_position(
            &file,
            "/mnt/onboard/Second.epub",
            &ReadingPosition {
                chapter: 1,
                page: 1,
                cur_start: 10,
                cur_end: 20,
                view_mode: ViewMode::Reading,
                bookmark: None,
                progress: 0.0,
            },
        );
        assert_eq!(
            last_book_path(&file),
            Some("/mnt/onboard/Second.epub".into())
        );
    }

    #[test]
    fn last_book_path_empty_file() {
        let dir = std::env::temp_dir().join("kothok_test_last_empty");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("positions");
        let _ = std::fs::write(&file, "");
        assert!(last_book_path(&file).is_none());
    }

    #[test]
    fn bookmark_roundtrip() {
        let dir = std::env::temp_dir().join("kothok_test_bm_roundtrip");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("positions");
        save_position(
            &file,
            "/mnt/onboard/X.epub",
            &ReadingPosition {
                chapter: 2,
                page: 5,
                cur_start: 100,
                cur_end: 150,
                view_mode: ViewMode::Audio,
                bookmark: Some(Bookmark {
                    chapter: 2,
                    page: 3,
                    offset: 80,
                }),
                progress: 0.0,
            },
        );
        let pos = load_position(&file, "/mnt/onboard/X.epub").unwrap();
        assert_eq!(
            pos.bookmark,
            Some(Bookmark {
                chapter: 2,
                page: 3,
                offset: 80
            })
        );
        assert_eq!(pos.view_mode, ViewMode::Audio);
    }

    #[test]
    fn backward_compat_old_format_no_bookmark() {
        let dir = std::env::temp_dir().join("kothok_test_bm_compat");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("positions");
        let _ = fs::write(&file, "/mnt/onboard/Old.epub|3|7|150|200\n");
        let pos = load_position(&file, "/mnt/onboard/Old.epub").unwrap();
        assert_eq!(pos.chapter, 3);
        assert_eq!(pos.page, 7);
        assert!(pos.bookmark.is_none());
        assert_eq!(pos.view_mode, ViewMode::Reading);
    }
}

#[cfg(test)]
mod progress_tests;
