use slint::platform::software_renderer::Rgb565Pixel;

pub use kobo_core::rendering::common::{is_rtl, set_rtl};

pub const ACCENT_BAR_RGB565: u16 = 0x0000;

#[allow(dead_code)]
pub const BRAND_GREEN_RGB565: u16 = 0x0349;
#[allow(dead_code)]
pub const BRAND_RED_RGB565: u16 = 0xF148;
#[allow(dead_code)]
pub const TEXT_PRIMARY_RGB565: u16 = 0x1082;
#[allow(dead_code)]
pub const TEXT_HINT_RGB565: u16 = 0x94B2;
#[allow(dead_code)]
pub const TRACK_RGB565: u16 = 0xD6BA;

pub(crate) fn rgb565_as_bytes(buf: &mut [Rgb565Pixel]) -> &mut [u8] {
    kobo_core::rendering::common::slice_as_bytes_mut(buf)
}

pub(crate) fn rgb565_as_bytes_ref(buf: &[Rgb565Pixel]) -> &[u8] {
    kobo_core::rendering::common::slice_as_bytes(buf)
}
