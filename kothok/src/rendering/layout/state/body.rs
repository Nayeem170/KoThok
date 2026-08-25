// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use kobo_core::html_text::{LinkRun, StyleRun};
use kobo_core::Chapter;

pub(crate) struct BuiltBody {
    pub body: String,
    pub seg_body_start: Vec<usize>,
    pub styles: Vec<StyleRun>,
    pub links: Vec<LinkRun>,
}

pub(crate) fn build_chapter_body(chapter: &Chapter) -> BuiltBody {
    let full = &chapter.text;
    let segs = &chapter.segments;
    let mut body = String::new();
    let mut seg_body_start: Vec<usize> = Vec::with_capacity(segs.len());
    let mut styles: Vec<StyleRun> = Vec::new();
    let mut links: Vec<LinkRun> = Vec::new();

    for seg in segs {
        if seg.src.is_some() || seg.tag == "figure" {
            let seg_base = body.len() + usize::from(!body.is_empty());
            seg_body_start.push(seg_base);
            if let Some(cap) = seg.caption.as_deref().filter(|c| !c.is_empty()) {
                if !body.is_empty() {
                    body.push('\n');
                }
                body.push_str(cap);
            } else if !body.is_empty() {
                body.push('\n');
            }
            continue;
        }

        let seg_text = full.get(seg.start..seg.end).unwrap_or("");
        if seg_text.is_empty() {
            seg_body_start.push(body.len());
            continue;
        }

        if is_heading(&seg.tag) {
            if !body.is_empty() {
                body.push('\n');
            }
            let seg_base = body.len();
            seg_body_start.push(seg_base);
            let trimmed = seg_text.trim();
            body.push_str(trimmed);
            let shift = |off: usize| seg_base + off.saturating_sub(seg.start);
            for r in &seg.styles {
                styles.push(StyleRun {
                    start: shift(r.start),
                    end: shift(r.end),
                    bold: r.bold,
                    italic: r.italic,
                    link: r.link,
                });
            }
            for l in &seg.links {
                links.push(LinkRun {
                    start: shift(l.start),
                    end: shift(l.end),
                    href: l.href.clone(),
                });
            }
        } else if seg.tag == "pre" {
            if !body.is_empty() {
                body.push('\n');
            }
            let seg_base = body.len();
            seg_body_start.push(seg_base);
            body.push_str(seg_text);
        } else {
            if !body.is_empty() {
                body.push('\n');
            }
            let marker = seg.list_marker();
            let marker_len = marker.as_ref().map_or(0, |m| m.len());
            let seg_base = body.len();
            seg_body_start.push(seg_base);
            let marker_str = marker.as_deref().unwrap_or("");
            body.push_str(marker_str);
            body.push_str(seg_text);
            let shift = |off: usize| seg_base + marker_len + off.saturating_sub(seg.start);
            for r in &seg.styles {
                styles.push(StyleRun {
                    start: shift(r.start),
                    end: shift(r.end),
                    bold: r.bold,
                    italic: r.italic,
                    link: r.link,
                });
            }
            for l in &seg.links {
                links.push(LinkRun {
                    start: shift(l.start),
                    end: shift(l.end),
                    href: l.href.clone(),
                });
            }
        }
    }

    BuiltBody {
        body,
        seg_body_start,
        styles,
        links,
    }
}

fn is_heading(tag: &str) -> bool {
    matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
}

#[cfg(test)]
mod tests {
    use super::*;
    use kobo_core::html_text::{BlockquoteKind, ListKind, StyleRun, TextSegment};

    fn make_chapter(text: String, segments: Vec<TextSegment>) -> Chapter {
        Chapter {
            index: 0,
            title: None,
            text,
            segments,
            images: Vec::new(),
            epub_path: String::new(),
            chapter_path: String::new(),
            body: String::new(),
            seg_body_start: Vec::new(),
            body_styles: Vec::new(),
            body_links: Vec::new(),
        }
    }

    fn seg(start: usize, end: usize, tag: &str) -> TextSegment {
        TextSegment {
            start,
            end,
            tag: tag.to_string(),
            id: None,
            src: None,
            caption: None,
            indent: 0.0,
            styles: Vec::new(),
            list: None,
            list_depth: 0,
            blockquote: BlockquoteKind::None,
            svg: None,
            code_indent: false,
            links: Vec::new(),
        }
    }

    #[test]
    fn simple_paragraph() {
        let text = "Hello world".to_string();
        let segments = vec![seg(0, 11, "p")];
        let ch = make_chapter(text, segments);
        let built = build_chapter_body(&ch);
        assert_eq!(built.body, "Hello world");
        assert_eq!(built.seg_body_start, vec![0]);
    }

    #[test]
    fn multiple_paragraphs_get_separated_by_newline() {
        let text = "First paragraph\nSecond paragraph".to_string();
        let segments = vec![seg(0, 15, "p"), seg(16, 32, "p")];
        let ch = make_chapter(text, segments);
        let built = build_chapter_body(&ch);
        assert_eq!(built.body, "First paragraph\nSecond paragraph");
        assert_eq!(built.seg_body_start, vec![0, 16]);
    }

    #[test]
    fn heading_trimmed_in_body() {
        let text = "  Chapter One  ".to_string();
        let segments = vec![seg(0, 15, "h1")];
        let ch = make_chapter(text, segments);
        let built = build_chapter_body(&ch);
        assert_eq!(built.body, "Chapter One");
        assert_eq!(built.seg_body_start, vec![0]);
    }

    #[test]
    fn list_marker_prepended() {
        let text = "First item\nSecond item".to_string();
        let segs = vec![
            TextSegment {
                start: 0,
                end: 10,
                tag: "li".to_string(),
                id: None,
                src: None,
                caption: None,
                indent: 0.0,
                styles: Vec::new(),
                list: Some(ListKind::Unordered),
                list_depth: 0,
                blockquote: BlockquoteKind::None,
                svg: None,
                code_indent: false,
                links: Vec::new(),
            },
            TextSegment {
                start: 11,
                end: 22,
                tag: "li".to_string(),
                id: None,
                src: None,
                caption: None,
                indent: 0.0,
                styles: Vec::new(),
                list: Some(ListKind::Ordered(1)),
                list_depth: 0,
                blockquote: BlockquoteKind::None,
                svg: None,
                code_indent: false,
                links: Vec::new(),
            },
        ];
        let text_clone = text.clone();
        let ch = make_chapter(text_clone, segs);
        let built = build_chapter_body(&ch);
        assert_eq!(built.body, "\u{2022} First item\n1. Second item");
    }

    #[test]
    fn pre_block_preserved_verbatim() {
        let code = "fn main() {\n    println!(\"hello\");\n}".to_string();
        let text = code.clone();
        let segments = vec![seg(0, code.len(), "pre")];
        let ch = make_chapter(text, segments);
        let built = build_chapter_body(&ch);
        assert_eq!(built.body, "fn main() {\n    println!(\"hello\");\n}");
    }

    #[test]
    fn figure_caption_in_body() {
        let text = String::new();
        let segments = vec![TextSegment {
            start: 0,
            end: 0,
            tag: "figure".to_string(),
            id: None,
            src: Some("image.png".to_string()),
            caption: Some("A nice diagram".to_string()),
            indent: 0.0,
            styles: Vec::new(),
            list: None,
            list_depth: 0,
            blockquote: BlockquoteKind::None,
            svg: None,
            code_indent: false,
            links: Vec::new(),
        }];
        let ch = make_chapter(text, segments);
        let built = build_chapter_body(&ch);
        assert_eq!(built.body, "A nice diagram");
    }

    #[test]
    fn style_runs_rebased_onto_body() {
        let text = "bold text here".to_string();
        let segments = vec![TextSegment {
            start: 0,
            end: 14,
            tag: "p".to_string(),
            id: None,
            src: None,
            caption: None,
            indent: 0.0,
            styles: vec![StyleRun {
                start: 0,
                end: 4,
                bold: true,
                italic: false,
                link: false,
            }],
            list: None,
            list_depth: 0,
            blockquote: BlockquoteKind::None,
            svg: None,
            code_indent: false,
            links: Vec::new(),
        }];
        let ch = make_chapter(text, segments);
        let built = build_chapter_body(&ch);
        assert_eq!(built.styles.len(), 1);
        assert_eq!(built.styles[0].start, 0);
        assert_eq!(built.styles[0].end, 4);
        assert!(built.styles[0].bold);
    }

    #[test]
    fn style_runs_rebased_with_list_marker() {
        let text = "bold text".to_string();
        let segments = vec![TextSegment {
            start: 0,
            end: 9,
            tag: "li".to_string(),
            id: None,
            src: None,
            caption: None,
            indent: 0.0,
            styles: vec![StyleRun {
                start: 0,
                end: 4,
                bold: true,
                italic: false,
                link: false,
            }],
            list: Some(ListKind::Unordered),
            list_depth: 0,
            blockquote: BlockquoteKind::None,
            svg: None,
            code_indent: false,
            links: Vec::new(),
        }];
        let ch = make_chapter(text, segments);
        let built = build_chapter_body(&ch);
        let marker = "\u{2022} ";
        let marker_len = marker.len();
        assert_eq!(built.body, format!("{marker}bold text"));
        assert_eq!(built.styles.len(), 1);
        assert_eq!(built.styles[0].start, marker_len);
        assert_eq!(built.styles[0].end, marker_len + 4);
    }

    #[test]
    fn style_runs_rebased_after_preceding_segment() {
        let text = "First\nSecond bold".to_string();
        let segments = vec![
            seg(0, 5, "p"),
            TextSegment {
                start: 6,
                end: 17,
                tag: "p".to_string(),
                id: None,
                src: None,
                caption: None,
                indent: 0.0,
                styles: vec![StyleRun {
                    start: 7,
                    end: 12,
                    bold: true,
                    italic: false,
                    link: false,
                }],
                list: None,
                list_depth: 0,
                blockquote: BlockquoteKind::None,
                svg: None,
                code_indent: false,
                links: Vec::new(),
            },
        ];
        let ch = make_chapter(text, segments);
        let built = build_chapter_body(&ch);
        assert_eq!(built.body, "First\nSecond bold");
        assert_eq!(built.styles.len(), 1);
        assert_eq!(built.styles[0].start, 7);
        assert_eq!(built.styles[0].end, 12);
    }

    #[test]
    fn link_runs_rebased_onto_body() {
        let text = "click here".to_string();
        let segments = vec![TextSegment {
            start: 0,
            end: 10,
            tag: "p".to_string(),
            id: None,
            src: None,
            caption: None,
            indent: 0.0,
            styles: Vec::new(),
            list: None,
            list_depth: 0,
            blockquote: BlockquoteKind::None,
            svg: None,
            code_indent: false,
            links: vec![LinkRun {
                start: 0,
                end: 10,
                href: "https://example.com".to_string(),
            }],
        }];
        let ch = make_chapter(text, segments);
        let built = build_chapter_body(&ch);
        assert_eq!(built.links.len(), 1);
        assert_eq!(built.links[0].start, 0);
        assert_eq!(built.links[0].end, 10);
        assert_eq!(built.links[0].href, "https://example.com");
    }

    #[test]
    fn figure_without_caption_still_separates() {
        let text = "Some text".to_string();
        let segments = vec![
            seg(0, 9, "p"),
            TextSegment {
                start: 0,
                end: 0,
                tag: "figure".to_string(),
                id: None,
                src: Some("img.png".to_string()),
                caption: None,
                indent: 0.0,
                styles: Vec::new(),
                list: None,
                list_depth: 0,
                blockquote: BlockquoteKind::None,
                svg: None,
                code_indent: false,
                links: Vec::new(),
            },
            seg(0, 0, "p"),
        ];
        let ch = make_chapter(text, segments);
        let built = build_chapter_body(&ch);
        assert_eq!(built.body, "Some text\n");
        assert_eq!(built.seg_body_start.len(), 3);
    }
}
