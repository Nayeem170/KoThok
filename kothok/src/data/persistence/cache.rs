use std::fs;
use std::path::Path;

pub const CACHE_DIR: &str = "/mnt/onboard/.adds/cache";
pub const CACHE_MAGIC: &[u8; 4] = b"KOCO";
pub const CACHE_LAYOUT_VERSION: u16 = 1;

pub fn book_hash(path: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for b in path.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn cache_path(book_path: &str, font_size: i32) -> String {
    let hash = book_hash(book_path);
    format!("{}/{}_{:04}.bin", CACHE_DIR, hash, font_size)
}

pub fn load_offset_cache(path: &str) -> Option<Vec<usize>> {
    let data = fs::read(path).ok()?;
    // Header: magic[0..4] + layout_version[4..6] + count[6..10] = 10 bytes;
    // payload (u32 LE offsets) starts at index 10.
    if data.len() < 10 {
        return None;
    }
    if &data[0..4] != CACHE_MAGIC {
        return None;
    }
    let layout_ver = u16::from_le_bytes([data[4], data[5]]);
    if layout_ver != CACHE_LAYOUT_VERSION {
        return None;
    }
    let expected_len = u32::from_le_bytes([data[6], data[7], data[8], data[9]]) as usize;
    let payload = &data[10..];
    if payload.len() % 4 != 0 {
        return None;
    }
    let count = payload.len() / 4;
    if count != expected_len {
        return None;
    }
    let offsets: Vec<usize> = (0..count)
        .map(|i| {
            u32::from_le_bytes([
                payload[i * 4],
                payload[i * 4 + 1],
                payload[i * 4 + 2],
                payload[i * 4 + 3],
            ]) as usize
        })
        .collect();
    Some(offsets)
}

/// Load the most-recently-written offset cache for a book, regardless of which
/// font size it was paginated at. Offset caches are font-keyed
/// (`{hash}_{font}.bin`), but the saved reading position (chapter/page) was
/// recorded against the cache at the font the user actually read with — so the
/// matching cache is the newest one for that book. This avoids a fixed-font
/// lookup that returned 0 % or a stale fraction whenever the reading font
/// differed (e.g. the screen-scaled default).
pub fn load_any_offset_cache(book_path: &str) -> Option<Vec<usize>> {
    load_any_offset_cache_in(CACHE_DIR, book_path)
}

fn load_any_offset_cache_in(dir: &str, book_path: &str) -> Option<Vec<usize>> {
    let prefix = format!("{}_", book_hash(book_path));
    let mut best: Option<(std::time::SystemTime, Vec<usize>)> = None;
    let entries = fs::read_dir(dir).ok()?;
    for e in entries.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with(&prefix) || !name.ends_with(".bin") {
            continue;
        }
        let mtime = e
            .metadata()
            .and_then(|m| m.modified())
            .unwrap_or(std::time::UNIX_EPOCH);
        let path = e.path();
        if let Some(offsets) = load_offset_cache(&path.to_string_lossy()) {
            if best.as_ref().is_none_or(|(t, _)| mtime > *t) {
                best = Some((mtime, offsets));
            }
        }
    }
    best.map(|(_, o)| o)
}

pub fn save_offset_cache(path: &str, offsets: &[usize]) {
    // best-effort: cache dir may not exist yet
    let _ = fs::create_dir_all(Path::new(path).parent().unwrap_or(Path::new(CACHE_DIR)));
    let mut data: Vec<u8> = Vec::with_capacity(12 + offsets.len() * 4);
    data.extend_from_slice(CACHE_MAGIC);
    data.extend_from_slice(&CACHE_LAYOUT_VERSION.to_le_bytes());
    data.extend_from_slice(&(offsets.len() as u32).to_le_bytes());
    for v in offsets {
        data.extend_from_slice(&(*v as u32).to_le_bytes());
    }
    let tmp = format!("{}.tmp", path);
    // best-effort: atomic rename via tmp; disk may be full
    let _ = fs::write(&tmp, &data);
    let _ = fs::rename(&tmp, path);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_any_offset_cache_finds_cache_regardless_of_font() {
        let dir = std::env::temp_dir().join("kothok_test_any_cache");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&dir);
        let book = "/mnt/onboard/Novel.epub";
        let h = book_hash(book);
        // Only a font-35 cache exists (e.g. the screen-scaled default). The old
        // fixed-font-36 lookup returned None here → hero showed 0 %. The helper
        // must find it.
        save_offset_cache(
            &format!("{}/{}_{:04}.bin", dir.display(), h, 35),
            &[0, 10, 20, 30],
        );
        let got = load_any_offset_cache_in(&dir.to_string_lossy(), book).expect("cache found");
        assert_eq!(got, vec![0, 10, 20, 30]);
        // An unrelated book's hash must not match.
        assert!(
            load_any_offset_cache_in(&dir.to_string_lossy(), "/mnt/onboard/Other.epub").is_none()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn offset_cache_roundtrip() {
        let dir = std::env::temp_dir().join("kothok_test_cache_roundtrip");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("cache.bin");
        let offs = vec![0, 100, 250, 900];
        save_offset_cache(path.to_str().unwrap(), &offs);
        let loaded = load_offset_cache(path.to_str().unwrap()).unwrap();
        assert_eq!(loaded, offs);
    }

    #[test]
    fn offset_cache_rejects_wrong_magic() {
        let dir = std::env::temp_dir().join("kothok_test_cache_magic");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("cache.bin");
        save_offset_cache(path.to_str().unwrap(), &[1, 2, 3]);
        let mut data = std::fs::read(&path).unwrap();
        data[0..4].copy_from_slice(b"XXXX");
        std::fs::write(&path, data).unwrap();
        assert!(load_offset_cache(path.to_str().unwrap()).is_none());
    }

    #[test]
    fn offset_cache_rejects_wrong_version() {
        let dir = std::env::temp_dir().join("kothok_test_cache_ver");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("cache.bin");
        save_offset_cache(path.to_str().unwrap(), &[1, 2]);
        let mut data = std::fs::read(&path).unwrap();
        data[4..6].copy_from_slice(&(999u16).to_le_bytes());
        std::fs::write(&path, data).unwrap();
        assert!(load_offset_cache(path.to_str().unwrap()).is_none());
    }

    #[test]
    fn offset_cache_preserves_multi_chapter_offsets() {
        let dir = std::env::temp_dir().join("kothok_test_cache_multi");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("cache.bin");
        let offs = vec![0, 4096, 1_000_000, 4_294_967_000];
        save_offset_cache(path.to_str().unwrap(), &offs);
        assert_eq!(load_offset_cache(path.to_str().unwrap()).unwrap(), offs);
    }

    #[test]
    fn offset_cache_empty_chapter_list() {
        let dir = std::env::temp_dir().join("kothok_test_cache_empty");
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("cache.bin");
        save_offset_cache(path.to_str().unwrap(), &[]);
        let loaded = load_offset_cache(path.to_str().unwrap()).unwrap();
        assert!(loaded.is_empty());
    }

    #[test]
    fn book_hash_deterministic() {
        assert_eq!(
            book_hash("/mnt/onboard/Book.epub"),
            book_hash("/mnt/onboard/Book.epub")
        );
    }

    #[test]
    fn book_hash_different_for_different_input() {
        assert_ne!(
            book_hash("/mnt/onboard/Book.epub"),
            book_hash("/mnt/onboard/Other.epub")
        );
        assert_ne!(book_hash(""), book_hash("a"));
    }

    #[test]
    fn cache_path_format_and_font_suffix() {
        let p = cache_path("/mnt/onboard/Book.epub", 36);
        assert!(
            p.starts_with(CACHE_DIR),
            "cache path must live under CACHE_DIR"
        );
        assert!(
            p.ends_with("_0036.bin"),
            "font size is zero-padded to 4 digits with .bin suffix: {p}"
        );
    }

    #[test]
    fn cache_path_distinguishes_same_book_different_font() {
        let a = cache_path("/mnt/onboard/Book.epub", 36);
        let b = cache_path("/mnt/onboard/Book.epub", 48);
        assert_ne!(
            a, b,
            "different font sizes must map to different cache files"
        );
    }

    #[test]
    fn cache_path_deterministic_for_same_inputs() {
        assert_eq!(
            cache_path("/mnt/onboard/Book.epub", 40),
            cache_path("/mnt/onboard/Book.epub", 40)
        );
    }
}
