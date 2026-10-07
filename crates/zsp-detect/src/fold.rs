//! Folding the duplicates out of a finished session (R3-3, and the history window's
//! "merge duplicate slides"): each capture is shown to a fresh detector for a few seconds,
//! in order, as if it were the screen. Captures that differ only in a live picture fold
//! into one, a build keeps only its last step (R3-2), and a return to an earlier slide is
//! a revisit.

use std::collections::BTreeMap;

use crate::analyze::{Detail, Grid};
use crate::detector::{Config, Detector, Event};

/// How long each capture is shown, and the sampling interval.
const SHOW_S: f64 = 3.0;
const STEP_S: f64 = 0.5;

/// For each capture (in session order), whether it is kept. Pure: the caller decodes the
/// images and passes their analysis inputs (`analyze`).
pub fn fold_captures<I>(captures: I) -> Vec<bool>
where
    I: IntoIterator<Item = (Grid, Detail)>,
{
    let mut detector = Detector::new(Config::default());
    // Detector capture index -> position of the capture that is its current version.
    let mut kept: BTreeMap<usize, usize> = BTreeMap::new();
    let apply = |events: Vec<Event>, kept: &mut BTreeMap<usize, usize>| {
        for ev in events {
            match ev {
                Event::Save { index, held_t, .. } | Event::Replace { index, held_t, .. } => {
                    kept.insert(index, (held_t / SHOW_S).floor() as usize);
                }
                Event::Hold { .. } | Event::Revisit { .. } | Event::Blank { .. } => {}
            }
        }
    };
    let (mut t, mut n, mut last) = (0.0, 0usize, None);
    for (grid, detail) in captures {
        let mut k = 0.0;
        while k < SHOW_S - 1e-9 {
            apply(detector.step(t, &grid, &detail), &mut kept);
            t += STEP_S;
            k += STEP_S;
        }
        n += 1;
        last = Some((grid, detail));
    }
    // Let the last capture settle, then end.
    if let Some((grid, detail)) = last {
        for _ in 0..(10.0 / STEP_S) as usize {
            apply(detector.step(t, &grid, &detail), &mut kept);
            t += STEP_S;
        }
    }
    apply(detector.finish(t), &mut kept);
    let mut keep = vec![false; n];
    for pos in kept.into_values() {
        keep[pos.min(n.saturating_sub(1))] = true;
    }
    keep
}
