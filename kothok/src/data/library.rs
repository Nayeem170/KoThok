// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use kobo_core::{Chapter, EpubBook};

use log::{debug, info, warn};

use crate::data::config::{BOOK_CACHE_DIR, POSITIONS_FILE};

pub use crate::data::config::{BOOK_DIR, DEVICE_BOOK};
pub use kobo_core::formats::{detect_language, progress_from_offsets};

/// On-disk cache of a fully-parsed EPUB (issue 2): the expensive part of opening
/// a large book is the per-chapter XHTML extraction (`html_text::extract`). This
/// serializes the extracted chapters so a re-open is a single file read.
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedBook {
    /// EPUB file mtime (seconds since epoch) - used to invalidate a stale cache.
    mtime: u64,
    language: Option<String>,
    chapters: Vec<Chapter>,
}

pub fn fnv1a(s: &str) -> u64 {
    crate::data::persistence::book_hash(s)
}

pub fn book_cache_path(path: &str) -> std::path::PathBuf {
    let h = fnv1a(path);
    std::path::Path::new(BOOK_CACHE_DIR).join(format!("{h:016x}.bc"))
}

fn epub_mtime(path: &str) -> u64 {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn load_cached_book(path: &str) -> Option<CachedBook> {
    let mtime = epub_mtime(path);
    if mtime == 0 {
        debug!("bookcache: miss {path}: no mtime");
        return None;
    }
    let cf = book_cache_path(path);
    let data = match std::fs::read(&cf) {
        Ok(d) => d,
        Err(_) => {
            debug!("bookcache: miss {path}: no cache file");
            return None;
        }
    };
    let cached: CachedBook = match bincode::deserialize(&data) {
        Ok(c) => c,
        Err(e) => {
            warn!("bookcache: miss {path}: deserialize error: {e}");
            return None;
        }
    };
    if cached.mtime != mtime {
        debug!(
            "bookcache: miss {path}: mtime {} != {} (invalidated)",
            cached.mtime, mtime
        );
        return None;
    }
    Some(cached)
}

fn save_cached_book(path: &str, mtime: u64, language: &Option<String>, chapters: &[Chapter]) {
    // best-effort: cache write; a failure (read-only fs, full disk) only means
    // the next open re-parses - the reader still works from the in-memory copy.
    if let Err(e) = std::fs::create_dir_all(BOOK_CACHE_DIR) {
        warn!("bookcache mkdir failed: {e}");
    }
    let cached = CachedBook {
        mtime,
        language: language.clone(),
        chapters: chapters.to_vec(),
    };
    if let Ok(bytes) = bincode::serialize(&cached) {
        if let Err(e) = std::fs::write(book_cache_path(path), bytes) {
            warn!("bookcache write failed: {e}");
        }
    }
}

pub struct EpubEntry {
    pub title: String,
    pub author: Option<String>,
    pub path: String,
    pub cover_bytes: Option<Vec<u8>>,
    pub progress: f32,
}

pub fn scan_epubs(root: &str) -> Option<Vec<EpubEntry>> {
    let mut books: Vec<EpubEntry> = Vec::new();
    walk(root, &mut books);
    books.retain(|b| !b.path.contains("/.kobo/eLabel/"));

    let pos_data = std::fs::read_to_string(POSITIONS_FILE).ok();
    let mut last_opened: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut pos_pages: std::collections::HashMap<String, (usize, usize)> =
        std::collections::HashMap::new();
    let mut stored_progress: std::collections::HashMap<String, f32> =
        std::collections::HashMap::new();
    if let Some(ref data) = pos_data {
        for (i, line) in data.lines().enumerate() {
            let parts: Vec<&str> = line.split('|').collect();
            if let Some(book_path) = parts.first() {
                last_opened.insert(book_path.to_string(), i);
                if parts.len() >= 3 {
                    let ch = parts[1].parse::<usize>().unwrap_or(0);
                    let pg = parts[2].parse::<usize>().unwrap_or(0);
                    pos_pages.insert(book_path.to_string(), (ch, pg));
                }
                // Progress the reader recorded when it last saved. Absent on
                // lines written before the field existed.
                if let Some(p) = parts
                    .get(7)
                    .and_then(|s| s.trim().parse::<f32>().ok())
                    .filter(|p| p.is_finite() && *p > 0.0)
                {
                    stored_progress.insert(book_path.to_string(), p.clamp(0.0, 1.0));
                }
            }
        }
    }
    for b in books.iter_mut() {
        // Prefer what the reader recorded. Deriving it here needs the offset
        // cache, which is keyed by font size and thrown away on any layout
        // change that repaginates -- so a part-read book would otherwise show
        // 0 % until it was opened again.
        b.progress = stored_progress
            .get(&b.path)
            .copied()
            .or_else(|| {
                pos_pages
                    .get(&b.path)
                    .map(|(ch, pg)| book_progress(&b.path, *ch, *pg))
            })
            .unwrap_or(0.0);
    }
    let has_position: std::collections::HashSet<String> = last_opened.keys().cloned().collect();
    // Books with a saved position come first; within that group the most-recently
    // read (highest line index) comes first; books without a position sort by title.
    books.sort_by(|a, b| {
        let a_has = has_position.contains(&a.path);
        let b_has = has_position.contains(&b.path);
        match (a_has, b_has) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => {
                let a_i = last_opened.get(&a.path).copied().unwrap_or(usize::MAX);
                let b_i = last_opened.get(&b.path).copied().unwrap_or(usize::MAX);
                b_i.cmp(&a_i).then_with(|| a.title.cmp(&b.title))
            }
        }
    });

    Some(books)
}

fn book_progress(path: &str, chapter: usize, page: usize) -> f32 {
    use crate::data::persistence::load_any_offset_cache;
    match load_any_offset_cache(path) {
        Some(o) => progress_from_offsets(&o, chapter, page),
        None => 0.0,
    }
}

fn walk(dir: &str, out: &mut Vec<EpubEntry>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for e in entries.filter_map(|e| e.ok()) {
            let path = e.path();
            if path.is_dir() {
                // Skip hidden dirs (.adds, .kobo, .kobo-images, etc.): these
                // are system/app infrastructure, not user book folders. Without
                // this, test EPUBs and the extracted current-book leak into the
                // library listing.
                if path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('.'))
                {
                    continue;
                }
                walk(&path.to_string_lossy(), out);
            } else if path.extension().is_some_and(|ext| ext == "epub") {
                let path_str = path.to_string_lossy().into_owned();
                let (title, author) = epub_metadata(&path_str);
                let cover_bytes = EpubBook::cover_bytes(&path_str);
                out.push(EpubEntry {
                    title,
                    author,
                    path: path_str,
                    cover_bytes,
                    progress: 0.0,
                });
            }
        }
    }
}

fn epub_metadata(path: &str) -> (String, Option<String>) {
    let file_stem = || {
        std::path::Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Unknown".into())
    };
    match EpubBook::open(path) {
        Ok(book) => {
            let title = book
                .title
                .map(|t| t.trim().to_string())
                .filter(|t| !t.is_empty())
                .unwrap_or_else(file_stem);
            let author = book
                .author
                .map(|a| a.trim().to_string())
                .filter(|a| !a.is_empty());
            (title, author)
        }
        Err(_) => (file_stem(), None),
    }
}

pub fn open_book(path: &str) -> Option<(Vec<Chapter>, Option<String>)> {
    if let Some(cached) = load_cached_book(path) {
        info!(
            "book: {path}: loaded from cache ({} chapter(s))",
            cached.chapters.len()
        );
        if cached.chapters.is_empty() {
            return None;
        }
        let lang = detect_language(&cached.chapters).or_else(|| cached.language.clone());
        return Some((cached.chapters, lang));
    }
    let mtime = epub_mtime(path);
    match EpubBook::open(path) {
        Ok(book) => {
            let n = book.chapters.len();
            info!("book: {path}: {n} chapter(s)");
            if book.chapters.is_empty() {
                return None;
            }
            let lang = detect_language(&book.chapters).or_else(|| book.language.clone());
            save_cached_book(path, mtime, &lang, &book.chapters);
            Some((book.chapters, lang))
        }
        Err(e) => {
            warn!("book: {path}: open error: {e}");
            None
        }
    }
}

pub fn chapter_display_title(ch: &Chapter, idx: usize) -> String {
    ch.display_title(idx)
}

#[cfg(test)]
mod tests;
