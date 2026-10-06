//! Crop regions, cropping, and thumbnails.

use zsp_capture::{PixelRect, Region, RgbaImage, crop, thumbnail_png};

fn px(x: u32, y: u32, w: u32, h: u32) -> PixelRect {
    PixelRect { x, y, w, h }
}

#[test]
fn validate_accepts_inside_and_rejects_outside() {
    assert!(Region::FULL.validate().is_ok());
    assert!(Region { x: 0.1, y: 0.2, w: 0.5, h: 0.5 }.validate().is_ok());
    assert!(Region { x: 0.5, y: 0.0, w: 0.5, h: 1.0 }.validate().is_ok());
    assert!(Region { x: 0.9, y: 0.9, w: 0.2, h: 0.2 }.validate().is_err());
    assert!(Region { x: -0.1, y: 0.0, w: 0.5, h: 0.5 }.validate().is_err());
    assert!(Region { x: 0.0, y: 0.0, w: 0.0, h: 0.5 }.validate().is_err());
    assert!(Region { x: f64::NAN, y: 0.0, w: 0.5, h: 0.5 }.validate().is_err());
}

#[test]
fn to_pixels_rounds_edges_and_clamps() {
    assert_eq!(Region::FULL.to_pixels(1000, 700), px(0, 0, 1000, 700));
    assert_eq!(Region { x: 0.1, y: 0.2, w: 0.5, h: 0.5 }.to_pixels(1000, 700), px(100, 140, 500, 350));
    // Edges round independently: 333.3 -> 333, 666.6 -> 667.
    assert_eq!(Region { x: 0.3333, y: 0.0, w: 0.3333, h: 1.0 }.to_pixels(1000, 700), px(333, 0, 334, 700));
    // Out-of-range regions are clamped to the image.
    assert_eq!(Region { x: 0.9, y: 0.9, w: 0.2, h: 0.2 }.to_pixels(1000, 700), px(900, 630, 100, 70));
    // Never smaller than 1 x 1.
    let tiny = Region { x: 0.5, y: 0.5, w: 1e-6, h: 1e-6 }.to_pixels(1000, 700);
    assert!(tiny.w >= 1 && tiny.h >= 1);
    // A region that follows a window resize: same fractions, new pixels.
    let r = Region { x: 0.25, y: 0.25, w: 0.5, h: 0.5 };
    assert_eq!(r.to_pixels(2000, 1400), px(500, 350, 1000, 700));
}

#[test]
fn crop_takes_the_region_pixels() {
    let img = RgbaImage::from_fn(10, 8, |x, y| image::Rgba([x as u8 * 10, y as u8 * 10, 7, 255]));
    let out = crop(&img, &Region { x: 0.2, y: 0.25, w: 0.5, h: 0.5 });
    assert_eq!(out.dimensions(), (5, 4));
    assert_eq!(out.get_pixel(0, 0), img.get_pixel(2, 2));
    assert_eq!(out.get_pixel(4, 3), img.get_pixel(6, 5));
}

#[test]
fn thumbnail_scales_down_only() {
    let big = RgbaImage::from_pixel(400, 200, image::Rgba([10, 20, 30, 255]));
    let png = thumbnail_png(&big, 100);
    let back = image::load_from_memory(&png).unwrap();
    assert_eq!((back.width(), back.height()), (100, 50));

    let small = RgbaImage::from_pixel(50, 20, image::Rgba([10, 20, 30, 255]));
    let back = image::load_from_memory(&thumbnail_png(&small, 100)).unwrap();
    assert_eq!((back.width(), back.height()), (50, 20));
}

/// Needs a GUI session (and Screen Recording permission on macOS). Run by hand:
/// cargo test -p zsp-capture --test region -- --ignored
#[test]
#[ignore = "needs a GUI session and screen-recording permission"]
fn lists_and_captures_a_real_window() {
    let windows = zsp_capture::list_windows().unwrap();
    assert!(!windows.is_empty(), "no capturable windows listed");
    assert!(windows.iter().all(|w| w.width >= 200 && w.height >= 120 && !w.title.is_empty()));
    let first = &windows[0];
    let cap = zsp_capture::WindowCapture::open(first.id).unwrap();
    let img = cap.capture().unwrap();
    assert!(img.width() >= first.width, "captured in pixels, at least the point size");
}
