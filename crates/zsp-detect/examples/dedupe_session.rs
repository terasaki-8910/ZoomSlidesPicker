//! Lists which captures of an existing session the current detector would keep (dry run,
//! nothing is moved or deleted). Each capture is shown to the detector for 3 s, in order,
//! as if it were the screen: duplicates that differ only in a live picture fold into one.
//!
//! ```sh
//! cargo run --release -p zsp-detect --example dedupe_session -- <session folder>
//! ```
//! Prints one line per capture (`keep` or `drop`), then the counts.

use std::fs;
use std::path::Path;

use zsp_detect::{Config, Detector, Event, analyze};

const SHOW_S: f64 = 3.0;
const STEP_S: f64 = 0.5;

fn main() {
    let dir = std::env::args()
        .nth(1)
        .expect("usage: dedupe_session <session folder>");
    let dir = Path::new(&dir);
    let session: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dir.join("session.json")).expect("session.json"))
            .expect("session.json parses");
    let files: Vec<String> = session["captures"]
        .as_array()
        .expect("captures")
        .iter()
        .map(|c| c["file"].as_str().unwrap().to_string())
        .collect();

    let mut detector = Detector::new(Config::default());
    let mut keep = vec![false; files.len()];
    let shown_at = |held_t: f64| ((held_t / SHOW_S).floor() as usize).min(files.len() - 1);
    let mut t = 0.0;
    let mut last = None;
    for file in &files {
        let img = image::open(dir.join(file)).expect("capture").into_rgba8();
        let (w, h) = img.dimensions();
        let (grid, detail) = analyze(img.as_raw(), w, h);
        let mut k = 0.0;
        while k < SHOW_S - 1e-9 {
            for ev in detector.step(t, &grid, &detail) {
                if let Event::Save { held_t, .. } = ev {
                    keep[shown_at(held_t)] = true;
                }
            }
            t += STEP_S;
            k += STEP_S;
        }
        last = Some((grid, detail));
    }
    // Let the last state settle, then end.
    if let Some((grid, detail)) = last {
        for _ in 0..(10.0 / STEP_S) as usize {
            for ev in detector.step(t, &grid, &detail) {
                if let Event::Save { held_t, .. } = ev {
                    keep[shown_at(held_t)] = true;
                }
            }
            t += STEP_S;
        }
    }
    for ev in detector.finish(t) {
        if let Event::Save { held_t, .. } = ev {
            keep[shown_at(held_t)] = true;
        }
    }

    for (file, k) in files.iter().zip(&keep) {
        println!("{}\t{file}", if *k { "keep" } else { "drop" });
    }
    let kept = keep.iter().filter(|&&k| k).count();
    println!(
        "keep {kept}, drop {}, of {}",
        files.len() - kept,
        files.len()
    );
}
