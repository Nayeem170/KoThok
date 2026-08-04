// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
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
    if let Some(merged) = merge_if_overlapping(marks, &m) {
        marks.retain(|x| x != &merged);
        marks.push(merged);
    } else {
        marks.push(m);
    }
    marks.sort_by_key(|m| m.start);
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

fn merge_if_overlapping(marks: &[Mark], new: &Mark) -> Option<Mark> {
    if new.kind != MarkKind::Highlight {
        return None;
    }
    let mut merged = new.clone();
    let mut changed = false;
    for m in marks.iter().filter(|m| {
        m.kind == MarkKind::Highlight
            && m.chapter == new.chapter
            && m.start <= new.end
            && new.start <= m.end
    }) {
        merged.start = merged.start.min(m.start);
        merged.end = merged.end.max(m.end);
        changed = true;
    }
    if changed {
        Some(merged)
    } else {
        None
    }
}
