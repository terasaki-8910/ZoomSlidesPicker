//! ACCEPTANCE B1: analysis grids and detail match the reference implementation.

mod common;

use common::{fixtures_dir, read_json};
use zsp_detect::{GRID_W, analyze, grid_shape};

#[test]
fn grid_shape_follows_aspect_ratio_and_clamps() {
    assert_eq!(grid_shape(1280, 720), (GRID_W, 90));
    assert_eq!(grid_shape(960, 720), (GRID_W, 120));
    assert_eq!(grid_shape(1270, 698), (GRID_W, 88));
    assert_eq!(grid_shape(2000, 1400), (GRID_W, 112));
    assert_eq!(grid_shape(1000, 50), (GRID_W, 30));
    assert_eq!(grid_shape(100, 1000), (GRID_W, 240));
}

#[test]
fn b1_grid_and_detail_match_reference() {
    let dir = fixtures_dir().join("frames");
    let index = read_json(&dir.join("index.json"));
    let scale = index["detail_scale"].as_f64().unwrap() as f32;
    let frames = index["frames"].as_object().unwrap();
    assert!(frames.len() >= 8);

    for (name, meta) in frames {
        let rgb = image::open(dir.join(format!("{name}.png")))
            .unwrap()
            .into_rgb8();
        let (w, h) = rgb.dimensions();
        assert_eq!(w as u64, meta["width"].as_u64().unwrap());
        let rgba: Vec<u8> = rgb.pixels().flat_map(|p| [p[0], p[1], p[2], 255]).collect();

        let (grid, detail) = analyze(&rgba, w, h);
        let gw = meta["grid_w"].as_u64().unwrap() as usize;
        let gh = meta["grid_h"].as_u64().unwrap() as usize;
        assert_eq!((grid.width, grid.height), (gw, gh), "{name}: grid shape");
        assert_eq!(
            (detail.width, detail.height),
            (gw, gh),
            "{name}: detail shape"
        );

        let ref_grid = image::open(dir.join(format!("{name}.grid.png")))
            .unwrap()
            .into_luma8()
            .into_raw();
        let diffs: Vec<i32> = grid
            .data
            .iter()
            .zip(&ref_grid)
            .map(|(a, b)| (*a as i32 - *b as i32).abs())
            .collect();
        let mean = diffs.iter().sum::<i32>() as f64 / diffs.len() as f64;
        let max = *diffs.iter().max().unwrap();
        assert!(mean <= 1.0, "{name}: grid mean |diff| {mean:.3} > 1.0");
        assert!(max <= 8, "{name}: grid max |diff| {max} > 8");

        let ref_detail: Vec<f32> = image::open(dir.join(format!("{name}.detail.png")))
            .unwrap()
            .into_luma16()
            .into_raw()
            .into_iter()
            .map(|v| v as f32 / scale)
            .collect();
        let rel: f64 = detail
            .data
            .iter()
            .zip(&ref_detail)
            .map(|(a, b)| ((a - b).abs() / b.max(1.0)) as f64)
            .sum::<f64>()
            / ref_detail.len() as f64;
        assert!(
            rel <= 0.05,
            "{name}: detail mean relative diff {rel:.4} > 0.05"
        );
    }
}
