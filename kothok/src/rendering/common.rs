use slint::platform::software_renderer::Rgb565Pixel;

pub use kobo_core::rendering::common::{is_rtl, set_rtl};

pub const ACCENT_BAR_RGB565: u16 = 0x0953;

pub(crate) fn rgb565_as_bytes(buf: &mut [Rgb565Pixel]) -> &mut [u8] {
    kobo_core::rendering::common::slice_as_bytes_mut(buf)
}

pub(crate) fn rgb565_as_bytes_ref(buf: &[Rgb565Pixel]) -> &[u8] {
    kobo_core::rendering::common::slice_as_bytes(buf)
}
