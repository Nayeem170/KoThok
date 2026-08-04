// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use super::*;

#[test]
fn fnv1a_deterministic() {
    assert_eq!(
        fnv1a("/mnt/onboard/Book.epub"),
        fnv1a("/mnt/onboard/Book.epub")
    );
}

#[test]
fn fnv1a_different_for_different_input() {
    assert_ne!(
        fnv1a("/mnt/onboard/Book.epub"),
        fnv1a("/mnt/onboard/Other.epub")
    );
}

#[test]
fn book_cache_path_uses_hash_and_ext() {
    let p = book_cache_path("/mnt/onboard/Book.epub");
    let name = p.file_name().unwrap().to_str().unwrap();
    assert!(name.ends_with(".bc"), "cache file must use .bc extension");
    let (hash, _) = name.split_once('.').expect("hash.version.bc");
    assert_eq!(hash.len(), 16, "16 hex chars of book hash: {name}");
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "{name}");
}

/// bincode is not self-describing, so a cache written by a build with a
/// different `TextSegment` layout parses as nonsense rather than failing. The
/// version is in the path so such a file is never opened.
#[test]
fn cache_path_is_versioned_and_stale_paths_differ() {
    let p = book_cache_path("/mnt/onboard/Book.epub");
    let name = p.file_name().unwrap().to_str().unwrap();
    assert!(
        name.contains(&format!(".v{CACHE_FORMAT}.")),
        "path must carry the format version: {name}"
    );
    let stale = stale_cache_paths("/mnt/onboard/Book.epub");
    assert!(!stale.is_empty(), "the unversioned original must be swept");
    assert!(
        !stale.contains(&p),
        "the live cache must never be listed as stale"
    );
}

#[test]
fn pre_block_preserves_code_lines() {
    // Regression: <pre>/<code> used to reflow as prose, merging code lines.
    let ch = Chapter::from_xhtml(
        0,
        None,
        "<pre><code>fn main() {\n    println!(\"hi\");\n}</code></pre>",
    );
    assert!(
        ch.text.contains("fn main() {\n    println!(\"hi\");\n}"),
        "pre lines/indent not preserved: {:?}",
        ch.text
    );
}

fn write_fixture_epub(path: &std::path::Path, chapters: &[&str]) {
    use std::io::Write;
    use zip::write::FileOptions;
    use zip::CompressionMethod;

    let file = std::fs::File::create(path).unwrap();
    let mut zw = zip::ZipWriter::new(file);
    zw.start_file(
        "mimetype",
        FileOptions::default().compression_method(CompressionMethod::Stored),
    )
    .unwrap();
    zw.write_all(b"application/epub+zip").unwrap();

    let opts = FileOptions::default().compression_method(CompressionMethod::Deflated);
    zw.start_file("META-INF/container.xml", opts).unwrap();
    zw.write_all(
        br#"<?xml version="1.0"?>
<container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
  <rootfiles><rootfile full-path="OEBPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
</container>"#,
    )
    .unwrap();

    let mut manifest = String::new();
    let mut spine = String::new();
    for i in 0..chapters.len() {
        manifest.push_str(&format!(
            "<item id=\"c{i}\" href=\"c{i}.xhtml\" media-type=\"application/xhtml+xml\"/>"
        ));
        spine.push_str(&format!("<itemref idref=\"c{i}\"/>"));
    }
    manifest
        .push_str("<item id=\"ncx\" href=\"toc.ncx\" media-type=\"application/x-dtbncx+xml\"/>");
    let opf = format!(
        r#"<?xml version="1.0"?>
<package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="bid">
  <metadata xmlns:dc="http://purl.org/dc/elements/1.1/">
    <dc:title>Lib Fixture</dc:title><dc:creator>Tester</dc:creator>
    <dc:language>en</dc:language><dc:identifier id="bid">lf-1</dc:identifier>
  </metadata>
  <manifest>{manifest}</manifest><spine toc="ncx">{spine}</spine>
</package>"#
    );
    zw.start_file("OEBPS/content.opf", opts).unwrap();
    zw.write_all(opf.as_bytes()).unwrap();
    zw.start_file("OEBPS/toc.ncx", opts).unwrap();
    zw.write_all(
        b"<?xml version=\"1.0\"?><ncx xmlns=\"http://www.daisy.org/z3986/2005/ncx/\" version=\"2005-1\"><navMap></navMap></ncx>",
    )
    .unwrap();
    for (i, body) in chapters.iter().enumerate() {
        zw.start_file(&format!("OEBPS/c{i}.xhtml"), opts).unwrap();
        zw.write_all(format!("<html><body>{body}</body></html>").as_bytes())
            .unwrap();
    }
    zw.finish().unwrap().sync_all().unwrap();
}

#[test]
fn open_book_returns_chapters_from_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("book.epub");
    write_fixture_epub(
        &path,
        &[
            "<h1>One</h1><p>First chapter body.</p>",
            "<p>Second chapter.</p>",
        ],
    );
    let (chapters, lang, _toc, _idx) =
        open_book(path.to_str().unwrap()).expect("fixture epub must open via open_book");
    assert_eq!(chapters.len(), 2);
    assert!(chapters[0].text.contains("First chapter body."));
    assert!(chapters[1].text.contains("Second chapter."));
    assert_eq!(lang.as_deref(), Some("en"));
}

#[test]
fn open_book_detects_bengali_from_fixture() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bn.epub");
    write_fixture_epub(&path, &["<p>এটি একটি বাংলা বই যার অনেক শব্দ আছে।</p>"]);
    let (chapters, lang, _toc, _idx) = open_book(path.to_str().unwrap()).unwrap();
    assert_eq!(chapters.len(), 1);
    assert_eq!(
        lang.as_deref(),
        Some("bn-BD"),
        "Bengali script must be detected"
    );
}

#[test]
fn open_book_returns_none_for_empty_epub() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.epub");
    write_fixture_epub(&path, &["<p></p>"]);
    assert!(open_book(path.to_str().unwrap()).is_none());
}

#[test]
fn scan_epubs_finds_and_lists_books() {
    let dir = tempfile::tempdir().unwrap();
    write_fixture_epub(&dir.path().join("a.epub"), &["<p>Book A.</p>"]);
    write_fixture_epub(&dir.path().join("b.epub"), &["<p>Book B.</p>"]);
    std::fs::write(dir.path().join("notes.txt"), "ignore me").unwrap();

    let books = scan_epubs(dir.path().to_str().unwrap())
        .expect("scan must return a vec for a readable directory");
    assert_eq!(books.len(), 2, "only .epub files are listed");
    let titles: Vec<&str> = books.iter().map(|b| b.title.as_str()).collect();
    assert!(
        titles.iter().all(|t| *t == "Lib Fixture"),
        "title pulled from OPF"
    );
    for b in &books {
        assert!(b.path.ends_with(".epub"));
    }
}

// --- toc_rows (issue 11) ----------------------------------------------------

fn toc_entry(
    label: &str,
    depth: usize,
    chapter: Option<usize>,
    children: Vec<TocEntry>,
) -> TocEntry {
    TocEntry {
        label: label.to_string(),
        depth,
        chapter,
        anchor: None,
        children,
    }
}

#[test]
fn toc_rows_falls_back_to_one_row_per_chapter_when_no_nav() {
    let ch0 = Chapter::from_xhtml(0, Some("Intro".to_string()), "<p>a</p>");
    let ch1 = Chapter::from_xhtml(1, Some("Middle".to_string()), "<p>b</p>");
    let rows = toc_rows(&[], &[ch0, ch1]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].depth, 0);
    assert_eq!(rows[0].chapter, Some(0));
    assert_eq!(rows[1].chapter, Some(1));
}

#[test]
fn toc_rows_flattens_a_nested_tree_in_document_order() {
    let ch0 = Chapter::from_xhtml(0, None, "<p>a</p>");
    let ch1 = Chapter::from_xhtml(1, None, "<p>b</p>");
    let tree = vec![toc_entry(
        "Part One",
        0,
        None,
        vec![
            toc_entry("Chapter One", 1, Some(0), vec![]),
            toc_entry("Chapter Two", 1, Some(1), vec![]),
        ],
    )];
    let rows = toc_rows(&tree, &[ch0, ch1]);
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[0].label, "Part One");
    assert_eq!(
        rows[0].chapter, None,
        "divider row is shown but not navigable"
    );
    assert_eq!(rows[1].label, "Chapter One");
    assert_eq!(rows[1].depth, 1);
    assert_eq!(rows[2].label, "Chapter Two");
}

/// A three-level nav (Part -> Chapter -> Section) flattens with each level's
/// depth incremented. File 0 is never named by the nav (only chapter 1 is), so
/// the merge synthesises a row for it before "Chapter Two"; the rest come
/// straight from the tree. Multiple entries into the same chapter (the chapter
/// and its section) stay as separate rows.
#[test]
fn toc_rows_flattens_a_three_level_tree_with_correct_depth() {
    let ch0 = Chapter::from_xhtml(0, None, "<p>a</p>");
    let ch1 = Chapter::from_xhtml(1, None, "<p>b</p>");
    let tree = vec![toc_entry(
        "Part One",
        0,
        None,
        vec![toc_entry(
            "Chapter Two",
            1,
            Some(1),
            vec![toc_entry("Section Two-A", 2, Some(1), vec![])],
        )],
    )];
    let rows = toc_rows(&tree, &[ch0, ch1]);
    assert_eq!(rows.len(), 4, "{rows:?}");
    assert_eq!(rows[0].depth, 0, "Part One");
    assert_eq!(
        rows[1].chapter,
        Some(0),
        "file 0 unnamed by the nav: synthesised from the spine"
    );
    assert_eq!(rows[1].depth, 0);
    assert_eq!(rows[2].depth, 1, "Chapter Two");
    assert_eq!(rows[3].depth, 2, "Section Two-A");
    assert_eq!(rows[2].chapter, Some(1));
    assert_eq!(rows[3].chapter, Some(1), "section shares the chapter file");
}

/// "The Book of Tomorrow" shape: the nav names only the first file, but the
/// spine keeps going. Every trailing unnamed file is synthesised from its own
/// content. Here the named entry ("Start") wins for file 0; files 1-3 become
/// plain spine rows.
#[test]
fn toc_rows_synthesises_trailing_spine_files_after_a_single_nav_entry() {
    let chs: Vec<Chapter> = (0..4)
        .map(|i| Chapter::from_xhtml(i, None, &format!("<p>Line {i}</p>")))
        .collect();
    let tree = vec![toc_entry("Start", 0, Some(0), vec![])];
    let rows = toc_rows(&tree, &chs);
    assert_eq!(rows.len(), 4, "{rows:?}");
    assert_eq!(rows[0].label, "Start");
    assert_eq!(rows[0].chapter, Some(0));
    for (i, r) in rows[1..].iter().enumerate() {
        assert_eq!(r.chapter, Some(i + 1), "trailing file synthesised");
        assert_eq!(r.depth, 0, "synthesised rows carry no nesting");
    }
}

/// A nav entry that points past earlier unnamed files fills the gap with
/// synthesised rows for those files, in spine order, before the entry itself.
#[test]
fn toc_rows_fills_unnamed_files_before_a_nav_entry() {
    let ch0 = Chapter::from_xhtml(0, Some("Cover".to_string()), "<p>a</p>");
    let ch1 = Chapter::from_xhtml(1, Some("Map".to_string()), "<p>b</p>");
    let ch2 = Chapter::from_xhtml(2, Some("Chapter 1".to_string()), "<p>c</p>");
    let tree = vec![toc_entry("Chapter 1", 0, Some(2), vec![])];
    let rows = toc_rows(&tree, &[ch0, ch1, ch2]);
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows[0].chapter, Some(0), "ch0 synthesised");
    assert_eq!(rows[0].label, "Cover");
    assert_eq!(rows[1].chapter, Some(1), "ch1 synthesised");
    assert_eq!(rows[2].chapter, Some(2), "nav entry wins for its file");
    assert_eq!(rows[2].label, "Chapter 1");
}

/// An anchor (`#fragment`) on a TocEntry must survive the flatten into
/// FlatTocRow, since the selection handler resolves it to a page offset.
#[test]
fn toc_rows_preserves_anchors_through_flattening() {
    let ch1 = Chapter::from_xhtml(0, None, "<p>b</p>");
    let mut tree = vec![toc_entry("Section", 0, Some(0), vec![])];
    tree[0].anchor = Some("intro".to_string());
    let rows = toc_rows(&tree, &[ch1]);
    assert_eq!(rows[0].anchor.as_deref(), Some("intro"));
}

#[test]
fn toc_rows_clamps_pathological_depth() {
    let ch0 = Chapter::from_xhtml(0, None, "<p>a</p>");
    let tree = vec![toc_entry("Too Deep", 999, Some(0), vec![])];
    let rows = toc_rows(&tree, &[ch0]);
    assert_eq!(rows[0].depth, MAX_TOC_DEPTH);
}

#[test]
fn welcome_epub_opens_and_has_expected_chapters() {
    let epub = format!("{}/samples/welcome.epub", env!("CARGO_MANIFEST_DIR"));
    if !std::path::Path::new(&epub).exists() {
        eprintln!("welcome.epub not found at {epub} - run package/make-tutorial.ps1");
        return;
    }
    let (chapters, lang, toc, _idx) =
        open_book(&epub).expect("welcome.epub must open via open_book");
    assert_eq!(chapters.len(), 16, "guide must have 16 chapters");
    assert_eq!(lang.as_deref(), Some("en"));
    assert!(!toc.is_empty(), "guide must have a TOC tree");
}
