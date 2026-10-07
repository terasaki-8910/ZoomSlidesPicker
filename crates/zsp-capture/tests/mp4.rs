//! ACCEPTANCE B8: the mp4 mode on fixtures/f1_2fps.mp4 gives the F1 results
//! (the captures at the end, content times within 0.5 s of the F1 windows; replacements and
//! revisits as in F1). Skipped when ffmpeg is not on PATH.

use std::path::Path;

use zsp_capture::mp4::{Mp4Frames, ffmpeg_available};
use zsp_detect::{Config, Detector, Event, analyze};

#[test]
fn b8_mp4_mode_matches_f1() {
    if !ffmpeg_available() {
        eprintln!("skipped: ffmpeg/ffprobe not on PATH");
        return;
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures");
    let mut frames = Mp4Frames::open(&root.join("f1_2fps.mp4"), 2.0).unwrap();
    assert_eq!(frames.size(), (1280, 720));

    let mut det = Detector::new(Config::default());
    // Capture index -> content time of its current version (a Replace overwrites it).
    let mut captures = std::collections::BTreeMap::new();
    let (mut replaces, mut revisits) = (0, 0);
    let mut last_t = 0.0;
    let mut n = 0;
    for item in frames.by_ref() {
        let (t, img) = item.unwrap();
        assert!((t - n as f64 * 0.5).abs() < 1e-9, "frame {n} is at {t}");
        let (g, d) = analyze(img.as_raw(), img.width(), img.height());
        for ev in det.step(t, &g, &d) {
            match ev {
                Event::Save {
                    index, t_content, ..
                } => {
                    captures.insert(index, t_content);
                }
                Event::Replace {
                    index, t_content, ..
                } => {
                    captures.insert(index, t_content);
                    replaces += 1;
                }
                Event::Revisit { .. } => revisits += 1,
                _ => {}
            }
        }
        last_t = t;
        n += 1;
    }
    for ev in det.finish(last_t) {
        if let Event::Save {
            index, t_content, ..
        }
        | Event::Replace {
            index, t_content, ..
        } = ev
        {
            captures.insert(index, t_content);
        }
    }
    let saves: Vec<f64> = captures.into_values().collect();
    assert_eq!(n, 176, "decoded frames");

    let expected: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("expected.json")).unwrap())
            .unwrap();
    let windows = expected["F1"]["saves"].as_array().unwrap();
    assert_eq!(saves.len(), windows.len(), "saves {saves:?}");
    for w in windows {
        let (from, to) = (
            w["from"].as_f64().unwrap() - 0.5,
            w["to"].as_f64().unwrap() + 0.5,
        );
        let hits = saves.iter().filter(|t| from <= **t && **t <= to).count();
        assert_eq!(
            hits, 1,
            "one save for {} in [{from}, {to}], got {saves:?}",
            w["label"]
        );
    }
    assert_eq!(
        revisits,
        expected["F1"]["revisits"].as_u64().unwrap() as usize
    );
    assert_eq!(
        replaces,
        expected["F1"]["replaces"].as_u64().unwrap_or(0) as usize
    );
}
