// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::fs;
use std::path::Path;

pub fn load_marks(marks_file: &Path, book_path: &str) -> Vec<crate::data::mark::Mark> {
    let data = match fs::read_to_string(marks_file) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };
    let mut marks = Vec::new();
    for line in data.lines() {
        let parts: Vec<&str> = line.splitn(8, '|').collect();
        if parts.len() < 8 || parts[0] != book_path {
            continue;
        }
        let kind = match parts[1] {
            "b" => crate::data::mark::MarkKind::Bookmark,
            "h" => crate::data::mark::MarkKind::Highlight,
            _ => continue,
        };
        let chapter = parts[2].parse().unwrap_or(0);
        let start = parts[3].parse().unwrap_or(0);
        let end = parts[4].parse().unwrap_or(0);
        let page_hint = parts[5].parse().unwrap_or(0);
        let created = parts[6].parse().unwrap_or(0);
        let excerpt = parts[7].to_string();
        marks.push(crate::data::mark::Mark {
            kind,
            chapter,
            start,
            end,
            page_hint,
            created,
            excerpt,
        });
    }
    marks
}

pub fn save_marks(marks_file: &Path, book_path: &str, marks: &[crate::data::mark::Mark]) {
    let mut lines: Vec<String> = fs::read_to_string(marks_file)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with(book_path))
        .map(String::from)
        .collect();
    for m in marks {
        let kind = match m.kind {
            crate::data::mark::MarkKind::Bookmark => "b",
            crate::data::mark::MarkKind::Highlight => "h",
        };
        lines.push(format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            book_path, kind, m.chapter, m.start, m.end, m.page_hint, m.created, m.excerpt
        ));
    }
    let _ = fs::write(marks_file, lines.join("\n"));
}

pub fn migrate_bookmark(
    marks_file: &Path,
    positions_file: &Path,
    book_path: &str,
    chapter_count: usize,
    marks: &mut Vec<crate::data::mark::Mark>,
    now: u64,
) {
    let bm_field = crate::data::persistence::load_bookmark_field(positions_file, book_path);
    let bm = match bm_field {
        Some(b) => b,
        None => return,
    };
    if bm.chapter >= chapter_count {
        return;
    }
    let already = marks
        .iter()
        .any(|m| m.kind == crate::data::mark::MarkKind::Bookmark && m.chapter == bm.chapter);
    if already {
        return;
    }
    marks.push(crate::data::mark::Mark {
        kind: crate::data::mark::MarkKind::Bookmark,
        chapter: bm.chapter,
        start: bm.offset,
        end: bm.offset,
        page_hint: 0,
        created: now,
        excerpt: String::new(),
    });
    crate::data::persistence::clear_bookmark_field(positions_file, book_path);
}

pub fn load_bookmark_field(
    positions_file: &Path,
    book_path: &str,
) -> Option<crate::data::persistence::Bookmark> {
    let pos = crate::data::persistence::load_position(positions_file, book_path)?;
    pos.bookmark
}

pub fn clear_bookmark_field(positions_file: &Path, book_path: &str) {
    if let Some(mut pos) = crate::data::persistence::load_position(positions_file, book_path) {
        pos.bookmark = None;
        crate::data::persistence::save_position(positions_file, book_path, &pos);
    }
}

pub fn marks_path() -> &'static Path {
    Path::new(crate::data::mark::MARKS_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn setup_temp(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("kothok_test_marks_{}", name));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn migration_sentinel_none() {
        let dir = setup_temp("sentinel");
        let pos_file = dir.join("positions");
        let marks_file = dir.join("marks");
        fs::write(
            &pos_file,
            "/mnt/onboard/Book.epub|2|5|100|200|0:0:0|r|0.3500\n",
        )
        .unwrap();
        let mut marks: Vec<crate::data::mark::Mark> = Vec::new();
        migrate_bookmark(
            &marks_file,
            &pos_file,
            "/mnt/onboard/Book.epub",
            10,
            &mut marks,
            1000,
        );
        let bookmarks: Vec<_> = marks
            .iter()
            .filter(|m| m.kind == crate::data::mark::MarkKind::Bookmark)
            .collect();
        assert!(
            bookmarks.is_empty(),
            "sentinel 0:0:0 must not produce a bookmark"
        );
    }

    #[test]
    fn migration_short_line() {
        let dir = setup_temp("short");
        let pos_file = dir.join("positions");
        let marks_file = dir.join("marks");
        fs::write(&pos_file, "/mnt/onboard/Short.epub|1|3|50|80\n").unwrap();
        let mut marks: Vec<crate::data::mark::Mark> = Vec::new();
        migrate_bookmark(
            &marks_file,
            &pos_file,
            "/mnt/onboard/Short.epub",
            5,
            &mut marks,
            1000,
        );
        assert!(marks.is_empty(), "short line must not produce marks");
    }

    #[test]
    fn migration_out_of_range_chapter() {
        let dir = setup_temp("oor");
        let pos_file = dir.join("positions");
        let marks_file = dir.join("marks");
        fs::write(
            &pos_file,
            "/mnt/onboard/Shrunk.epub|2|1|10|20|99:1:0|a|0.1000\n",
        )
        .unwrap();
        let mut marks: Vec<crate::data::mark::Mark> = Vec::new();
        migrate_bookmark(
            &marks_file,
            &pos_file,
            "/mnt/onboard/Shrunk.epub",
            5,
            &mut marks,
            1000,
        );
        assert!(marks.is_empty(), "out-of-range chapter must be dropped");
    }

    #[test]
    fn migration_idempotent_second_run() {
        let dir = setup_temp("idem");
        let pos_file = dir.join("positions");
        let marks_file = dir.join("marks");
        fs::write(
            &pos_file,
            "/mnt/onboard/Idem.epub|2|3|100|200|2:3:80|r|0.2500\n",
        )
        .unwrap();
        let mut marks: Vec<crate::data::mark::Mark> = Vec::new();
        migrate_bookmark(
            &marks_file,
            &pos_file,
            "/mnt/onboard/Idem.epub",
            10,
            &mut marks,
            1000,
        );
        assert_eq!(marks.len(), 1);
        assert_eq!(marks[0].kind, crate::data::mark::MarkKind::Bookmark);
        assert_eq!(marks[0].chapter, 2);
        assert_eq!(marks[0].start, 80);
        let after_first = fs::read_to_string(&pos_file).unwrap();
        assert!(
            after_first.contains("0:0:0"),
            "positions must be rewritten with sentinel after migration"
        );
        let count_after_first = marks.len();
        migrate_bookmark(
            &marks_file,
            &pos_file,
            "/mnt/onboard/Idem.epub",
            10,
            &mut marks,
            1001,
        );
        assert_eq!(
            marks.len(),
            count_after_first,
            "second migration must be a no-op"
        );
    }

    #[test]
    fn cap_200() {
        let mut marks: Vec<crate::data::mark::Mark> = Vec::new();
        for i in 0..200 {
            marks.push(crate::data::mark::Mark {
                kind: crate::data::mark::MarkKind::Bookmark,
                chapter: 0,
                start: i,
                end: i,
                page_hint: 0,
                created: i as u64,
                excerpt: String::new(),
            });
        }
        let result = crate::data::mark::add_mark(
            &mut marks,
            crate::data::mark::Mark {
                kind: crate::data::mark::MarkKind::Bookmark,
                chapter: 0,
                start: 999,
                end: 999,
                page_hint: 0,
                created: 201,
                excerpt: String::new(),
            },
        );
        assert_eq!(result, Err("Marks limit reached (200/book)"));
        assert_eq!(marks.len(), 200);
    }

    #[test]
    fn bookmark_toggle_off() {
        let mut marks: Vec<crate::data::mark::Mark> = Vec::new();
        let count = crate::data::mark::toggle_bookmark(&mut marks, 3, 100, "excerpt".into(), 1);
        assert_eq!(count, 1);
        let bm = marks.iter().find(|m| {
            m.kind == crate::data::mark::MarkKind::Bookmark && m.chapter == 3 && m.start == 100
        });
        assert!(bm.is_some());
        let count = crate::data::mark::toggle_bookmark(&mut marks, 3, 100, "excerpt".into(), 2);
        assert_eq!(count, 0);
        assert!(marks
            .iter()
            .find(|m| {
                m.kind == crate::data::mark::MarkKind::Bookmark && m.chapter == 3 && m.start == 100
            })
            .is_none());
    }

    #[test]
    fn highlight_merge() {
        let mut marks: Vec<crate::data::mark::Mark> = Vec::new();
        crate::data::mark::add_mark(
            &mut marks,
            crate::data::mark::Mark {
                kind: crate::data::mark::MarkKind::Highlight,
                chapter: 2,
                start: 200,
                end: 300,
                page_hint: 0,
                created: 1,
                excerpt: String::new(),
            },
        )
        .unwrap();
        crate::data::mark::add_mark(
            &mut marks,
            crate::data::mark::Mark {
                kind: crate::data::mark::MarkKind::Highlight,
                chapter: 2,
                start: 250,
                end: 350,
                page_hint: 0,
                created: 2,
                excerpt: String::new(),
            },
        )
        .unwrap();
        let highlights: Vec<_> = marks
            .iter()
            .filter(|m| m.kind == crate::data::mark::MarkKind::Highlight && m.chapter == 2)
            .collect();
        assert_eq!(highlights.len(), 1);
        assert_eq!(highlights[0].start, 200);
        assert_eq!(highlights[0].end, 350);
    }

    #[test]
    fn marks_save_load_roundtrip() {
        let dir = setup_temp("roundtrip");
        let marks_file = dir.join("marks");
        let book_path = "/mnt/onboard/Test.epub";
        let marks = vec![
            crate::data::mark::Mark {
                kind: crate::data::mark::MarkKind::Bookmark,
                chapter: 1,
                start: 50,
                end: 50,
                page_hint: 3,
                created: 1000,
                excerpt: "test excerpt".into(),
            },
            crate::data::mark::Mark {
                kind: crate::data::mark::MarkKind::Highlight,
                chapter: 2,
                start: 100,
                end: 200,
                page_hint: 5,
                created: 2000,
                excerpt: "highlighted text".into(),
            },
        ];
        save_marks(&marks_file, book_path, &marks);
        let loaded = load_marks(&marks_file, book_path);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].kind, crate::data::mark::MarkKind::Bookmark);
        assert_eq!(loaded[0].chapter, 1);
        assert_eq!(loaded[0].start, 50);
        assert_eq!(loaded[0].excerpt, "test excerpt");
        assert_eq!(loaded[1].kind, crate::data::mark::MarkKind::Highlight);
        assert_eq!(loaded[1].start, 100);
        assert_eq!(loaded[1].end, 200);
        let other = load_marks(&marks_file, "/mnt/onboard/Other.epub");
        assert!(other.is_empty());
    }
}
