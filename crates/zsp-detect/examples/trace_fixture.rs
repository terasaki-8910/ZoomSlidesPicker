//! Prints the detector's events on a committed fixture (debugging aid).
//! `cargo run -p zsp-detect --example trace_fixture -- F2`
use std::path::Path;
use zsp_detect::{Config, Detail, Detector, Event, Grid};

fn main() {
    let id = std::env::args().nth(1).expect("fixture id");
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures")
        .join(&id);
    let meta: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("times.json")).unwrap()).unwrap();
    let gw = meta["grid_w"].as_u64().unwrap() as usize;
    let gh = meta["grid_h"].as_u64().unwrap() as usize;
    let scale = meta["detail_scale"].as_f64().unwrap() as f32;
    let times: Vec<f64> = meta["times"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_f64().unwrap())
        .collect();
    let g = image::open(dir.join("grid.png"))
        .unwrap()
        .into_luma8()
        .into_raw();
    let d = image::open(dir.join("detail.png"))
        .unwrap()
        .into_luma16()
        .into_raw();
    let cells = gw * gh;
    let mut det = Detector::new(Config::default());
    for (i, &t) in times.iter().enumerate() {
        let grid = Grid {
            width: gw,
            height: gh,
            data: g[i * cells..(i + 1) * cells].to_vec(),
        };
        let detail = Detail {
            width: gw,
            height: gh,
            data: d[i * cells..(i + 1) * cells]
                .iter()
                .map(|&v| v as f32 / scale)
                .collect(),
        };
        for ev in det.step(t, &grid, &detail) {
            match ev {
                Event::Hold { .. } => println!("{t:7.2} hold"),
                other => println!("{t:7.2} {other:?}"),
            }
        }
    }
}
