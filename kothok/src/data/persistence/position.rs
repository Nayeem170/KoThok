use std::fs;
use std::path::Path;

pub const POSITIONS_FILE: &str = "/mnt/onboard/.adds/positions";

pub struct ReadingPosition {
    pub chapter: usize,
    pub page: usize,
    pub cur_start: usize,
    pub cur_end: usize,
}

pub fn save_position(file: &Path, book_path: &str, pos: &ReadingPosition) {
    let mut lines: Vec<String> = fs::read_to_string(file)
        .unwrap_or_default()
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with(book_path))
        .map(String::from)
        .collect();
    lines.push(format!(
        "{}|{}|{}|{}|{}",
        book_path, pos.chapter, pos.page, pos.cur_start, pos.cur_end
    ));
    let _ = fs::write(file, lines.join("\n"));
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
            return Some(ReadingPosition {
                chapter: ch,
                page: pg,
                cur_start: cs,
                cur_end: ce,
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
            },
        );
        let pos = load_position(&file, "/mnt/onboard/Book.epub").unwrap();
        assert_eq!(pos.chapter, 3);
        assert_eq!(pos.page, 7);
        assert_eq!(pos.cur_start, 150);
        assert_eq!(pos.cur_end, 200);
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
            },
        );
        let pos = load_position(&file, "/mnt/onboard/Book.epub").unwrap();
        assert_eq!(pos.chapter, 5);
        assert_eq!(pos.page, 9);
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
}
