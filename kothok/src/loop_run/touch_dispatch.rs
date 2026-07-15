use super::*;

use crate::device::input::{
    ABS_MT_POSITION_X, ABS_MT_POSITION_Y, BTN_TOUCH_CODE, EV_ABS, EV_KEY, EV_SYN, SYN_REPORT,
};
use std::io::Read;

pub(super) fn poll_and_dispatch_touch(st: &mut LoopState, ctx: &mut LoopContext) -> bool {
    let reader = ctx.reader;
    let cb = ctx.cb;
    let to_display = |rx: i32, ry: i32| -> (f32, f32) { touch::to_display(rx, ry, ctx.touch_cfg) };
    let pbar_y: f32 = ctx.h as f32 - layout::FOOTER_H_F;
    let pbar_x: f32 = (layout::PAD_LEFT + layout::GUTTER_W + layout::GUTTER_PAD) as f32 + 8.0;
    let pbar_w: f32 = ctx.w as f32 - 200.0;
    let pbar_right: f32 = pbar_x + pbar_w;
    let pp_zone_x: f32 = pbar_right;
    let tap_cooldown = std::time::Duration::from_millis(TAP_COOLDOWN_MS);
    let mut rec = [0u8; 16];
    let mut had_event = false;
    {
        let mut pfd = libc::pollfd {
            fd: ctx.touch_fd,
            events: libc::POLLIN,
            revents: 0,
        };
        const LOOP_POLL_MS: libc::c_int = 100;
        // SAFETY: poll takes a single initialized `pollfd` on our stack frame pointing
        // at the caller-owned touch_fd. It only writes `revents` (no aliasing) and
        // returns an int count; LOOP_POLL_MS bounds the wait.
        unsafe {
            libc::poll(&mut pfd, 1, LOOP_POLL_MS);
        }
    }
    loop {
        match ctx.touch_dev.read(&mut rec) {
            Ok(16) => {
                had_event = true;
                let typ = u16::from_le_bytes([rec[8], rec[9]]);
                let code = u16::from_le_bytes([rec[10], rec[11]]);
                let val = i32::from_le_bytes([rec[12], rec[13], rec[14], rec[15]]);
                match (typ, code) {
                    (EV_KEY, BTN_TOUCH_CODE) => st.frame_down = val == 1,
                    (EV_ABS, ABS_MT_POSITION_X) => st.frame_x = val,
                    (EV_ABS, ABS_MT_POSITION_Y) => st.frame_y = val,
                    (EV_SYN, SYN_REPORT) => {
                        let now = std::time::Instant::now();
                        let (dx, dy) = to_display(st.frame_x, st.frame_y);
                        if st.frame_down && !st.prev_down {
                            st.press_x = st.frame_x;
                            st.press_y = st.frame_y;
                            st.press_time = now;
                            let footer_zone = if !st.picker_active
                                && !st.panel_open
                                && !reader.get_chapter_overlay_open()
                            {
                                gesture::classify_footer_zone(
                                    dx, dy, pbar_y, PBAR_H, pbar_x, pbar_right, pp_zone_x,
                                )
                            } else {
                                gesture::FooterZone::None
                            };
                            match footer_zone {
                                gesture::FooterZone::PlayPause => {
                                    st.pp_pressed = true;
                                }
                                gesture::FooterZone::ProgressBar => {
                                    let frac = ((dx - pbar_x) / pbar_w).clamp(0.0, 1.0);
                                    cb.progress_target.set((frac * 1000.0) as i32);
                                    st.scrubbing = true;
                                    debug!("pbar: scrub start frac={:.2}", frac);
                                }
                                gesture::FooterZone::None => {}
                            }
                            let header_zone = if !st.picker_active
                                && !st.panel_open
                                && !reader.get_chapter_overlay_open()
                                && !st.cover_page_visible
                                && st.header_visible
                            {
                                gesture::classify_header_zone(dx, dy, ctx.w as f32)
                            } else {
                                gesture::HeaderZone::None
                            };
                            match header_zone {
                                gesture::HeaderZone::Library => st.lib_pressed = true,
                                gesture::HeaderZone::Menu => st.menu_pressed = true,
                                gesture::HeaderZone::None => {}
                            }
                            let near = st.last_tap_y >= 0
                                && (st.frame_y - st.last_tap_y).abs() < SWIPE_DELTA_TOLERANCE_PX;
                            if (now.duration_since(st.last_tap_time) >= tap_cooldown || !near)
                                && !st.scrubbing
                                && !st.pp_pressed
                                && !st.lib_pressed
                                && !st.menu_pressed
                            {
                                st.tap_xy = Some((dx, dy));
                                st.last_tap_time = now;
                                st.last_tap_y = st.frame_y;
                                if !st.picker_active {
                                    if !st.panel_open && !reader.get_chapter_overlay_open() {
                                        st.press_dispatched = false;
                                    } else {
                                        st.press_dispatched = true;
                                        ctx.window.window().dispatch_event(
                                            slint::platform::WindowEvent::PointerPressed {
                                                position: slint::LogicalPosition::new(dx, dy),
                                                button: slint::platform::PointerEventButton::Left,
                                            },
                                        );
                                    }
                                } else {
                                    st.press_dispatched = false;
                                }
                            }
                        } else if !st.frame_down && st.prev_down {
                            touch_release::on_release(st, ctx, dx, dy, now);
                        } else if st.frame_down {
                            if st.scrubbing {
                                let frac = ((dx - pbar_x) / pbar_w).clamp(0.0, 1.0);
                                cb.progress_target.set((frac * 1000.0) as i32);
                            } else {
                                ctx.window.window().dispatch_event(
                                    slint::platform::WindowEvent::PointerMoved {
                                        position: slint::LogicalPosition::new(dx, dy),
                                    },
                                );
                            }
                        }
                        if !st.frame_down {
                            st.scrubbing = false;
                        }
                        st.prev_down = st.frame_down;
                    }
                    _ => {}
                }
            }
            _ => break,
        }
    }
    had_event
}
