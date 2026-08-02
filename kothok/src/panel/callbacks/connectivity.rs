// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::cell::Cell;

use slint::SharedString;

use crate::device::{
    bt_connect_device, bt_set_target_device, bt_toggle, wifi_select_network, wifi_toggle,
};
use crate::loop_state::LoopState;
use crate::Reader;

pub(super) fn ensure_wifi_bt_lists(st: &mut LoopState, reader: &Reader) {
    if let Some(rx) = st.wifi_bt_list_rx.take() {
        match rx.try_recv() {
            Ok(result) => {
                if !result.wifi.is_empty() {
                    let connected_idx = result.wifi.iter().position(|e| e.connected);
                    st.wifi_list = result.wifi.iter().map(|e| (e.ssid.clone(), e.id)).collect();
                    st.wifi_list_idx = connected_idx.unwrap_or(0);
                    st.wifi_list_ids_valid = result.wifi_ids_valid;
                    let name = &st.wifi_list[st.wifi_list_idx].0;
                    reader.set_wifi_name(SharedString::from(name.as_str()));
                } else {
                }
                st.wifi_list_fetched = true;

                if !result.bt.is_empty() {
                    let connected_idx = result.bt.iter().position(|e| e.connected);
                    st.bt_list = result
                        .bt
                        .iter()
                        .map(|e| (e.name.clone(), e.path.clone()))
                        .collect();
                    st.bt_list_idx = connected_idx.unwrap_or(0);
                    st.bt_list_ids_valid = true;
                    let name = &st.bt_list[st.bt_list_idx].0;
                    reader.set_bt_name(SharedString::from(name.as_str()));
                    if connected_idx.is_none() && reader.get_bt_on() {
                        let (_, path) = &st.bt_list[st.bt_list_idx];
                        let p = path.clone();
                        std::thread::spawn(move || bt_connect_device(&p));
                    }
                } else {
                }
                st.bt_list_fetched = true;
                return;
            }
            Err(_) => {
                st.wifi_bt_list_rx = Some(rx);
                return;
            }
        }
    }

    if !st.wifi_list_fetched || !st.bt_list_fetched {
        st.wifi_bt_list_rx = Some(crate::panel::spawn_wifi_bt_list_fetch());
    }
}

fn toggle_selector(
    reader: &Reader,
    toggle_cell: &Cell<bool>,
    _kind: &str,
    is_on: impl Fn(&Reader) -> bool,
    set_on: impl Fn(&Reader, bool),
    cur_name: impl Fn(&Reader) -> SharedString,
    connected_name: impl Fn(&Reader) -> SharedString,
    toggle_dev: impl Fn(bool),
) -> Option<bool> {
    if !toggle_cell.replace(false) {
        return None;
    }
    let connected = is_on(reader) && cur_name(reader) == connected_name(reader);
    if connected {
        toggle_dev(false);
        set_on(reader, false);
        Some(false)
    } else {
        toggle_dev(true);
        set_on(reader, true);
        Some(true)
    }
}

fn cycle_selector(
    names: &[String],
    idx: &mut usize,
    cycle_cell: &Cell<i32>,
    set_name: impl Fn(&str),
    _kind: &str,
) {
    let dir = cycle_cell.replace(0);
    if dir == 0 || names.len() < 2 {
        return;
    }
    *idx = if dir == 2 {
        if *idx == 0 {
            names.len() - 1
        } else {
            *idx - 1
        }
    } else {
        (*idx + 1) % names.len()
    };
    let name = &names[*idx];
    set_name(name);
}

pub(super) fn handle_wifi(
    reader: &Reader,
    st: &mut LoopState,
    wifi_toggle_cell: &Cell<bool>,
    wifi_cycle_cell: &Cell<i32>,
) {
    match toggle_selector(
        reader,
        wifi_toggle_cell,
        "wifi",
        |r| r.get_wifi_on(),
        |r, v| r.set_wifi_on(v),
        |r| r.get_wifi_name(),
        |r| r.get_wifi_connected_name(),
        wifi_toggle,
    ) {
        Some(true) => {
            st.wifi_user_on = true;
            if st.wifi_list_ids_valid && !st.wifi_list.is_empty() {
                let id = st.wifi_list[st.wifi_list_idx].1;
                std::thread::spawn(move || wifi_select_network(id));
            }
            if !st.wifi_list_ids_valid {
                st.wifi_list_fetched = false;
            }
        }
        Some(false) => {
            st.wifi_user_on = false;
        }
        None => {}
    }

    let names: Vec<String> = st.wifi_list.iter().map(|(s, _)| s.clone()).collect();
    cycle_selector(
        &names,
        &mut st.wifi_list_idx,
        wifi_cycle_cell,
        |n| {
            reader.set_wifi_name(SharedString::from(n));
        },
        "wifi",
    );
}

pub(super) fn handle_bt(
    reader: &Reader,
    st: &mut LoopState,
    bt_toggle_cell: &Cell<bool>,
    bt_cycle_cell: &Cell<i32>,
) {
    // Pin the user's list-selected device before `bt_toggle(true)` fires, so
    // its `reconnect_bt` thread targets it directly instead of falling back
    // to cached/last-known/default. Doing this here (rather than a separate
    // `bt_connect_device` call after) keeps a single connect thread in
    // flight instead of two racing to Connect the same device.
    let selected_path = st.bt_list.get(st.bt_list_idx).map(|(_, p)| p.clone());
    match toggle_selector(
        reader,
        bt_toggle_cell,
        "bt",
        |r| r.get_bt_on(),
        |r, v| r.set_bt_on(v),
        |r| r.get_bt_name(),
        |r| r.get_bt_connected_name(),
        |on| {
            if on {
                if let Some(path) = &selected_path {
                    bt_set_target_device(path);
                }
            }
            bt_toggle(on);
        },
    ) {
        Some(true) => {
            st.bt_user_on = true;
            st.bt_list_fetched = false;
            // Clear the poll's failure streak, or a count left over from the
            // disconnect that prompted this toggle re-greys the play button on
            // the very next hiccuped poll -- the debounce failing exactly when
            // the user has just acted.
            st.bt_fail_count = 0;
        }
        Some(false) => {
            st.bt_user_on = false;
        }
        None => {}
    }

    let names: Vec<String> = st.bt_list.iter().map(|(n, _)| n.clone()).collect();
    cycle_selector(
        &names,
        &mut st.bt_list_idx,
        bt_cycle_cell,
        |n| {
            reader.set_bt_name(SharedString::from(n));
        },
        "bt",
    );
}
