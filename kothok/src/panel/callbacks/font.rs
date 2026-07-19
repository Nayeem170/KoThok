// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use log::debug;

use slint::platform::software_renderer::Rgb565Pixel;

use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::data::config::{save_config, AppConfig};
use crate::loop_state::LoopState;
use crate::reader::apply_page;
use crate::rendering::layout::{build_state, estimate_chapter_offsets, spawn_offset_computation};
use crate::Reader;

use super::super::FONT_DEBOUNCE_MS;

pub(super) fn handle_font_slider(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cfg: &mut AppConfig,
    cb: &Callbacks,
) -> bool {
    let mut text_dirty = false;

    if let Some(frac) = cb.font_frac_in.take() {
        let new_val = (20.0 + frac * 40.0).round() as i32;
        let new_val = (new_val / 2) * 2;
        if (20..=60).contains(&new_val) && new_val != cfg.font_size {
            cfg.font_size = new_val;
            reader.set_font_size_val(new_val);
            save_config(cfg);
            cb.font_pending_val.set(Some(new_val));
            cb.font_last_change.set(Some(Instant::now()));
        }
    }
    if let (Some(val), Some(t)) = (cb.font_pending_val.get(), cb.font_last_change.get()) {
        if t.elapsed() >= Duration::from_millis(FONT_DEBOUNCE_MS) {
            cb.font_pending_val.set(None);
            cb.font_last_change.set(None);
            apply_font_reflow(val, st, reader, cmd_tx);
            text_dirty = true;
        }
    }
    text_dirty
}

fn apply_font_reflow(new_val: i32, st: &mut LoopState, reader: &Reader, cmd_tx: &Sender<Cmd>) {
    let cur_start = reader.get_cur_start() as usize;
    let anchor = if cur_start > 0 {
        cur_start
    } else {
        let (rs, _) = st
            .state
            .pages
            .get(st.current_page)
            .copied()
            .unwrap_or((0, 0));
        st.state
            .all_rows
            .get(rs)
            .map(|r| r.start.max(1) as usize)
            .unwrap_or(1)
    };
    st.body_px = new_val as f32;
    st.head_px = new_val as f32 * crate::rendering::layout::HEADING_SCALE;
    st.line_h = (new_val as f32 * crate::rendering::layout::LINE_HEIGHT_SCALE) as i32;
    let cc = st.current_chapter;
    st.state = build_state(&mut st.chapters[cc], st.body_px, st.head_px, st.line_h);
    st.text_cache.fill(Rgb565Pixel(0xFFFF));
    st.current_page = st
        .state
        .pages
        .iter()
        .enumerate()
        .find(|(_, (rs, re))| {
            st.state.all_rows[*rs..*re]
                .iter()
                .any(|r| r.start as usize <= anchor && r.end as usize > anchor)
        })
        .map(|(i, _)| i)
        .unwrap_or(0);
    let layout = crate::rendering::layout::screen_layout();
    st.chapter_offsets.clone_from(&estimate_chapter_offsets(
        &st.chapters,
        (cc, st.state.pages.len()),
        st.line_h,
        &layout,
    ));
    st.offset_rx = Some(spawn_offset_computation(
        st.chapters.clone(),
        st.body_px,
        st.line_h,
        new_val,
        st.current_book_path.clone(),
        layout,
    ));
    apply_page(reader, &st.state, st.current_page, &st.chapter_offsets, cc);
    if let Some(row) = st
        .state
        .all_rows
        .iter()
        .find(|r| r.start as usize <= anchor && r.end as usize > anchor && r.start < r.end)
    {
        reader.set_cur_start(row.start);
        reader.set_cur_end(row.end);
    }
    reader.set_saved_page((st.chapter_offsets[cc] + st.current_page) as i32);
    let utts = crate::audio::glue::page_utterances(st.current_page, &st.state);
    crate::audio::glue::best_effort_send(cmd_tx, Cmd::Reload(utts.clone()));
    if reader.get_playing() {
        let utt_idx = crate::audio::glue::utterance_index_for_offset(&utts, anchor);
        crate::audio::glue::best_effort_send(cmd_tx, Cmd::Seek(utt_idx));
    } else {
        crate::audio::glue::best_effort_send(cmd_tx, Cmd::Seek(0));
    }
    let first_row = st
        .state
        .pages
        .get(st.current_page)
        .and_then(|(rs, _)| st.state.all_rows.get(*rs))
        .map(|r| r.text.as_str())
        .unwrap_or("");
    debug!(
        "font-reflow: ch={} page={}/{} rows={} anchor={} first-row=\"{}\"",
        cc + 1,
        st.current_page + 1,
        st.state.pages.len(),
        st.state.all_rows.len(),
        anchor,
        first_row
    );
}
