//! Shared helpers: load the committed fixtures and run the detector over them.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

use serde_json::Value;
use zsp_detect::{Config, Detail, Detector, Event, Grid, SaveReason};

pub fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

pub fn read_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap()
}

/// A fixture's samples: one grid and one detail per time.
pub struct Strip {
    pub times: Vec<f64>,
    pub grids: Vec<Grid>,
    pub details: Vec<Detail>,
}

pub fn load_strip(id: &str) -> Strip {
    let dir = fixtures_dir().join(id);
    let meta = read_json(&dir.join("times.json"));
    let gw = meta["grid_w"].as_u64().unwrap() as usize;
    let gh = meta["grid_h"].as_u64().unwrap() as usize;
    let scale = meta["detail_scale"].as_f64().unwrap() as f32;
    let times: Vec<f64> = meta["times"].as_array().unwrap().iter().map(|v| v.as_f64().unwrap()).collect();

    let grid_img = image::open(dir.join("grid.png")).unwrap().into_luma8();
    let detail_img = image::open(dir.join("detail.png")).unwrap().into_luma16();
    assert_eq!(grid_img.width() as usize, gw);
    assert_eq!(grid_img.height() as usize, gh * times.len());

    let gpix = grid_img.into_raw();
    let dpix = detail_img.into_raw();
    let cells = gw * gh;
    let mut grids = Vec::with_capacity(times.len());
    let mut details = Vec::with_capacity(times.len());
    for i in 0..times.len() {
        grids.push(Grid { width: gw, height: gh, data: gpix[i * cells..(i + 1) * cells].to_vec() });
        details.push(Detail {
            width: gw,
            height: gh,
            data: dpix[i * cells..(i + 1) * cells].iter().map(|&v| v as f32 / scale).collect(),
        });
    }
    Strip { times, grids, details }
}

pub struct Expected {
    pub saves: Vec<(String, f64, f64)>,
    pub quiet: Vec<(String, f64, f64)>,
    pub revisits: usize,
    pub blanks: usize,
    pub sharp_from: Option<Vec<f64>>,
}

pub fn expected(id: &str) -> Expected {
    let all = read_json(&fixtures_dir().join("expected.json"));
    let e = &all[id];
    let windows = |key: &str| -> Vec<(String, f64, f64)> {
        e[key]
            .as_array()
            .unwrap()
            .iter()
            .map(|w| (w["label"].as_str().unwrap().to_string(), w["from"].as_f64().unwrap(), w["to"].as_f64().unwrap()))
            .collect()
    };
    Expected {
        saves: windows("saves"),
        quiet: windows("quiet"),
        revisits: e["revisits"].as_u64().unwrap() as usize,
        blanks: e["blanks"].as_u64().unwrap() as usize,
        sharp_from: e.get("sharp_from").map(|v| v.as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect()),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SaveRec {
    pub index: usize,
    pub t_content: f64,
    pub t_save: f64,
    pub held_t: f64,
    pub reason: SaveReason,
}

#[derive(Debug, Default)]
pub struct Run {
    pub saves: Vec<SaveRec>,
    pub revisits: Vec<f64>,
    pub blanks: Vec<f64>,
}

/// Apply events the way the session engine will, checking the Hold/Save protocol:
/// every non-manual Save must write the frame of the most recent Hold.
pub fn collect(events: Vec<Event>, held: &mut Option<f64>, run: &mut Run) {
    for ev in events {
        match ev {
            Event::Hold { t } => *held = Some(t),
            Event::Save { index, t_content, t_save, held_t, reason } => {
                if reason != SaveReason::Manual {
                    assert_eq!(Some(held_t), *held, "Save at {t_save} must write the last held frame");
                }
                run.saves.push(SaveRec { index, t_content, t_save, held_t, reason });
            }
            Event::Revisit { t_content, .. } => run.revisits.push(t_content),
            Event::Blank { t } => run.blanks.push(t),
        }
    }
}

pub fn run_strip(strip: &Strip) -> Run {
    let mut det = Detector::new(Config::default());
    let mut run = Run::default();
    let mut held = None;
    for i in 0..strip.times.len() {
        let events = det.step(strip.times[i], &strip.grids[i], &strip.details[i]);
        collect(events, &mut held, &mut run);
    }
    let events = det.finish(*strip.times.last().unwrap());
    collect(events, &mut held, &mut run);
    run
}
