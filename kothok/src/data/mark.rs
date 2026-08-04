// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
#![allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MarkKind {
    Bookmark,
    Highlight,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mark {
    pub kind: MarkKind,
    pub chapter: usize,
    pub start: usize,
    pub end: usize,
    pub page_hint: usize,
    pub created: u64,
    pub excerpt: String,
}

pub const MARKS_FILE: &str = "/mnt/onboard/.adds/marks";
pub const MAX_MARKS_PER_BOOK: usize = 200;

pub fn add_mark(marks: &mut Vec<Mark>, m: Mark) -> Result<(), &'static str> {
    if marks.len() >= MAX_MARKS_PER_BOOK {
        return Err("Marks limit reached (200/book)");
    }
    if m.kind == MarkKind::Highlight {
        let chapter = m.chapter;
        let overlapping: Vec<(usize, usize)> = marks
            .iter()
            .filter(|x| {
                x.kind == MarkKind::Highlight
                    && x.chapter == chapter
                    && x.start <= m.end
                    && m.start <= x.end
            })
            .map(|x| (x.start, x.end))
            .collect();
        let mut merged = m;
        for (s, e) in &overlapping {
            merged.start = merged.start.min(*s);
            merged.end = merged.end.max(*e);
        }
        marks.retain(|x| {
            !(x.kind == MarkKind::Highlight
                && x.chapter == chapter
                && x.start <= merged.end
                && merged.start <= x.end)
        });
        marks.push(merged);
        marks.sort_by_key(|m| m.start);
    } else {
        marks.push(m);
        marks.sort_by_key(|m| m.start);
    }
    Ok(())
}

pub fn remove_mark(marks: &mut Vec<Mark>, idx: usize) {
    if idx < marks.len() {
        marks.remove(idx);
    }
}

pub fn toggle_bookmark(
    marks: &mut Vec<Mark>,
    chapter: usize,
    offset: usize,
    excerpt: String,
    now: u64,
) -> usize {
    if let Some(pos) = marks
        .iter()
        .position(|m| m.kind == MarkKind::Bookmark && m.chapter == chapter && m.start == offset)
    {
        marks.remove(pos);
    } else {
        marks.push(Mark {
            kind: MarkKind::Bookmark,
            chapter,
            start: offset,
            end: offset,
            page_hint: 0,
            created: now,
            excerpt,
        });
    }
    marks.len()
}
