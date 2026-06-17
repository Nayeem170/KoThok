use std::cell::Cell;
use std::sync::mpsc::Sender;
use std::time::{Duration, Instant};

use log::debug;

use slint::platform::software_renderer::Rgb565Pixel;
use slint::{ModelRc, SharedString, VecModel};

use kobo_core::Chapter;

use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::data::config::{save_config, AppConfig};
use crate::device::power::frontlight_set;
use crate::device::{bt_toggle, wifi_toggle};
use crate::loop_state::LoopState;
use crate::reader::apply_page;
use crate::rendering::layout::{build_state, estimate_chapter_offsets, spawn_offset_computation};
use crate::{ChapterItem, Reader};

use super::{voice_label, voices_for_lang, FONT_DEBOUNCE_MS};

pub fn process_panel_callbacks(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cfg: &mut AppConfig,
    fl_path: &Option<std::path::PathBuf>,
    cb: &Callbacks,
) -> bool {
    let frac_opt = cb.panel_frac.take();

    handle_brightness(reader, cfg, fl_path, &cb.panel_br, &frac_opt);
    handle_volume(reader, cmd_tx, cfg, &cb.panel_vol, &frac_opt);
    handle_tts_rate(reader, cmd_tx, cfg, &cb.panel_sp, &frac_opt);
    let mut text_dirty = handle_font_slider(st, reader, cfg, cb);
    handle_voice_cycle(reader, cmd_tx, cfg, &cb.panel_voice_cell);
    handle_toggles(reader, &cb.wifi_toggle_cell, &cb.bt_toggle_cell);
    text_dirty |= handle_chapter_overlay(st, reader, cmd_tx, cb);

    text_dirty
}

fn handle_brightness(
    reader: &Reader,
    cfg: &mut AppConfig,
    fl_path: &Option<std::path::PathBuf>,
    panel_br: &Cell<i32>,
    frac_opt: &Option<(i32, f32)>,
) {
    let br_delta = panel_br.replace(0);
    if br_delta != 0 {
        let new_val = (reader.get_brightness_val() + br_delta).clamp(0, 100);
        reader.set_brightness_val(new_val);
        cfg.brightness = new_val;
        if let Some(ref path) = fl_path {
            frontlight_set(path, new_val as u32);
            debug!("panel: brightness set to {new_val}");
        }
        save_config(cfg);
    }

    if let Some((0, frac)) = frac_opt {
        let new_val = (frac * 100.0).round() as i32;
        reader.set_brightness_val(new_val);
        cfg.brightness = new_val;
        if let Some(ref path) = fl_path {
            frontlight_set(path, new_val as u32);
        }
        save_config(cfg);
    }
}

fn handle_volume(
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cfg: &mut AppConfig,
    panel_vol: &Cell<i32>,
    frac_opt: &Option<(i32, f32)>,
) {
    let vol_delta = panel_vol.replace(0);
    if vol_delta != 0 {
        let new_val = (cfg.volume + vol_delta).clamp(0, 100);
        cfg.volume = new_val;
        reader.set_volume_val(new_val);
        // best-effort: channel may be full
        let _ = cmd_tx.send(Cmd::Volume(new_val as u32));
        save_config(cfg);
        debug!("panel: volume set to {new_val}%");
    }

    if let Some((3, frac)) = frac_opt {
        let new_val = (frac * 100.0).round() as i32;
        cfg.volume = new_val;
        reader.set_volume_val(new_val);
        // best-effort: channel may be full
        let _ = cmd_tx.send(Cmd::Volume(new_val as u32));
        save_config(cfg);
    }
}

fn handle_tts_rate(
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cfg: &mut AppConfig,
    panel_sp: &Cell<i32>,
    frac_opt: &Option<(i32, f32)>,
) {
    if let Some((1, frac)) = frac_opt {
        let new_val = (frac * 100.0).round() as i32;
        cfg.tts_rate = new_val;
        reader.set_tts_speed(new_val);
        // best-effort: channel may be full
        let _ = cmd_tx.send(Cmd::Rate(crate::data::config::rate_string(new_val)));
        save_config(cfg);
    }

    let sp_delta = panel_sp.replace(0);
    if sp_delta != 0 {
        let new_val = (cfg.tts_rate + sp_delta).clamp(0, 100);
        cfg.tts_rate = new_val;
        reader.set_tts_speed(new_val);
        let rate_str = crate::data::config::rate_string(new_val);
        debug!("panel: rate set to {}", rate_str);
        // best-effort: channel may be full
        let _ = cmd_tx.send(Cmd::Rate(rate_str));
        save_config(cfg);
    }
}

fn handle_font_slider(
    st: &mut LoopState,
    reader: &Reader,
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
            apply_font_reflow(val, st, reader);
            text_dirty = true;
        }
    }
    let fs_delta = cb.panel_fs.replace(0);
    if fs_delta != 0 {
        let new_val = (cfg.font_size + fs_delta).clamp(20, 60);
        if new_val != cfg.font_size {
            cfg.font_size = new_val;
            reader.set_font_size_val(new_val);
            save_config(cfg);
            cb.font_pending_val.set(Some(new_val));
            cb.font_last_change.set(Some(Instant::now()));
        }
    }
    text_dirty
}

fn handle_voice_cycle(
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cfg: &mut AppConfig,
    panel_voice_cell: &Cell<bool>,
) {
    if panel_voice_cell.replace(false) {
        let voices = voices_for_lang(&cfg.tts_lang);
        let current = cfg.tts_voice.as_str();
        let idx = voices.iter().position(|v| v.id == current).unwrap_or(0);
        let new_voice = voices[(idx + 1) % voices.len()].id;
        cfg.tts_voice = new_voice.to_string();
        cfg.voices
            .insert(cfg.tts_lang.clone(), new_voice.to_string());
        debug!(
            "voice-cycle: lang={} new={} saved_map_size={}",
            cfg.tts_lang,
            new_voice,
            cfg.voices.len()
        );
        reader.set_tts_voice(SharedString::from(new_voice));
        reader.set_tts_voice_label(SharedString::from(voice_label(new_voice)));
        // best-effort: channel may be full
        let _ = if cfg.tts_lang == crate::meta::LANG_BN_BD {
            cmd_tx.send(Cmd::BnVoice(new_voice.to_string()))
        } else {
            cmd_tx.send(Cmd::Voice(new_voice.to_string()))
        };
        save_config(cfg);
        debug!("panel: voice set to {}", new_voice);
    }
}

fn handle_toggles(reader: &Reader, wifi_toggle_cell: &Cell<bool>, bt_toggle_cell: &Cell<bool>) {
    if wifi_toggle_cell.replace(false) {
        let on = !reader.get_wifi_on();
        reader.set_wifi_on(on);
        wifi_toggle(on);
    }

    if bt_toggle_cell.replace(false) {
        let on = !reader.get_bt_on();
        reader.set_bt_on(on);
        bt_toggle(on);
    }
}

fn handle_chapter_overlay(
    st: &mut LoopState,
    reader: &Reader,
    cmd_tx: &Sender<Cmd>,
    cb: &Callbacks,
) -> bool {
    let mut text_dirty = false;

    if cb.chapter_panel_cell.replace(false) && !st.picker_active {
        let items = all_chapter_items(&st.chapters);
        reader.set_chapter_items(ModelRc::new(VecModel::from(items)));
        reader.set_current_chapter_idx(st.current_chapter as i32);
        reader.set_chapter_overlay_open(true);
        debug!(
            "panel: chapter overlay opened ({} chapters, current {})",
            st.chapter_count,
            st.current_chapter + 1
        );
    }

    if let Some(nc) = cb.chapter_select_cell.replace(None) {
        debug!(
            "chapter_select consumed nc={} (current={})",
            nc, st.current_chapter
        );
        if nc != st.current_chapter && nc < st.chapter_count {
            crate::reader::switch_chapter(st, reader, cmd_tx, nc, false, false);
            text_dirty = true;
            let cn = crate::data::library::chapter_display_title(&st.chapters[nc], nc);
            crate::set_chapter_name(reader, &cn);
            debug!("chapter selected: {}", nc + 1);
        }
    }

    text_dirty
}

fn all_chapter_items(chapters: &[Chapter]) -> Vec<ChapterItem> {
    (0..chapters.len())
        .map(|idx| {
            let title = crate::clean_ws(&crate::data::library::chapter_display_title(
                &chapters[idx],
                idx,
            ));
            let (img, img_h) = crate::rendering::render::text_image(&title, 24.0, 820, 1);
            debug!(
                "ch-item[{}]: title_len={} bangla={} img_w={} img_h={} first40={:?}",
                idx,
                title.chars().count(),
                crate::has_bangla(&title),
                img.size().width,
                img_h,
                title.chars().take(40).collect::<String>()
            );
            ChapterItem {
                title: SharedString::from(title),
                index: idx as i32,
                img,
                img_h: img_h as i32,
            }
        })
        .collect()
}

fn apply_font_reflow(new_val: i32, st: &mut LoopState, reader: &Reader) {
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
    st.head_px = new_val as f32 * 0.78;
    st.line_h = (new_val as f32 * 1.4) as i32;
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
    st.chapter_offsets.clone_from(&estimate_chapter_offsets(
        &st.chapters,
        cc,
        st.state.pages.len(),
        st.line_h,
    ));
    st.offset_rx = Some(spawn_offset_computation(
        st.chapters.clone(),
        st.body_px,
        st.head_px,
        st.line_h,
        new_val,
        st.current_book_path.clone(),
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
