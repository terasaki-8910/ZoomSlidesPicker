//! The save decision. Port of `Detector` in samples/proto/detect.py, plus manual save,
//! undo, and pause (SPEC.md > 判定 11-13), which the reference does not have.
//!
//! Protocol for the caller (the session engine):
//! - Call [`Detector::step`] once per sample, in time order, and apply the returned events
//!   in order.
//! - [`Event::Hold`]: keep the full-size frame of this sample as the frame to save for the
//!   pending state, replacing any frame held before.
//! - [`Event::Save`] with reason `Settled` or `End`: write the held frame. A `Save` is always
//!   reported before a `Hold` that replaces the held frame in the same step.
//! - [`Detector::save_now`] returns a `Save` with reason `Manual`: write the current frame.
//! - Indices are 1-based and dense: the next save gets `saved_count() + 1`, so after an undo
//!   the undone index is reused.

use crate::analyze::{Detail, Grid};

/// Detection parameters. Defaults are the values in samples/proto/detect.py `Config`.
#[derive(Clone, Debug, PartialEq)]
pub struct Config {
    /// Comparison period N (seconds). Not used by the detector itself; carried for the caller.
    pub interval_s: f64,
    /// Quiet time before a state counts as settled.
    pub settle_s: f64,
    /// Additions closer together than this merge into one save.
    pub final_quiet_s: f64,
    /// Per-cell |difference| (0-255) that counts as a changed cell (strictly greater).
    pub cell_thresh: i32,
    /// Share of compared cells changed => a real change (strictly greater).
    pub change_thresh: f64,
    /// Share of cells that moved back toward the background => a different slide.
    pub removed_thresh: f64,
    /// Number of recent comparisons kept for motion detection.
    pub motion_window: usize,
    /// A cell is moving when active in more than this share of the window.
    pub motion_ratio: f64,
    /// Grow moving regions by this many cells (square neighbourhood).
    pub mask_pad: usize,
    /// Population standard deviation of the grid below this => single-colour frame.
    pub uniform_std: f64,
    /// Report a single-colour stretch once it lasts this long.
    pub blank_report_s: f64,
    /// Replace the held frame only when this much sharper.
    pub sharper_ratio: f64,
    /// A change counts as the same slide sharpening only when this much sharper.
    pub refine_ratio: f64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_s: 0.5,
            settle_s: 1.0,
            final_quiet_s: 5.0,
            cell_thresh: 12,
            change_thresh: 0.002,
            removed_thresh: 0.001,
            motion_window: 6,
            motion_ratio: 0.5,
            mask_pad: 1,
            uniform_std: 2.0,
            blank_report_s: 10.0,
            sharper_ratio: 1.10,
            refine_ratio: 1.20,
        }
    }
}

/// Why a frame was saved.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveReason {
    /// The state settled and stayed quiet (or a different slide arrived).
    Settled,
    /// Input ended with a pending state ([`Detector::finish`]).
    End,
    /// The user asked to save now ([`Detector::save_now`]).
    Manual,
}

/// What the caller must do, in order.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Keep the full-size frame of the sample at `t` as the frame to save for the pending
    /// state, replacing any previously held frame.
    Hold { t: f64 },
    /// Save. `held_t` is the time of the sample whose frame is written (the last `Hold`
    /// for `Settled`/`End`; the current sample for `Manual`).
    Save {
        index: usize,
        t_content: f64,
        t_save: f64,
        held_t: f64,
        reason: SaveReason,
    },
    /// The settled state equals the already-saved state `matches` (its index); nothing saved.
    Revisit { t_content: f64, matches: usize },
    /// A single-colour stretch has lasted `blank_report_s`; reported once per stretch.
    Blank { t: f64 },
}

/// The save decision state machine. See the module docs for the caller protocol.
pub struct Detector {
    cfg: Config,
}

impl Detector {
    pub fn new(cfg: Config) -> Self {
        Self { cfg }
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    /// Process one sample (`Detector.step` in the reference).
    ///
    /// Additions to the reference:
    /// - Emit `Hold { t }` whenever the reference starts or replaces the pending state, or
    ///   updates `held_t` to a sharper frame.
    /// - When committing, a pending state equal to an undone state (see [`Self::undo_last`])
    ///   is dropped silently: no `Save`, no `Revisit`; it becomes the comparison baseline
    ///   (`last_saved` in the reference) so it is not proposed again until the screen changes.
    pub fn step(&mut self, t: f64, grid: &Grid, detail: &Detail) -> Vec<Event> {
        let _ = (t, grid, detail);
        todo!("port Detector.step from samples/proto/detect.py")
    }

    /// End of input: commit a pending state with reason `End` (`Detector.finish`).
    pub fn finish(&mut self, t: f64) -> Vec<Event> {
        let _ = t;
        todo!()
    }

    /// Save the current sample now (SPEC 判定 11). Returns `Save { reason: Manual,
    /// t_content: t, t_save: t, held_t: t, .. }`; the caller writes the current frame.
    /// The grid joins the saved history (later equal states are revisits, not saves) and
    /// becomes the comparison baseline. A pending state equal to `grid` is dropped.
    pub fn save_now(&mut self, t: f64, grid: &Grid) -> Event {
        let _ = (t, grid);
        todo!()
    }

    /// Undo the most recent save (SPEC 判定 12): remove it from the saved history and never
    /// save an equal state again in this session. The baseline becomes the previous saved
    /// state (or none). Returns the undone index, or `None` when nothing was saved.
    pub fn undo_last(&mut self) -> Option<usize> {
        todo!()
    }

    /// Pause (SPEC 判定 13): drop the pending state. The caller stops calling `step` and
    /// drops its held frame.
    pub fn pause(&mut self) {
        todo!()
    }

    /// Resume at time `t`: compare afresh. No previous sample, empty motion history,
    /// no single-colour stretch in progress, and the quiet timer starts at `t`.
    pub fn resume(&mut self, t: f64) {
        let _ = t;
        todo!()
    }

    /// Number of saves currently in the history (after undos).
    pub fn saved_count(&self) -> usize {
        todo!()
    }
}
