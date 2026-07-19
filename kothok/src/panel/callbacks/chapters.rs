// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::sync::mpsc::Sender;

use log::debug;

use slint::{ModelRc, SharedString, VecModel};

use kobo_core::Chapter;

use crate::audio::Cmd;
use crate::callbacks::Callbacks;
use crate::loop_state::LoopState;
use crate::{ChapterItem, Reader};

pub(super) fn handle_chapter_overlay(
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
            crate::reader::switch_chapter(
                st,
                reader,
                cmd_tx,
                nc,
                crate::reader::ChapterSwitchOpts {
                    to_last_page: false,
                    update_cursor: false,
                    load_audio: true,
                },
            );
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
