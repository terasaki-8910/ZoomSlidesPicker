//! Runs the detector over a recorded frame sequence and prints what it would save.
//!
//! For checking the detector against real screens (not committed: recordings contain
//! lecture content). Input directory layout: `frames/NNNN.png` (whole window) and
//! `index.tsv` (`NNNN<TAB>t_seconds` per line), as written by a 0.5 s window recorder.
//!
//! ```sh
//! cargo run --release -p zsp-detect --example replay -- <dir> <x> <y> <w> <h>
//! ```
//! `x y w h` is the region as fractions of the window (session.json `region`).

use std::fs;
use std::path::Path;

use zsp_detect::{Config, Detector, Event, analyze};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 {
        eprintln!("usage: replay <dir> <x> <y> <w> <h>");
        std::process::exit(2);
    }
    let dir = Path::new(&args[1]);
    let region: Vec<f64> = args[2..6].iter().map(|s| s.parse().unwrap()).collect();

    let index = fs::read_to_string(dir.join("index.tsv")).expect("index.tsv");
    let mut detector = Detector::new(Config::default());
    let (mut saves, mut revisits, mut last_t) = (0usize, 0usize, 0.0);

    for line in index.lines() {
        let mut parts = line.split('\t');
        let (Some(name), Some(t)) = (parts.next(), parts.next()) else {
            continue;
        };
        let t: f64 = t.parse().unwrap();
        let Ok(img) = image::open(dir.join("frames").join(format!("{name}.png"))) else {
            continue; // a frame the recorder failed to write
        };
        let img = img.into_rgba8();
        let (w, h) = img.dimensions();
        let px = |frac: f64, len: u32| ((frac * len as f64).round() as u32).min(len);
        let (x0, y0) = (px(region[0], w), px(region[1], h));
        let (x1, y1) = (px(region[0] + region[2], w), px(region[1] + region[3], h));
        let crop = image::imageops::crop_imm(&img, x0, y0, x1 - x0, y1 - y0).to_image();
        let (cw, ch) = crop.dimensions();

        let (grid, detail) = analyze(crop.as_raw(), cw, ch);
        for ev in detector.step(t, &grid, &detail) {
            match ev {
                Event::Save {
                    index, t_content, ..
                } => {
                    saves += 1;
                    println!("{t:8.2}  save #{index}  content {t_content:.2}");
                }
                Event::Revisit { t_content, matches } => {
                    revisits += 1;
                    println!("{t:8.2}  revisit of #{matches}  content {t_content:.2}");
                }
                Event::Hold { .. } if std::env::var_os("REPLAY_HOLDS").is_some() => {
                    println!("{t:8.2}  hold");
                }
                Event::Blank { .. } | Event::Hold { .. } => {}
            }
        }
        last_t = t;
    }
    for ev in detector.finish(last_t) {
        if let Event::Save { index, .. } = ev {
            saves += 1;
            println!("{last_t:8.2}  save #{index}  (end)");
        }
    }
    println!("saves {saves}, revisits {revisits}, frames to {last_t:.1} s");
}
