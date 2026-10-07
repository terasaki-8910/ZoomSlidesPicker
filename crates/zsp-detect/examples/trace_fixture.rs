//! Prints the detector's events on a committed fixture (debugging aid).
//! `cargo run -p zsp-detect --example trace_fixture -- F2`
//!
//! The argument may also be a directory in the same layout (`grid.png`, `detail.png`,
//! `times.json`), such as a real recording converted by samples/verify/grids_to_fixture.py.
use std::path::Path;
use zsp_detect::{Config, Detail, Detector, Event, Grid};

fn main() {
    let id = std::env::args().nth(1).expect("fixture id or directory");
    let dir = if Path::new(&id).is_dir() {
        Path::new(&id).to_path_buf()
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(&id)
    };
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
    let mut det = Detector::new(config_from_env());
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
        let evs = det.step(t, &grid, &detail);
        if let Ok(spec) = std::env::var("TRACE_BOX") {
            // x0,y0,x1,y1 (inclusive): live cells in that box, and the oldest live age there
            let b: Vec<usize> = spec.split(',').map(|v| v.parse().unwrap()).collect();
            let cells = det.debug_cells(t);
            let (mut live, mut total, mut since) = (0, 0, f64::INFINITY);
            for y in b[1]..=b[3] {
                for x in b[0]..=b[2] {
                    let (l, _, s) = cells[y * gw + x];
                    total += 1;
                    if l {
                        live += 1;
                        since = since.min(s);
                    }
                }
            }
            println!("{t:7.2} box live {live}/{total} since {since:.1}");
        }
        for ev in evs {
            match ev {
                Event::Hold { .. } => println!("{t:7.2} hold"),
                other => println!("{t:7.2} {other:?}"),
            }
        }
    }
}

/// `Config::default()`, with overrides from `ZSP_CFG` (`name=value,...`; numeric fields).
fn config_from_env() -> Config {
    let mut cfg = Config::default();
    let Ok(spec) = std::env::var("ZSP_CFG") else {
        return cfg;
    };
    for item in spec.split(',').filter(|s| !s.is_empty()) {
        let (name, value) = item.split_once('=').expect("name=value");
        let v: f64 = value.parse().expect("number");
        match name {
            "settle_s" => cfg.settle_s = v,
            "final_quiet_s" => cfg.final_quiet_s = v,
            "transient_s" => cfg.transient_s = v,
            "live_window_s" => cfg.live_window_s = v,
            "motion_quiet_s" => cfg.motion_quiet_s = v,
            "event_gap_s" => cfg.event_gap_s = v,
            "live_events" => cfg.live_events = v as usize,
            "live_events_within_s" => cfg.live_events_within_s = v,
            "live_event_share" => cfg.live_event_share = v,
            "live_tile_share" => cfg.live_tile_share = v,
            "live_min_cells" => cfg.live_min_cells = v as usize,
            "slide_change_share" => cfg.slide_change_share = v,
            "live_settled_s" => cfg.live_settled_s = v,
            "live_sustain_s" => cfg.live_sustain_s = v,
            "live_sustain_events" => cfg.live_sustain_events = v as usize,
            "live_strong_cells" => cfg.live_strong_cells = v as usize,
            "live_run_gap_s" => cfg.live_run_gap_s = v,
            "motion_window" => cfg.motion_window = v as usize,
            other => panic!("unknown config field {other}"),
        }
    }
    cfg
}
