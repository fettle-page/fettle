//! The logo on printouts: a PNG or JPEG the user chooses, checked and made small once (a PNG that
//! fits 800 × 400 pixels), then kept as logo.png and drawn at the top of page 1.

use image::{DynamicImage, ImageDecoder, ImageFormat, ImageReader, Limits};
use std::io::Cursor;

/// The biggest file taken, chosen or read back.
pub const MAX_BYTES: usize = 2 * 1024 * 1024;
/// The prepared logo fits in this many pixels across and down.
pub const MAX_SIZE: (u32, u32) = (800, 400);

/// What the app says when a picture isn't taken (app string keys).
pub const TOO_BIG: &str = "settings.logo.e.too_big";
pub const NOT_A_PICTURE: &str = "settings.logo.e.type";
pub const UNREADABLE: &str = "settings.logo.e.unreadable";

fn format_of(bytes: &[u8]) -> Option<ImageFormat> {
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some(ImageFormat::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(ImageFormat::Jpeg)
    } else {
        None
    }
}

/// Decodes with limits, so a small file that claims a huge picture can't take the PC's memory.
fn decode(bytes: &[u8], format: ImageFormat) -> Option<DynamicImage> {
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = Limits::default();
    limits.max_image_width = Some(12_000);
    limits.max_image_height = Some(12_000);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    let mut decoder = reader.into_decoder().ok()?;
    // A phone photo of a logo is often stored sideways, with a note saying which way is up.
    let orientation = decoder.orientation().ok();
    let mut picture = DynamicImage::from_decoder(decoder).ok()?;
    if let Some(o) = orientation {
        picture.apply_orientation(o);
    }
    Some(picture)
}

/// A chosen picture made ready to keep: PNG or JPEG up to 2 MB, scaled down to fit 800 × 400 pixels
/// (never up), as a PNG. Err is an app string key for a plain message; nothing is kept then.
pub fn prepare(bytes: &[u8]) -> Result<Vec<u8>, &'static str> {
    if bytes.len() > MAX_BYTES {
        return Err(TOO_BIG);
    }
    let format = format_of(bytes).ok_or(NOT_A_PICTURE)?;
    let mut picture = decode(bytes, format).ok_or(UNREADABLE)?;
    if picture.width() == 0 || picture.height() == 0 {
        return Err(UNREADABLE);
    }
    if picture.width() > MAX_SIZE.0 || picture.height() > MAX_SIZE.1 {
        picture = picture.resize(MAX_SIZE.0, MAX_SIZE.1, image::imageops::FilterType::Lanczos3);
    }
    let mut out = Vec::new();
    DynamicImage::ImageRgba8(picture.to_rgba8()).write_to(&mut Cursor::new(&mut out), ImageFormat::Png).map_err(|_| UNREADABLE)?;
    Ok(out)
}

/// logo.png as read back from the data folder, if it's still a PNG Fettle can draw (not over 2 MB,
/// and it decodes). Anything else prints as no logo.
pub fn readable(bytes: Vec<u8>) -> Option<Vec<u8>> {
    (bytes.len() <= MAX_BYTES && format_of(&bytes) == Some(ImageFormat::Png) && decode(&bytes, ImageFormat::Png).is_some()).then_some(bytes)
}

/// A PNG's size in pixels, from its header.
pub fn size(png: &[u8]) -> Option<(u32, u32)> {
    let (w, h) = ImageReader::with_format(Cursor::new(png), ImageFormat::Png).into_dimensions().ok()?;
    (w > 0 && h > 0).then_some((w, h))
}

/// A PNG as pixels on white, top row first, each 0x00RRGGBB (paper has no transparency).
pub fn pixels_on_white(png: &[u8]) -> Option<(u32, u32, Vec<u32>)> {
    let rgba = decode(png, ImageFormat::Png)?.to_rgba8();
    let (w, h) = rgba.dimensions();
    let px = rgba
        .pixels()
        .map(|p| {
            let [r, g, b, a] = p.0;
            let on_white = |c: u8| (u32::from(c) * u32::from(a) + 255 * (255 - u32::from(a)) + 127) / 255;
            (on_white(r) << 16) | (on_white(g) << 8) | on_white(b)
        })
        .collect();
    Some((w, h, px))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use image::{ImageEncoder, RgbaImage};

    /// A made-up picture: a red square on transparent, `w` × `h`.
    pub(crate) fn picture(w: u32, h: u32) -> RgbaImage {
        RgbaImage::from_fn(w, h, |x, y| if x < w / 2 && y < h / 2 { image::Rgba([0xD9, 0x48, 0x1E, 255]) } else { image::Rgba([0, 0, 0, 0]) })
    }

    pub(crate) fn png(w: u32, h: u32) -> Vec<u8> {
        let mut out = Vec::new();
        image::codecs::png::PngEncoder::new(&mut out).write_image(picture(w, h).as_raw(), w, h, image::ExtendedColorType::Rgba8).unwrap();
        out
    }

    fn jpeg(w: u32, h: u32) -> Vec<u8> {
        let rgb = DynamicImage::ImageRgba8(picture(w, h)).to_rgb8();
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85).write_image(rgb.as_raw(), w, h, image::ExtendedColorType::Rgb8).unwrap();
        out
    }

    #[test]
    fn a_small_png_is_kept_at_its_size() {
        let out = prepare(&png(120, 40)).unwrap();
        assert_eq!(size(&out), Some((120, 40)));
        assert_eq!(readable(out.clone()), Some(out));
    }

    #[test]
    fn a_jpeg_becomes_a_png() {
        let out = prepare(&jpeg(300, 100)).unwrap();
        assert!(out.starts_with(b"\x89PNG"));
        assert_eq!(size(&out), Some((300, 100)));
    }

    #[test]
    fn a_big_picture_is_scaled_down_to_fit_keeping_its_shape() {
        assert_eq!(size(&prepare(&png(2400, 600)).unwrap()), Some((800, 200)));
        assert_eq!(size(&prepare(&png(500, 1000)).unwrap()), Some((200, 400)));
    }

    #[test]
    fn anything_else_is_refused_with_a_plain_message() {
        assert_eq!(prepare(b"GIF89a not taken"), Err(NOT_A_PICTURE));
        assert_eq!(prepare(b""), Err(NOT_A_PICTURE));
        let mut broken = png(50, 50);
        broken.truncate(40);
        assert_eq!(prepare(&broken), Err(UNREADABLE));
        assert_eq!(prepare(b"\xFF\xD8\xFF\xE0 not really a jpeg"), Err(UNREADABLE));
        let mut big = png(10, 10);
        big.resize(MAX_BYTES + 1, 0);
        assert_eq!(prepare(&big), Err(TOO_BIG));
    }

    #[test]
    fn a_logo_read_back_that_is_too_big_or_broken_is_no_logo() {
        let mut big = png(10, 10);
        big.resize(MAX_BYTES + 1, 0);
        assert_eq!(readable(big), None);
        assert_eq!(readable(b"\x89PNG\r\n\x1a\n not really".to_vec()), None);
        assert_eq!(readable(jpeg(10, 10)), None, "only the PNG Fettle wrote");
    }

    #[test]
    fn transparent_parts_print_white() {
        let (w, h, px) = pixels_on_white(&png(4, 4)).unwrap();
        assert_eq!((w, h), (4, 4));
        assert_eq!(px[0], 0xD9481E);
        assert_eq!(px[15], 0xFFFFFF);
    }
}
