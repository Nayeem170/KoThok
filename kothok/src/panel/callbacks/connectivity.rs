// SPDX-License-Identifier: PolyForm-Noncommercial-1.0.0
// Copyright (c) 2026 Nayeem Bin Ahsan
use std::cell::Cell;

use log::debug;

use slint::SharedString;

use crate::device::{bt_connect_device, bt_toggle, wifi_select_network, wifi_toggle};
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
                    debug!(
                        "wifi-list: {} networks, selected [{}] {} (ids_valid={})",
                        st.wifi_list.len(),
                        st.wifi_list_idx,
                        name,
                        st.wifi_list_ids_valid
                    );
                } else {
                    debug!("wifi-list: empty, single-line mode");
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
                    debug!(
                        "bt-list: {} devices, selected [{}] {}",
                        st.bt_list.len(),
                        st.bt_list_idx,
                        name
                    );
                    if connected_idx.is_none() && reader.get_bt_on() {
                        let (_, path) = &st.bt_list[st.bt_list_idx];
                        bt_connect_device(path);
                        debug!("bt-list: list arrived, connecting to [{}]", st.bt_list_idx);
                    }
                } else {
                    debug!("bt-list: empty, single-line mode");
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
    kind: &str,
    is_on: impl Fn(&Reader) -> bool,
    set_on: impl Fn(&Reader, bool),
    cur_name: impl Fn(&Reader) -> SharedString,
    connected_name: impl Fn(&Reader) -> SharedString,
    toggle_dev: impl Fn(bool),
) -> bool {
    if !toggle_cell.replace(false) {
        return false;
    }
    let connected = is_on(reader) && cur_name(reader) == connected_name(reader);
    if connected {
        toggle_dev(false);
        set_on(reader, false);
        debug!("{}-toggle: turning off (was connected to selected)", kind);
        false
    } else {
        toggle_dev(true);
        set_on(reader, true);
        true
    }
}

fn cycle_selector(
    names: &[String],
    idx: &mut usize,
    cycle_cell: &Cell<i32>,
    set_name: impl Fn(&str),
    kind: &str,
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
    debug!(
        "{}-cycle: {}/{} -> {} ({})",
        kind,
        *idx + 1,
        names.len(),
        name,
        if dir == 2 { "prev" } else { "next" }
    );
    set_name(name);
}

pub(super) fn handle_wifi(
    reader: &Reader,
    st: &mut LoopState,
    wifi_toggle_cell: &Cell<bool>,
    wifi_cycle_cell: &Cell<i32>,
) {
    if toggle_selector(
        reader,
        wifi_toggle_cell,
        "wifi",
        |r| r.get_wifi_on(),
        |r, v| r.set_wifi_on(v),
        |r| r.get_wifi_name(),
        |r| r.get_wifi_connected_name(),
        wifi_toggle,
    ) {
        if st.wifi_list_ids_valid && !st.wifi_list.is_empty() {
            let (_, id) = &st.wifi_list[st.wifi_list_idx];
            wifi_select_network(*id);
        }
        if !st.wifi_list_ids_valid {
            st.wifi_list_fetched = false;
        }
        debug!(
            "wifi-toggle: connecting to selected [{}] {}",
            st.wifi_list_idx,
            reader.get_wifi_name()
        );
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
    if toggle_selector(
        reader,
        bt_toggle_cell,
        "bt",
        |r| r.get_bt_on(),
        |r, v| r.set_bt_on(v),
        |r| r.get_bt_name(),
        |r| r.get_bt_connected_name(),
        bt_toggle,
    ) {
        if !st.bt_list.is_empty() {
            let (_, path) = &st.bt_list[st.bt_list_idx];
            bt_connect_device(path);
            debug!(
                "bt-toggle: connecting to selected [{}] {}",
                st.bt_list_idx,
                reader.get_bt_name()
            );
        }
        st.bt_list_fetched = false;
        debug!("bt-toggle: adapter on, re-fetching device list");
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
