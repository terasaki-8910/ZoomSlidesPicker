//! Data-URL thumbnails for the webview (docs/impl/ipc.md: `WindowEntry.thumb`,
//! `Snapshot.image`, `SessionView.lastThumb`). The webview never touches files directly,
//! so every image it sees is a PNG data URL built here.

use std::io;
use std::path::Path;

use base64::Engine as _;
use base64::prelude::BASE64_STANDARD;
use zsp_capture::RgbaImage;

fn data_url(png: &[u8]) -> String {
    format!("data:image/png;base64,{}", BASE64_STANDARD.encode(png))
}

/// A PNG data URL of `img`, scaled down to at most `max_width` wide (never enlarged).
pub fn thumbnail_data_url(img: &RgbaImage, max_width: u32) -> String {
    data_url(&zsp_capture::thumbnail_png(img, max_width))
}

/// A PNG data URL of the whole image, uncropped and unscaled beyond `max_width`
/// (`get_region_snapshot`'s image, SPEC.md: at most 1600 px wide).
pub fn full_data_url(img: &RgbaImage, max_width: u32) -> String {
    let (width, _) = image::GenericImageView::dimensions(img);
    if width <= max_width {
        data_url(&zsp_capture::thumbnail_png(img, width))
    } else {
        data_url(&zsp_capture::thumbnail_png(img, max_width))
    }
}

/// Decodes a capture PNG from disk and thumbnails it (the session engine only hands the
/// notify callback a path, not the pixels -- SPEC.md's captures are read back once per
/// save to build `SessionView.lastThumb`).
pub fn file_thumbnail_data_url(path: &Path, max_width: u32) -> io::Result<String> {
    let img = image::open(path).map_err(io::Error::other)?.into_rgba8();
    Ok(thumbnail_data_url(&img, max_width))
}
