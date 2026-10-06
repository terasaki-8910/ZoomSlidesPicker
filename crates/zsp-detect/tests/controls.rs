//! ACCEPTANCE A12-A14: manual save, undo, pause and resume, on synthetic slides.

mod common;

use common::{Run, collect};
use zsp_detect::{Config, Detail, Detector, Event, Grid, SaveReason};

const W: usize = 160;
const H: usize = 90;

/// A white slide with dark blocks (x, y, w, h in cells). Detail is uniform so sharpness
/// never decides anything here.
fn slide(blocks: &[(usize, usize, usize, usize)]) -> (Grid, Detail) {
    let mut data = vec![250u8; W * H];
    for &(x, y, w, h) in blocks {
        for yy in y..y + h {
            for xx in x..x + w {
                data[yy * W + xx] = 20;
            }
        }
    }
    (Grid { width: W, height: H, data }, Detail { width: W, height: H, data: vec![5.0; W * H] })
}

fn slide_a() -> (Grid, Detail) {
    slide(&[(10, 5, 60, 8), (12, 25, 40, 4), (12, 35, 50, 4)])
}

fn slide_b() -> (Grid, Detail) {
    slide(&[(10, 5, 70, 8), (90, 30, 50, 30)])
}

/// Feed `frame` every 0.5 s for t in [from, to).
fn feed(det: &mut Detector, frame: &(Grid, Detail), from: f64, to: f64, held: &mut Option<f64>, run: &mut Run) {
    let mut t = from;
    while t < to - 1e-9 {
        collect(det.step(t, &frame.0, &frame.1), held, run);
        t += 0.5;
    }
}

/// A12: a manual save writes the current frame, and the same state is not saved again.
#[test]
fn a12_save_now() {
    let mut det = Detector::new(Config::default());
    let (mut held, mut run) = (None, Run::default());
    let a = slide_a();
    feed(&mut det, &a, 0.0, 3.0, &mut held, &mut run);
    let ev = det.save_now(3.0, &a.0);
    match ev {
        Event::Save { index, held_t, reason, .. } => {
            assert_eq!(index, 1);
            assert_eq!(held_t, 3.0);
            assert_eq!(reason, SaveReason::Manual);
        }
        other => panic!("expected a manual Save, got {other:?}"),
    }
    assert_eq!(det.saved_count(), 1);
    feed(&mut det, &a, 3.0, 20.0, &mut held, &mut run);
    collect(det.finish(20.0), &mut held, &mut run);
    assert!(run.saves.is_empty(), "the manually saved state must not be saved again: {:?}", run.saves);

    // A different slide afterwards is still saved, as index 2.
    let mut det = Detector::new(Config::default());
    let (mut held, mut run) = (None, Run::default());
    feed(&mut det, &a, 0.0, 3.0, &mut held, &mut run);
    let _ = det.save_now(3.0, &a.0);
    feed(&mut det, &slide_b(), 3.0, 15.0, &mut held, &mut run);
    collect(det.finish(15.0), &mut held, &mut run);
    assert_eq!(run.saves.len(), 1, "{:?}", run.saves);
    assert_eq!(run.saves[0].index, 2);
}

/// A13: an undone state is never saved again in the session; the next save reuses the index.
#[test]
fn a13_undo() {
    let mut det = Detector::new(Config::default());
    let (mut held, mut run) = (None, Run::default());
    let a = slide_a();
    feed(&mut det, &a, 0.0, 8.0, &mut held, &mut run);
    assert_eq!(run.saves.len(), 1, "A is saved once it settles: {:?}", run.saves);
    assert_eq!(det.undo_last(), Some(1));
    assert_eq!(det.saved_count(), 0);
    assert_eq!(det.undo_last(), None);

    feed(&mut det, &a, 8.0, 20.0, &mut held, &mut run);
    assert_eq!(run.saves.len(), 1, "A stays undone while on screen: {:?}", run.saves);

    feed(&mut det, &slide_b(), 20.0, 30.0, &mut held, &mut run);
    assert_eq!(run.saves.len(), 2, "B is saved: {:?}", run.saves);
    assert_eq!(run.saves[1].index, 1, "indices stay dense after an undo");

    feed(&mut det, &a, 30.0, 45.0, &mut held, &mut run);
    collect(det.finish(45.0), &mut held, &mut run);
    assert_eq!(run.saves.len(), 2, "A is not saved again later in the session: {:?}", run.saves);
    assert!(run.revisits.is_empty(), "an undone state is not a revisit: {:?}", run.revisits);
}

/// A14: pausing drops the pending state; after resuming, states are compared afresh.
#[test]
fn a14_pause_and_resume() {
    // A is pending (settled at 1.0, not yet quiet for 5 s) when paused at 3.0.
    let mut det = Detector::new(Config::default());
    let (mut held, mut run) = (None, Run::default());
    feed(&mut det, &slide_a(), 0.0, 3.0, &mut held, &mut run);
    assert!(run.saves.is_empty());
    det.pause();
    det.resume(10.0);
    feed(&mut det, &slide_b(), 10.0, 20.0, &mut held, &mut run);
    collect(det.finish(20.0), &mut held, &mut run);
    assert_eq!(run.saves.len(), 1, "only B is saved: {:?}", run.saves);
    assert!(run.saves[0].t_content >= 10.0);

    // If A is still on screen after resuming, it is detected again and saved once.
    let mut det = Detector::new(Config::default());
    let (mut held, mut run) = (None, Run::default());
    feed(&mut det, &slide_a(), 0.0, 3.0, &mut held, &mut run);
    det.pause();
    det.resume(10.0);
    feed(&mut det, &slide_a(), 10.0, 20.0, &mut held, &mut run);
    collect(det.finish(20.0), &mut held, &mut run);
    assert_eq!(run.saves.len(), 1, "{:?}", run.saves);
    assert!(run.saves[0].t_content >= 11.0, "settles again after resume: {:?}", run.saves);
}
