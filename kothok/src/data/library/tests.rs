use super::*;

#[test]
fn progress_from_offsets_mid_book() {
    assert_eq!(progress_from_offsets(&[0, 10, 20, 30], 1, 5), 0.5);
}

#[test]
fn progress_from_offsets_start_is_zero() {
    assert_eq!(progress_from_offsets(&[0, 10, 20, 30], 0, 0), 0.0);
}

#[test]
fn progress_from_offsets_end_clamps_to_one() {
    assert_eq!(progress_from_offsets(&[0, 10, 20, 30], 2, 10), 1.0);
}

#[test]
fn progress_from_offsets_empty_offsets_is_zero() {
    assert_eq!(progress_from_offsets(&[], 0, 0), 0.0);
}

#[test]
fn progress_from_offsets_overflow_chapter_uses_page_only() {
    assert_eq!(progress_from_offsets(&[0, 10], 9, 3), 0.3);
}

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
    assert_eq!(name.len(), 16 + 3, "name is 16 hex chars + .bc");
}

#[test]
fn detect_language_bengali() {
    let ch = Chapter::from_xhtml(0, None, "<p>বাংলা ভাষা একটি ইন্দো-আর্য ভাষা</p>");
    assert_eq!(detect_language(&[ch]).as_deref(), Some("bn-BD"));
}

#[test]
fn detect_language_arabic() {
    let ch = Chapter::from_xhtml(0, None, "<p>اللغة العربية لغة سامية</p>");
    assert_eq!(detect_language(&[ch]).as_deref(), Some("ar-SA"));
}

#[test]
fn detect_language_latin_returns_none() {
    let ch = Chapter::from_xhtml(
        0,
        None,
        "<p>The quick brown fox jumps over the lazy dog.</p>",
    );
    assert_eq!(detect_language(&[ch]), None);
}

#[test]
fn chapter_display_title_from_title() {
    let ch = Chapter::from_xhtml(0, Some("  Prologue  ".into()), "<p>text</p>");
    assert_eq!(chapter_display_title(&ch, 0), "Prologue");
}

#[test]
fn chapter_display_title_fallback_to_chapter_n() {
    let ch = Chapter::from_xhtml(2, None, "");
    assert_eq!(chapter_display_title(&ch, 2), "Chapter 3");
}

#[test]
fn chapter_display_title_ignores_empty_title() {
    let ch = Chapter::from_xhtml(0, Some("   ".into()), "");
    assert_eq!(chapter_display_title(&ch, 0), "Chapter 1");
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
    let (chapters, lang) =
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
    let (chapters, lang) = open_book(path.to_str().unwrap()).unwrap();
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
