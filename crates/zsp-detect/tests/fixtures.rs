//! ACCEPTANCE A1-A10: the detector on the committed fixtures F1-F6.

mod common;

use common::{expected, load_strip, run_strip};

fn check_fixture(id: &str) {
    let exp = expected(id);
    let run = run_strip(&load_strip(id));
    let got: Vec<f64> = run.saves.iter().map(|s| s.t_content).collect();

    assert_eq!(
        run.saves.len(),
        exp.saves.len(),
        "{id}: number of saves, got content times {got:?}"
    );
    for (label, from, to) in &exp.saves {
        let hits = run
            .saves
            .iter()
            .filter(|s| *from <= s.t_content && s.t_content <= *to)
            .count();
        assert_eq!(
            hits, 1,
            "{id}: one save for {label} in [{from}, {to}], got content times {got:?}"
        );
    }
    for (label, from, to) in &exp.quiet {
        let inside: Vec<f64> = got
            .iter()
            .copied()
            .filter(|t| *from <= *t && *t < *to)
            .collect();
        assert!(
            inside.is_empty(),
            "{id}: no save during {label} [{from}, {to}), got {inside:?}"
        );
    }
    assert_eq!(
        run.revisits.len(),
        exp.revisits,
        "{id}: revisits, got {:?}",
        run.revisits
    );
    assert_eq!(
        run.blanks.len(),
        exp.blanks,
        "{id}: single-colour reports, got {:?}",
        run.blanks
    );
    if let Some(sharp_from) = &exp.sharp_from {
        for (s, from) in run.saves.iter().zip(sharp_from) {
            assert!(
                s.held_t >= *from,
                "{id}: save at content {} must use a frame from >= {from}, got held {}",
                s.t_content,
                s.held_t
            );
        }
    }
    for (i, s) in run.saves.iter().enumerate() {
        assert_eq!(s.index, i + 1, "{id}: indices are 1-based and dense");
    }
}

/// A1-A3: lecture at normal speed (7 saves, none while the video plays, one revisit).
#[test]
fn a1_a3_f1_lecture() {
    check_fixture("F1");
}

/// A4: lecture at 2x.
#[test]
fn a4_f2_lecture_2x() {
    check_fixture("F2");
}

/// A5: slides that arrive blurred and sharpen later; each save uses a sharp frame.
#[test]
fn a5_f3_sharpening() {
    check_fixture("F3");
}

/// A6: black and grey screens are not saved and are reported once.
#[test]
fn a6_f4_single_colour() {
    check_fixture("F4");
}

/// A7: cursor, laser pointer, and a crossfade do not cause saves.
#[test]
fn a7_f5_pointer_and_crossfade() {
    check_fixture("F5");
}

/// A8: a build on a dark background merges into one save.
#[test]
fn a8_f6_dark_build() {
    check_fixture("F6");
}

/// A9: no single-colour frame is ever saved (grid std at the held sample >= 2.0).
#[test]
fn a9_no_single_colour_frame_saved() {
    for id in ["F1", "F2", "F3", "F4", "F5", "F6"] {
        let strip = load_strip(id);
        let run = run_strip(&strip);
        for s in &run.saves {
            let i = strip
                .times
                .iter()
                .position(|t| (t - s.held_t).abs() < 1e-6)
                .expect("held_t is a sample time");
            let g = &strip.grids[i].data;
            let mean = g.iter().map(|&v| v as f64).sum::<f64>() / g.len() as f64;
            let var = g.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / g.len() as f64;
            assert!(
                var.sqrt() >= 2.0,
                "{id}: save at {} is a single-colour frame",
                s.held_t
            );
        }
    }
}

/// A10: the same input gives the same saves.
#[test]
fn a10_deterministic() {
    let strip = load_strip("F1");
    let a = run_strip(&strip);
    let b = run_strip(&strip);
    assert_eq!(a.saves, b.saves);
    assert_eq!(a.revisits, b.revisits);
}
