use std::cell::Cell;
use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::Reader;

pub struct Callbacks {
    pub page_delta: Rc<Cell<i32>>,
    pub quit: Rc<Cell<bool>>,
    pub picker_scroll_delta: Rc<Cell<i32>>,
    pub exit_app: Rc<Cell<bool>>,
    pub panel_open_cell: Rc<Cell<bool>>,
    pub progress_target: Rc<Cell<i32>>,
    pub panel_voice_cell: Rc<Cell<i32>>,
    pub panel_frac: Rc<Cell<Option<(i32, f32)>>>,
    pub font_frac_in: Rc<Cell<Option<f32>>>,
    pub wifi_toggle_cell: Rc<Cell<bool>>,
    pub bt_toggle_cell: Rc<Cell<bool>>,
    pub wifi_cycle_cell: Rc<Cell<i32>>,
    pub bt_cycle_cell: Rc<Cell<i32>>,
    pub play_toggle_cell: Rc<Cell<bool>>,
    pub chapter_panel_cell: Rc<Cell<bool>>,
    pub chapter_select_cell: Rc<Cell<Option<usize>>>,
    pub jump_to_reading_cell: Rc<Cell<bool>>,
    pub font_pending_val: Rc<Cell<Option<i32>>>,
    pub font_last_change: Rc<Cell<Option<std::time::Instant>>>,
}

struct ChapterCells {
    panel_cell: Rc<Cell<bool>>,
    select_cell: Rc<Cell<Option<usize>>>,
    jump_cell: Rc<Cell<bool>>,
}

fn register_chapter(reader: &Reader, panel_open_cell: &Rc<Cell<bool>>) -> ChapterCells {
    let panel_cell = Rc::new(Cell::new(false));
    let select_cell = Rc::new(Cell::new(None::<usize>));
    let jump_cell = Rc::new(Cell::new(false));

    let cp_jtr = jump_cell.clone();
    let cp_jtr_panel = panel_open_cell.clone();
    reader.on_jump_to_reading(move || {
        cp_jtr_panel.set(false);
        cp_jtr.set(true);
    });

    let cp = panel_cell.clone();
    let cp_ch_panel = panel_open_cell.clone();
    reader.on_panel_chapters(move || {
        cp.set(true);
    });

    let cp_ch_sel = select_cell.clone();
    let reader_clone = reader.as_weak();
    reader.on_chapter_selected(move |idx: i32| {
        log::debug!("chapter-selected callback idx={}", idx);
        let Some(reader) = reader_clone.upgrade() else {
            return;
        };
        let idx = idx as usize;
        if idx < reader.get_chapter_count() as usize {
            reader.set_chapter_overlay_open(false);
            reader.set_chapter_preview_idx(-1);
            reader.set_chapter_pending(-1);
            cp_ch_panel.set(false);
            cp_ch_sel.set(Some(idx));
        }
    });

    ChapterCells {
        panel_cell,
        select_cell,
        jump_cell,
    }
}

pub fn register(reader: &Reader) -> Callbacks {
    let page_delta = Rc::new(Cell::new(0i32));
    let quit = Rc::new(Cell::new(false));
    let q = quit.clone();
    reader.on_quit(move || {
        q.set(true);
    });

    let picker_scroll_delta = Rc::new(Cell::new(0i32));
    let exit_app = Rc::new(Cell::new(false));

    let panel_open_cell = Rc::new(Cell::new(false));
    let poc = panel_open_cell.clone();
    reader.on_panel_close(move || {
        poc.set(false);
    });

    let progress_target = Rc::new(Cell::new(-1i32));
    let pt = progress_target.clone();
    reader.on_progress_tap(move |frac: f32| {
        let pm = (frac.clamp(0.0, 1.0) * 1000.0) as i32;
        pt.set(pm);
    });

    let panel_voice_cell = Rc::new(Cell::new(0i32));
    let pv = panel_voice_cell.clone();
    reader.on_panel_voice(move |dir: SharedString| {
        pv.set(if dir == "prev" { 2 } else { 1 });
    });

    let panel_frac = Rc::new(Cell::new(None::<(i32, f32)>));
    let pf = panel_frac.clone();
    let font_frac_in = Rc::new(Cell::new(None::<f32>));
    let ffi = font_frac_in.clone();
    reader.on_panel_frac(move |which: i32, frac: f32| {
        let frac = frac.clamp(0.0, 1.0);
        if which == 2 {
            ffi.set(Some(frac));
        } else {
            pf.set(Some((which, frac)));
        }
    });

    let wifi_toggle_cell = Rc::new(Cell::new(false));
    let wt = wifi_toggle_cell.clone();
    reader.on_panel_wifi_toggle(move || {
        wt.set(true);
    });

    let bt_toggle_cell = Rc::new(Cell::new(false));
    let bt = bt_toggle_cell.clone();
    reader.on_panel_bt_toggle(move || {
        bt.set(true);
    });

    let wifi_cycle_cell = Rc::new(Cell::new(0i32));
    let wc = wifi_cycle_cell.clone();
    reader.on_panel_wifi_cycle(move |dir: SharedString| {
        wc.set(if dir == "prev" { 2 } else { 1 });
    });

    let bt_cycle_cell = Rc::new(Cell::new(0i32));
    let bc = bt_cycle_cell.clone();
    reader.on_panel_bt_cycle(move |dir: SharedString| {
        bc.set(if dir == "prev" { 2 } else { 1 });
    });

    let play_toggle_cell = Rc::new(Cell::new(false));
    let ppc = play_toggle_cell.clone();
    reader.on_play_pause_toggle(move || {
        ppc.set(true);
    });

    let chapter = register_chapter(reader, &panel_open_cell);
    let font_pending_val = Rc::new(Cell::new(None::<i32>));
    let font_last_change = Rc::new(Cell::new(None::<std::time::Instant>));

    Callbacks {
        page_delta,
        quit,
        picker_scroll_delta,
        exit_app,
        panel_open_cell,
        progress_target,
        panel_voice_cell,
        panel_frac,
        font_frac_in,
        wifi_toggle_cell,
        bt_toggle_cell,
        wifi_cycle_cell,
        bt_cycle_cell,
        play_toggle_cell,
        chapter_panel_cell: chapter.panel_cell,
        chapter_select_cell: chapter.select_cell,
        jump_to_reading_cell: chapter.jump_cell,
        font_pending_val,
        font_last_change,
    }
}
