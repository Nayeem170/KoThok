// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use kobo_core::{Chapter, EpubBook, TocEntry};

use log::{info, warn};

use crate::data::config::{BOOK_CACHE_DIR, ELABEL_PATH_FILTER, POSITIONS_FILE};
use crate::data::word_index::build_word_index;
use crate::rendering::layout::state::build_chapter_body;

pub use crate::data::config::{BOOK_DIR, DEVICE_BOOK};
pub use kobo_core::formats::{detect_language, progress_from_offsets};

pub use crate::data::word_index::WordIndex;

/// On-disk cache of a fully-parsed EPUB (issue 2): the expensive part of opening
/// a large book is the per-chapter XHTML extraction (`html_text::extract`). This
/// serializes the extracted chapters so a re-open is a single file read.
#[derive(serde::Serialize, serde::Deserialize)]
struct CachedBook {
    /// EPUB file mtime (seconds since epoch) - used to invalidate a stale cache.
    mtime: u64,
    language: Option<String>,
    chapters: Vec<Chapter>,
    /// Table of contents tree (issue 11). Serde-defaulted so a cache written
    /// before this field existed would still deserialize -- moot in practice
    /// since `CACHE_FORMAT` was bumped alongside it, but cheap insurance.
    #[serde(default)]
    toc_tree: Vec<TocEntry>,
    #[serde(default)]
    word_index: WordIndex,
}

/// One row of the chapter-overlay list after the TOC tree is flattened for
/// painting: a depth-aware tree can't be drawn or hit-tested directly, both
/// need a flat, indexable sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct FlatTocRow {
    pub label: String,
    pub depth: usize,
    /// `None` for a divider/grouping entry that names no real chapter (see
    /// [`kobo_core::TocEntry::chapter`]) -- shown, but not a valid jump target.
    pub chapter: Option<usize>,
    pub anchor: Option<String>,
}

/// Untrusted nesting depth from the book's own nav tree feeds directly into a
/// pixel indent at paint time; without a ceiling a pathological nav (or one
/// that cycles through play-order oddities into deep nesting) could push a
/// row's text off the edge of the column entirely.
const MAX_TOC_DEPTH: usize = 6;

/// Flatten `toc_tree` into paintable rows, or fall back to one row per spine
/// chapter when the book ships no NCX/nav at all -- today's only behaviour,
/// so a book without a TOC does not regress.
pub fn toc_rows(toc_tree: &[TocEntry], chapters: &[Chapter]) -> Vec<FlatTocRow> {
    if toc_tree.is_empty() {
        return chapters
            .iter()
            .enumerate()
            .map(|(i, ch)| FlatTocRow {
                label: chapter_display_title(ch, i),
                depth: 0,
                chapter: Some(i),
                anchor: None,
            })
            .collect();
    }
    fn walk(entries: &[TocEntry], out: &mut Vec<FlatTocRow>) {
        for e in entries {
            out.push(FlatTocRow {
                label: e.label.clone(),
                depth: e.depth.min(MAX_TOC_DEPTH),
                chapter: e.chapter,
                anchor: e.anchor.clone(),
            });
            walk(&e.children, out);
        }
    }
    let mut out = Vec::new();
    walk(toc_tree, &mut out);
    out
}

pub fn fnv1a(s: &str) -> u64 {
    crate::data::persistence::book_hash(s)
}

/// Bumped whenever the serialised shape of `Chapter`/`TextSegment` changes.
///
/// It lives in the *filename* rather than inside the payload because bincode is
/// not self-describing: a cache written before a field existed does not fail to
/// parse, it parses as whatever the new layout makes of those bytes -- and a
/// length prefix read at the wrong offset asks for an allocation the device
/// cannot serve. A version that is part of the path means an outdated cache is
/// never opened at all.
const CACHE_FORMAT: u32 = 6;

pub fn book_cache_path(path: &str) -> std::path::PathBuf {
    let h = fnv1a(path);
    std::path::Path::new(BOOK_CACHE_DIR).join(format!("{h:016x}.v{CACHE_FORMAT}.bc"))
}

/// Cache files this build will never read, for the same book.
///
/// Removed on write rather than left behind: the reader keeps its cache on the
/// user's book partition, and a stale copy of every book parsed by an earlier
/// version is real space on a device that has little.
fn stale_cache_paths(path: &str) -> Vec<std::path::PathBuf> {
    let h = fnv1a(path);
    let dir = std::path::Path::new(BOOK_CACHE_DIR);
    let mut out = vec![dir.join(format!("{h:016x}.bc"))];
    out.extend((1..CACHE_FORMAT).map(|v| dir.join(format!("{h:016x}.v{v}.bc"))));
    out
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
        return None;
    }
    let cf = book_cache_path(path);
    let data = match std::fs::read(&cf) {
        Ok(d) => d,
        Err(_) => {
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
        return None;
    }
    Some(cached)
}

fn save_cached_book(
    path: &str,
    mtime: u64,
    language: &Option<String>,
    chapters: &[Chapter],
    toc_tree: &[TocEntry],
    word_index: &WordIndex,
) {
    if let Err(e) = std::fs::create_dir_all(BOOK_CACHE_DIR) {
        warn!("bookcache mkdir failed: {e}");
    }
    let chapters: Vec<Chapter> = chapters
        .iter()
        .map(|ch| {
            if ch.body.is_empty() && !ch.segments.is_empty() {
                let built = build_chapter_body(ch);
                let mut c = ch.clone();
                c.body = built.body;
                c.seg_body_start = built.seg_body_start;
                c.body_styles = built.styles;
                c.body_links = built.links;
                c
            } else {
                ch.clone()
            }
        })
        .collect();
    let cached = CachedBook {
        mtime,
        language: language.clone(),
        chapters,
        toc_tree: toc_tree.to_vec(),
        word_index: word_index.clone(),
    };
    if let Ok(bytes) = bincode::serialize(&cached) {
        if let Err(e) = std::fs::write(book_cache_path(path), bytes) {
            warn!("bookcache write failed: {e}");
            return;
        }
        for stale in stale_cache_paths(path) {
            let _ = std::fs::remove_file(stale);
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
    books.retain(|b| !b.path.contains(ELABEL_PATH_FILTER));

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

#[allow(clippy::type_complexity)]
pub fn open_book(path: &str) -> Option<(Vec<Chapter>, Option<String>, Vec<TocEntry>, WordIndex)> {
    if let Some(cached) = load_cached_book(path) {
        info!(
            "book: {path}: loaded from cache ({} chapter(s))",
            cached.chapters.len()
        );
        if cached.chapters.is_empty() {
            return None;
        }
        let lang = detect_language(&cached.chapters).or_else(|| cached.language.clone());
        return Some((cached.chapters, lang, cached.toc_tree, cached.word_index));
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
            let mut chapters = book.chapters;
            for ch in &mut chapters {
                if ch.body.is_empty() && !ch.segments.is_empty() {
                    let built = build_chapter_body(ch);
                    ch.body = built.body;
                    ch.seg_body_start = built.seg_body_start;
                    ch.body_styles = built.styles;
                    ch.body_links = built.links;
                }
            }
            let word_index = build_word_index(&chapters);
            save_cached_book(path, mtime, &lang, &chapters, &book.toc_tree, &word_index);
            Some((chapters, lang, book.toc_tree, word_index))
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
