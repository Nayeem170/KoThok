// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use slint::platform::software_renderer::Rgb565Pixel;

use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::data::config::{save_settings_for, AppConfig};
use crate::loop_state::LoopState;
use crate::reader::apply_page;
use crate::rendering::layout::{
    self as layout, build_state, estimate_chapter_offsets, spawn_offset_computation,
};
use crate::Reader;

use super::super::FONT_DEBOUNCE_MS;

pub(super) fn handle_font_slider(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cfg: &mut AppConfig,
    cb: &Callbacks,
    frac_opt: &Option<(i32, f32)>,
) -> bool {
    let mut text_dirty = false;

    // Font size slider (index 2, special-cased in callbacks.rs)
    if let Some(frac) = cb.font_frac_in.take() {
        let new_val = (20.0 + frac * 40.0).round() as i32;
        let new_val = (new_val / 2) * 2;
        if (20..=60).contains(&new_val) && new_val != cfg.font_size {
            cfg.font_size = new_val;
            reader.set_font_size_val(new_val);
            save_settings_for(&st.current_book_path, cfg, st.picker_active);
            cb.font_pending_val.set(Some(new_val));
            cb.font_last_change.set(Some(Instant::now()));
        }
    }
    // Line spacing slider (index 4) -- also triggers debounced reflow
    if let Some(frac) = super::sliders::handle_line_spacing_frac(frac_opt) {
        let new_pct = (100.0 + frac * 100.0).round() as i32;
        if (100..=200).contains(&new_pct) && new_pct != cfg.line_spacing_pct {
            cfg.line_spacing_pct = new_pct;
            st.line_spacing_pct = new_pct;
            // Without this the slider is a one-way street: the reflow happens
            // but `line-spacing-val` never moves, so the fill and the "1.4x"
            // caption sit where they were and the control reads as dead.
            reader.set_line_spacing_val(new_pct);
            save_settings_for(&st.current_book_path, cfg, st.picker_active);
            cb.font_pending_val.set(Some(cfg.font_size));
            cb.font_last_change.set(Some(Instant::now()));
        }
    }
    // Margins slider (index 5). Changing the margin changes `text_w()`, so it
    // needs the same debounced full reflow as font size -- every cached row was
    // wrapped to the old column.
    if let Some(frac) = super::sliders::handle_margin_frac(frac_opt) {
        let range = (layout::MARGIN_MAX_PX - layout::MARGIN_MIN_PX) as f32;
        let new_px = (layout::MARGIN_MIN_PX as f32 + frac * range).round() as i32;
        let new_px = (new_px / 4) * 4;
        if (layout::MARGIN_MIN_PX..=layout::MARGIN_MAX_PX).contains(&new_px)
            && new_px != cfg.margin_px
        {
            cfg.margin_px = new_px;
            layout::set_margin_px(new_px);
            reader.set_margin_val(new_px);
            save_settings_for(&st.current_book_path, cfg, st.picker_active);
            cb.font_pending_val.set(Some(cfg.font_size));
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
    st.line_h = (new_val as f32 * st.line_spacing_pct as f32 / 100.0) as i32;
    let cc = st.current_chapter;
    let Some(chapter) = st.chapters.get_mut(cc) else {
        return;
    };
    st.state = build_state(chapter, st.body_px, st.head_px, st.line_h, st.text_justify);
    st.text_cache.fill(Rgb565Pixel(0xFFFF));
    st.current_page = st
        .state
        .pages
        .iter()
        .enumerate()
        .find(|(_, (rs, re))| {
            st.state
                .all_rows
                .get(*rs..*re)
                .unwrap_or(&[])
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
    let Some(&offset) = st.chapter_offsets.get(cc) else {
        return;
    };
    reader.set_saved_page((offset + st.current_page) as i32);
    let utts = crate::audio::glue::page_utterances(st.current_page, &st.state);
    crate::audio::glue::best_effort_send(cmd_tx, Cmd::Reload(utts.clone()));
    if reader.get_playing() {
        let utt_idx = crate::audio::glue::utterance_index_for_offset(&utts, anchor);
        crate::audio::glue::best_effort_send(cmd_tx, Cmd::Seek(utt_idx));
    } else {
        crate::audio::glue::best_effort_send(cmd_tx, Cmd::Seek(0));
    }
}
