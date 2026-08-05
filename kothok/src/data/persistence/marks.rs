// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::fs;
use std::path::Path;

pub fn load_marks(marks_file: &Path, book_path: &str) -> Vec<crate::data::mark::Mark> {
    let data = match fs::read_to_string(marks_file) {
        Ok(d) => d,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
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
        let excerpt = unescape_excerpt(parts[7]);
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

pub fn save_marks(
    marks_file: &Path,
    book_path: &str,
    marks: &[crate::data::mark::Mark],
) -> std::io::Result<()> {
    let existing = match fs::read_to_string(marks_file) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e),
    };
    let prefix = format!("{book_path}|");
    let mut lines: Vec<String> = existing
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with(&prefix))
        .map(String::from)
        .collect();
    for m in marks {
        let kind = match m.kind {
            crate::data::mark::MarkKind::Bookmark => "b",
            crate::data::mark::MarkKind::Highlight => "h",
        };
        lines.push(format!(
            "{}|{}|{}|{}|{}|{}|{}|{}",
            book_path,
            kind,
            m.chapter,
            m.start,
            m.end,
            m.page_hint,
            m.created,
            escape_excerpt(&m.excerpt),
        ));
    }
    fs::write(marks_file, lines.join("\n"))
}

fn escape_excerpt(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '|' => out.push_str("\\|"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            c if c.is_control() && c != '\t' => {
                out.push('\\');
                out.push('x');
                out.push_str(&format!("{:02X}", c as u8));
            }
            c => out.push(c),
        }
    }
    out
}

fn unescape_excerpt(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('|') => out.push('|'),
                Some('x') => {
                    let hex: String = chars.by_ref().take(2).collect();
                    if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                        out.push(byte as char);
                    } else {
                        out.push('\\');
                        out.push('x');
                        out.push_str(&hex);
                    }
                }
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn migrate_bookmark(
    marks_file: &Path,
    positions_file: &Path,
    book_path: &str,
    chapter_count: usize,
    marks: &mut Vec<crate::data::mark::Mark>,
    now: u64,
    current_page: usize,
) -> std::io::Result<bool> {
    let bm_field = crate::data::persistence::load_bookmark_field(positions_file, book_path);
    let bm = match bm_field {
        Some(b) => b,
        None => return Ok(false),
    };
    if bm.chapter >= chapter_count {
        return Ok(false);
    }
    let already = marks
        .iter()
        .any(|m| m.kind == crate::data::mark::MarkKind::Bookmark && m.chapter == bm.chapter);
    if already {
        return Ok(false);
    }
    marks.push(crate::data::mark::Mark {
        kind: crate::data::mark::MarkKind::Bookmark,
        chapter: bm.chapter,
        start: bm.offset,
        end: bm.offset,
        page_hint: current_page,
        created: now,
        excerpt: String::new(),
    });
    save_marks(marks_file, book_path, marks)?;
    crate::data::persistence::clear_bookmark_field(positions_file, book_path);
    Ok(true)
}

#[allow(dead_code)]
pub fn load_bookmark_field(
    positions_file: &Path,
    book_path: &str,
) -> Option<crate::data::persistence::Bookmark> {
    let pos = crate::data::persistence::load_position(positions_file, book_path)?;
    pos.bookmark
}

#[allow(dead_code)]
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
            0,
        )
        .unwrap();
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
            0,
        )
        .unwrap();
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
            0,
        )
        .unwrap();
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
            0,
        )
        .unwrap();
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
            0,
        )
        .unwrap();
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
        let count = crate::data::mark::toggle_bookmark(&mut marks, 3, 100, "excerpt".into(), 1, 0);
        assert_eq!(count, 1);
        let bm = marks.iter().find(|m| {
            m.kind == crate::data::mark::MarkKind::Bookmark && m.chapter == 3 && m.start == 100
        });
        assert!(bm.is_some());
        let count = crate::data::mark::toggle_bookmark(&mut marks, 3, 100, "excerpt".into(), 2, 0);
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
        save_marks(&marks_file, book_path, &marks).unwrap();
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

    #[test]
    fn save_does_not_clobber_similar_book() {
        let dir = setup_temp("prefix_key");
        let marks_file = dir.join("marks");
        let path_a = "/mnt/onboard/A.epub";
        let path_b = "/mnt/onboard/A.epub.bak";
        fs::write(&marks_file, format!("{path_b}|b|0|0|0|0|100|stub\n")).unwrap();
        let marks = vec![crate::data::mark::Mark {
            kind: crate::data::mark::MarkKind::Bookmark,
            chapter: 0,
            start: 10,
            end: 10,
            page_hint: 0,
            created: 200,
            excerpt: String::new(),
        }];
        save_marks(&marks_file, path_a, &marks).unwrap();
        let loaded_b = load_marks(&marks_file, path_b);
        assert_eq!(
            loaded_b.len(),
            1,
            "A.epub.bak mark must survive a save of A.epub"
        );
        assert_eq!(loaded_b[0].chapter, 0);
        assert_eq!(loaded_b[0].start, 0);
    }

    #[test]
    fn excerpt_roundtrip_with_special_chars() {
        let dir = setup_temp("excerpt_escape");
        let marks_file = dir.join("marks");
        let book_path = "/mnt/onboard/Special.epub";
        let excerpt = "line1\nline2|\t\r\x01end";
        let marks = vec![crate::data::mark::Mark {
            kind: crate::data::mark::MarkKind::Highlight,
            chapter: 0,
            start: 5,
            end: 100,
            page_hint: 0,
            created: 1,
            excerpt: excerpt.to_string(),
        }];
        save_marks(&marks_file, book_path, &marks).unwrap();
        let content = fs::read_to_string(&marks_file).unwrap();
        assert!(
            !content.contains('\n'),
            "saved file must not contain bare newlines"
        );
        let loaded = load_marks(&marks_file, book_path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(
            loaded[0].excerpt, excerpt,
            "escaped excerpt must round-trip"
        );
    }

    #[test]
    fn read_error_does_not_wipe_file() {
        let dir = setup_temp("read_err");
        let book_path = "/mnt/onboard/Test.epub";
        let result = load_marks(&dir, book_path);
        assert!(
            result.is_empty(),
            "directory-as-path must return empty, not panic"
        );
    }

    #[test]
    fn save_error_does_not_wipe_on_bad_read() {
        let dir = setup_temp("save_err");
        let marks_file = dir.join("marks");
        fs::write(&marks_file, b"other.epub|b|0|0|0|0|1|\xff\xfe\n").unwrap();
        let res = save_marks(&marks_file, "/mnt/onboard/Test.epub", &[]);
        assert!(
            res.is_err(),
            "save must fail on invalid UTF-8 read, not silently proceed"
        );
        let remaining = fs::read(&marks_file).unwrap();
        assert!(
            remaining.starts_with(b"other.epub"),
            "save must not overwrite the file when read fails"
        );
    }

    #[test]
    fn migrate_preserves_positions_when_save_not_durable() {
        let dir = setup_temp("mig_durability");
        let marks_file = dir.join("no_write_dir").join("marks");
        let pos_file = dir.join("positions");
        fs::write(
            &pos_file,
            "/mnt/onboard/Dur.epub|2|3|100|200|2:3:80|r|0.2500\n",
        )
        .unwrap();
        let mut marks: Vec<crate::data::mark::Mark> = Vec::new();
        let res = migrate_bookmark(
            &marks_file,
            &pos_file,
            "/mnt/onboard/Dur.epub",
            10,
            &mut marks,
            1000,
            0,
        );
        assert!(res.is_err(), "save to nonexistent dir must fail");
        let pos_after = fs::read_to_string(&pos_file).unwrap();
        assert!(
            pos_after.contains("2:3:80"),
            "bookmark field must NOT be cleared if marks save failed"
        );
    }

    #[test]
    fn notfound_proceeds_as_empty() {
        let dir = setup_temp("notfound");
        let marks_file = dir.join("nonexistent_marks");
        let loaded = load_marks(&marks_file, "/mnt/onboard/Nope.epub");
        assert!(loaded.is_empty(), "missing file must yield empty marks");
    }
}
