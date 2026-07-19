// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use slint::platform::software_renderer::MinimalSoftwareWindow;

use kobo_core::Capabilities;

use crate::loop_run::{LoopContext, LoopFlow, PICKER_ENTER_DEBOUNCE_MS};
use crate::loop_state::LoopState;
use crate::rendering::fb::Fb;
use crate::rendering::picker::{picker_scroll_cells, show_book_picker, PickerRefresh};
use crate::Reader;

pub(super) fn handle_about_close(
    dx: f32,
    dy: f32,
    w: usize,
    st: &mut LoopState,
    reader: &Reader,
    fb: &Fb,
    window: &MinimalSoftwareWindow,
    all_books: &[crate::data::library::EpubEntry],
    caps: &dyn Capabilities,
) -> bool {
    const CLOSE_PX: f32 = 76.0;
    const CLOSE_PAD: f32 = 23.0;
    const CLOSE_TOP: f32 = 17.0;
    let cl = w as f32 - CLOSE_PX - CLOSE_PAD;
    let cr = w as f32 - CLOSE_PAD;
    if dx >= cl && dx < cr && dy >= CLOSE_TOP && dy < CLOSE_TOP + CLOSE_PX {
        st.about_open = false;
        st.picker_last_tap_idx = None;
        st.exit_armed = false;
        show_book_picker(
            reader,
            fb,
            window,
            &mut st.buffer,
            &mut st.text_cache,
            &mut st.picker_cover_cache,
            all_books,
            st.picker_scroll,
            st.library_filter,
            &caps.current_clock(),
            caps.battery_pct(),
            "",
            PickerRefresh::Full,
        );
        st.picker_cells = picker_scroll_cells(all_books, st.picker_scroll, st.library_filter);
        st.prev_buffer.copy_from_slice(&st.buffer);
        true
    } else {
        false
    }
}
