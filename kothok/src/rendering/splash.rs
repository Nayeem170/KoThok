use slint::platform::software_renderer::Rgb565Pixel;

use crate::rendering::common::rgb565_as_bytes;
use crate::rendering::text_render;

const SPLASH_PNG: &[u8] = include_bytes!("../../ui/splash-portrait.png");

pub(crate) fn splash_png() -> &'static [u8] {
    SPLASH_PNG
}

pub fn paint_kothok_splash(buffer: &mut [Rgb565Pixel]) {
    let w = crate::w();
    let h = crate::h();
    buffer.fill(Rgb565Pixel(0xFFFF));
    let buf = rgb565_as_bytes(buffer);
    if let Some(img) = text_render::decode_image(SPLASH_PNG, w, h) {
        text_render::blit_rgb565_image(buf, w, &img.rgb, img.width, img.height, 0, 0, w, h);
    }
}
