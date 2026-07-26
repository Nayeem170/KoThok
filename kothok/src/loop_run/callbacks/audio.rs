// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use crate::loop_state::LoopState;
use crate::Reader;

const COVER_ROTATION_STEP: f32 = crate::rendering::vinyl::MARKER_STEP_DEG;
const COVER_ROTATION_TICK_MS: u64 = 250;

pub(super) fn advance_cover_rotation(st: &mut LoopState, reader: &Reader) -> bool {
    let playing = reader.get_playing() && !st.panel_open && !st.picker_active;
    if st.prev_playing && !playing {
        st.disk_settle = true;
    }
    st.prev_playing = playing;
    if !playing {
        return false;
    }
    let tick = std::time::Duration::from_millis(COVER_ROTATION_TICK_MS);
    if st.last_cover_rot.elapsed() < tick {
        return false;
    }
    st.last_cover_rot = std::time::Instant::now();
    st.prev_cover_rotation = st.cover_rotation;
    st.cover_rotation = (st.cover_rotation + COVER_ROTATION_STEP).rem_euclid(360.0);
    st.disk_spin_only = true;
    true
}

pub(super) fn refresh_audio_disk(
    st: &mut LoopState,
    reader: &Reader,
    ring_frac: f32,
    screen_w: usize,
) {
    let rotation_step = (st.cover_rotation / COVER_ROTATION_STEP).round() as i32;
    let key = (
        st.current_book_path.clone(),
        st.current_chapter,
        st.current_page,
        rotation_step,
    );
    if st.disk_key.as_ref() == Some(&key) {
        return;
    }
    if let Some((path, ch, pg, _)) = st.disk_key.as_ref() {
        if path != &key.0 || *ch != key.1 || *pg != key.2 {
            st.disk_spin_only = false;
        }
    } else {
        st.disk_spin_only = false;
    }
    st.disk_key = Some(key);
    let title = reader.get_book_title().to_string();
    let author = reader.get_book_author().to_string();
    let size = crate::rendering::vinyl::disk_px(screen_w);
    if st.disk_cover_path != st.current_book_path {
        st.disk_cover_path = st.current_book_path.clone();
        st.disk_cover = crate::rendering::render::disk_cover(&st.current_book_path, size);
    }
    let cover = st.disk_cover.as_ref();
    reader.set_audio_disk_img(crate::rendering::vinyl::render_vinyl_disk(
        cover,
        &title,
        &author,
        ring_frac,
        st.cover_rotation,
        size,
    ));
}
