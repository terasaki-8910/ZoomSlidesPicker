//! Lists which captures of an existing session the current detector would keep (dry run,
//! nothing is moved or deleted): `zsp_detect::fold_captures` over the session's PNGs.
//!
//! ```sh
//! cargo run --release -p zsp-detect --example dedupe_session -- <session folder>
//! ```
//! Prints one line per capture (`keep` or `drop`), then the counts.

use std::fs;
use std::path::Path;

use zsp_detect::{analyze, fold_captures};

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

    let keep = fold_captures(files.iter().map(|file| {
        let img = image::open(dir.join(file)).expect("capture").into_rgba8();
        let (w, h) = img.dimensions();
        analyze(img.as_raw(), w, h)
    }));

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
