//! Operation icons as images the window can draw (ICON-004, UI-013): PNG, JPEG or ICO, decoded
//! once and shrunk to the pixels they cover.

use std::path::Path;

use slint::{Image, Rgba8Pixel, SharedPixelBuffer};

/// The image at `path`, shrunk to fit `pixels` square when it is larger; `None` when it cannot
/// be read as an image. An ICO gives its largest image.
pub fn load_scaled(path: &Path, pixels: u32) -> Option<Image> {
    let decoded = image::open(path).ok()?;
    let fitted = if pixels > 0 && decoded.width().max(decoded.height()) > pixels {
        decoded.thumbnail(pixels, pixels)
    } else {
        decoded
    };
    let rgba = fitted.into_rgba8();
    let buffer = SharedPixelBuffer::<Rgba8Pixel>::clone_from_slice(
        rgba.as_raw(),
        rgba.width(),
        rgba.height(),
    );
    Some(Image::from_rgba8(buffer))
}
