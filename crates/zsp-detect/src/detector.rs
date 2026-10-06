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

use std::collections::VecDeque;

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

/// Share of cells in `keep` where `a` and `b` differ by more than `cell_thresh` (0.0 when
/// no cell is kept). Reference: `changed_frac`.
fn changed_frac(a: &[u8], b: &[u8], keep: &[bool], cell_thresh: i32) -> f64 {
    let mut kept = 0usize;
    let mut changed = 0usize;
    for ((&av, &bv), &k) in a.iter().zip(b).zip(keep) {
        if k {
            kept += 1;
            if (av as i32 - bv as i32).abs() > cell_thresh {
                changed += 1;
            }
        }
    }
    if kept == 0 {
        0.0
    } else {
        changed as f64 / kept as f64
    }
}

/// Median of `values` (mean of the two middle values for an even count), sorting in place.
fn median_f64(values: &mut [f64]) -> f64 {
    values.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap());
    let n = values.len();
    if n % 2 == 1 {
        values[n / 2]
    } else {
        (values[n / 2 - 1] + values[n / 2]) / 2.0
    }
}

/// Share of kept cells that changed and moved toward `old`'s background level (its median,
/// truncated to an integer). 0.0 when no cell is kept. Reference: `removed_frac`.
fn removed_frac(old: &[u8], new: &[u8], keep: &[bool], cell_thresh: i32) -> f64 {
    let kept: Vec<(i32, i32)> = old
        .iter()
        .zip(new)
        .zip(keep)
        .filter(|&(_, &k)| k)
        .map(|((&o, &n), _)| (o as i32, n as i32))
        .collect();
    if kept.is_empty() {
        return 0.0;
    }
    let mut os: Vec<f64> = kept.iter().map(|&(o, _)| o as f64).collect();
    let bg = median_f64(&mut os) as i32;
    let changed_toward = kept
        .iter()
        .filter(|&&(o, n)| (n - o).abs() > cell_thresh && (n - bg).abs() < (o - bg).abs())
        .count();
    changed_toward as f64 / kept.len() as f64
}

/// `removed_frac` on 2x2 blocks of cells (block means, float background level, no
/// truncation): blur halos average out at this scale, removed text does not. Reference:
/// `coarse_removed_frac`.
fn coarse_removed_frac(old: &Grid, new: &Grid, keep: &[bool], cell_thresh: i32) -> f64 {
    let (w, h) = (old.width, old.height);
    let (w2, h2) = (w / 2, h / 2);
    let mut blocks: Vec<(f64, f64, bool)> = Vec::with_capacity(w2 * h2);
    for by in 0..h2 {
        for bx in 0..w2 {
            let mut o_sum = 0.0;
            let mut n_sum = 0.0;
            let mut kept = true;
            for dy in 0..2 {
                for dx in 0..2 {
                    let idx = (by * 2 + dy) * w + (bx * 2 + dx);
                    o_sum += old.data[idx] as f64;
                    n_sum += new.data[idx] as f64;
                    kept &= keep[idx];
                }
            }
            blocks.push((o_sum / 4.0, n_sum / 4.0, kept));
        }
    }
    let mut kept_o: Vec<f64> = blocks.iter().filter(|b| b.2).map(|b| b.0).collect();
    if kept_o.is_empty() {
        return 0.0;
    }
    let bg = median_f64(&mut kept_o);
    let thresh = cell_thresh as f64;
    let (mut changed_toward, mut total) = (0usize, 0usize);
    for &(o, n, k) in &blocks {
        if k {
            total += 1;
            if (n - o).abs() > thresh && (n - bg).abs() < (o - bg).abs() {
                changed_toward += 1;
            }
        }
    }
    changed_toward as f64 / total as f64
}

/// Grow `mask` ( `width` x `height`, row-major) by OR-ing each `(2r+1)x(2r+1)` neighbourhood;
/// cells outside the grid count as false. Reference: `dilate`.
fn dilate(mask: &[bool], width: usize, height: usize, r: usize) -> Vec<bool> {
    if r == 0 || width == 0 || height == 0 {
        return mask.to_vec();
    }
    let mut out = vec![false; width * height];
    for y in 0..height {
        let y0 = y.saturating_sub(r);
        let y1 = (y + r).min(height - 1);
        for x in 0..width {
            let x0 = x.saturating_sub(r);
            let x1 = (x + r).min(width - 1);
            let mut hit = false;
            'search: for yy in y0..=y1 {
                for xx in x0..=x1 {
                    if mask[yy * width + xx] {
                        hit = true;
                        break 'search;
                    }
                }
            }
            out[y * width + x] = hit;
        }
    }
    out
}

/// Fraction of the (up to `motion_window`-long) recent-change history active for each cell,
/// compared against `motion_ratio` (strictly greater). Divides by the deque's current
/// length, which grows up to `motion_window` at the start of a session.
fn motion_mask(recent: &VecDeque<Vec<bool>>, len: usize, motion_ratio: f64) -> Vec<bool> {
    let n = recent.len() as f64;
    let mut counts = vec![0u32; len];
    for mask in recent {
        for (c, &v) in counts.iter_mut().zip(mask.iter()) {
            if v {
                *c += 1;
            }
        }
    }
    counts
        .iter()
        .map(|&c| (c as f64 / n) > motion_ratio)
        .collect()
}

/// Population standard deviation (ddof 0) of `data`.
fn std_dev(data: &[u8]) -> f64 {
    let n = data.len() as f64;
    let mean = data.iter().map(|&v| v as f64).sum::<f64>() / n;
    let var = data.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n;
    var.sqrt()
}

/// A not-yet-committed state: its content, and the sharpest frame seen for it so far.
struct Pending {
    /// Content time: when this state was first judged settled (or re-started).
    t: f64,
    /// The comparison grid for this state (may be replaced by a sharper same-slide frame).
    grid: Grid,
    /// Time of the frame currently held for saving.
    held_t: f64,
    /// Mean detail of the held frame, for comparing against new candidates.
    held_sharp: f64,
}

/// The save decision state machine. See the module docs for the caller protocol.
pub struct Detector {
    cfg: Config,
    /// Previous sample's grid, for frame-to-frame motion detection.
    prev: Option<Grid>,
    /// Recent per-cell "changed vs previous sample" masks, newest last, capped at
    /// `cfg.motion_window`.
    recent: VecDeque<Vec<bool>>,
    /// Time of the most recent sample that differed enough from the previous one.
    last_change_t: f64,
    /// The comparison baseline: the most recently saved, revisited, or rejected-matched grid.
    last_saved: Option<Grid>,
    /// Saved states, oldest first; `history.len()` is the dense 1-based index of the next save.
    history: Vec<Grid>,
    /// States removed by `undo_last`: never saved again in this session.
    rejected: Vec<Grid>,
    /// The not-yet-committed state, if any.
    pending: Option<Pending>,
    /// When the current single-colour stretch started, if any.
    uniform_since: Option<f64>,
    /// Whether the current single-colour stretch has already been reported.
    blank_reported: bool,
}

impl Detector {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            prev: None,
            recent: VecDeque::new(),
            last_change_t: 0.0,
            last_saved: None,
            history: Vec::new(),
            rejected: Vec::new(),
            pending: None,
            uniform_since: None,
            blank_reported: false,
        }
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    /// Commit `pending`: a `Save` if it matches no saved or rejected state, a `Revisit` if
    /// it matches a saved one, or nothing if it matches a rejected one (it still becomes the
    /// new comparison baseline). Reference: `Detector._commit`.
    fn commit(
        &mut self,
        pending: Pending,
        t_save: f64,
        keep: &[bool],
        reason: SaveReason,
        events: &mut Vec<Event>,
    ) {
        let thresh = self.cfg.cell_thresh;
        let change_thresh = self.cfg.change_thresh;
        for (idx, hist) in self.history.iter().enumerate().rev() {
            if changed_frac(&pending.grid.data, &hist.data, keep, thresh) <= change_thresh {
                events.push(Event::Revisit {
                    t_content: pending.t,
                    matches: idx + 1,
                });
                self.last_saved = Some(hist.clone());
                return;
            }
        }
        for rej in self.rejected.iter().rev() {
            if changed_frac(&pending.grid.data, &rej.data, keep, thresh) <= change_thresh {
                self.last_saved = Some(rej.clone());
                return;
            }
        }
        let index = self.history.len() + 1;
        events.push(Event::Save {
            index,
            t_content: pending.t,
            t_save,
            held_t: pending.held_t,
            reason,
        });
        self.last_saved = Some(pending.grid.clone());
        self.history.push(pending.grid);
    }

    /// Start a new pending state and report it as held. Reference: `Detector._start_pending`.
    fn start_pending(&mut self, t: f64, grid: Grid, sharp: f64, events: &mut Vec<Event>) {
        self.pending = Some(Pending {
            t,
            grid,
            held_t: t,
            held_sharp: sharp,
        });
        events.push(Event::Hold { t });
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
        let cfg = self.cfg.clone();
        let mut events = Vec::new();
        let len = grid.data.len();

        let prev_data: Vec<u8> = match &self.prev {
            Some(p) => p.data.clone(),
            None => grid.data.clone(),
        };

        let diff_mask: Vec<bool> = grid
            .data
            .iter()
            .zip(&prev_data)
            .map(|(&a, &b)| (a as i32 - b as i32).abs() > cfg.cell_thresh)
            .collect();
        self.recent.push_back(diff_mask);
        if self.recent.len() > cfg.motion_window {
            self.recent.pop_front();
        }
        let moving_raw = motion_mask(&self.recent, len, cfg.motion_ratio);
        let moving = dilate(&moving_raw, grid.width, grid.height, cfg.mask_pad);
        let keep: Vec<bool> = moving.iter().map(|&m| !m).collect();

        if changed_frac(&grid.data, &prev_data, &keep, cfg.cell_thresh) > cfg.change_thresh {
            self.last_change_t = t;
        }
        let quiet_for = t - self.last_change_t;

        let uniform = std_dev(&grid.data) < cfg.uniform_std;
        if uniform {
            match self.uniform_since {
                None => {
                    self.uniform_since = Some(t);
                    self.blank_reported = false;
                }
                Some(since) if !self.blank_reported && t - since >= cfg.blank_report_s => {
                    events.push(Event::Blank { t });
                    self.blank_reported = true;
                }
                Some(_) => {}
            }
        } else {
            self.uniform_since = None;
        }

        let kept_detail: Vec<f64> = detail
            .data
            .iter()
            .zip(&keep)
            .filter(|&(_, &k)| k)
            .map(|(&d, _)| d as f64)
            .collect();
        let sharp = if kept_detail.is_empty() {
            0.0
        } else {
            kept_detail.iter().sum::<f64>() / kept_detail.len() as f64
        };

        if !uniform && quiet_for >= cfg.settle_s {
            match self.pending.take() {
                None => {
                    let differs = match &self.last_saved {
                        None => true,
                        Some(ls) => {
                            changed_frac(&grid.data, &ls.data, &keep, cfg.cell_thresh)
                                > cfg.change_thresh
                        }
                    };
                    if differs {
                        self.start_pending(t, grid.clone(), sharp, &mut events);
                    }
                }
                Some(mut p) => {
                    let differs = changed_frac(&grid.data, &p.grid.data, &keep, cfg.cell_thresh)
                        > cfg.change_thresh;
                    if differs {
                        let sharpened = sharp >= p.held_sharp * cfg.refine_ratio
                            && coarse_removed_frac(&p.grid, grid, &keep, cfg.cell_thresh)
                                < cfg.removed_thresh;
                        if sharpened {
                            p.grid = grid.clone();
                            p.held_t = t;
                            p.held_sharp = sharp;
                            events.push(Event::Hold { t });
                            self.pending = Some(p);
                        } else if removed_frac(&p.grid.data, &grid.data, &keep, cfg.cell_thresh)
                            >= cfg.removed_thresh
                        {
                            self.commit(p, t, &keep, SaveReason::Settled, &mut events);
                            let differs2 = match &self.last_saved {
                                None => true,
                                Some(ls) => {
                                    changed_frac(&grid.data, &ls.data, &keep, cfg.cell_thresh)
                                        > cfg.change_thresh
                                }
                            };
                            if differs2 {
                                self.start_pending(t, grid.clone(), sharp, &mut events);
                            }
                        } else {
                            self.start_pending(t, grid.clone(), sharp, &mut events);
                        }
                    } else {
                        if sharp > p.held_sharp * cfg.sharper_ratio {
                            p.held_t = t;
                            p.held_sharp = sharp;
                            events.push(Event::Hold { t });
                        }
                        self.pending = Some(p);
                    }
                }
            }
        }

        if self.pending.is_some() && quiet_for >= cfg.final_quiet_s {
            let p = self.pending.take().unwrap();
            self.commit(p, t, &keep, SaveReason::Settled, &mut events);
        }

        self.prev = Some(grid.clone());
        events
    }

    /// End of input: commit a pending state with reason `End` (`Detector.finish`).
    pub fn finish(&mut self, t: f64) -> Vec<Event> {
        let mut events = Vec::new();
        if let Some(p) = self.pending.take() {
            let keep = vec![true; p.grid.data.len()];
            self.commit(p, t, &keep, SaveReason::End, &mut events);
        }
        events
    }

    /// Save the current sample now (SPEC 判定 11). Returns `Save { reason: Manual,
    /// t_content: t, t_save: t, held_t: t, .. }`; the caller writes the current frame.
    /// The grid joins the saved history (later equal states are revisits, not saves) and
    /// becomes the comparison baseline. A pending state equal to `grid` is dropped.
    pub fn save_now(&mut self, t: f64, grid: &Grid) -> Event {
        let keep = vec![true; grid.data.len()];
        if let Some(p) = &self.pending
            && changed_frac(&p.grid.data, &grid.data, &keep, self.cfg.cell_thresh)
                <= self.cfg.change_thresh
        {
            self.pending = None;
        }
        let index = self.history.len() + 1;
        self.last_saved = Some(grid.clone());
        self.history.push(grid.clone());
        Event::Save {
            index,
            t_content: t,
            t_save: t,
            held_t: t,
            reason: SaveReason::Manual,
        }
    }

    /// Undo the most recent save (SPEC 判定 12): remove it from the saved history and never
    /// save an equal state again in this session. The baseline becomes the previous saved
    /// state (or none). Returns the undone index, or `None` when nothing was saved.
    pub fn undo_last(&mut self) -> Option<usize> {
        let grid = self.history.pop()?;
        let index = self.history.len() + 1;
        self.rejected.push(grid);
        self.last_saved = self.history.last().cloned();
        Some(index)
    }

    /// Pause (SPEC 判定 13): drop the pending state. The caller stops calling `step` and
    /// drops its held frame.
    pub fn pause(&mut self) {
        self.pending = None;
    }

    /// Resume at time `t`: compare afresh. No previous sample, empty motion history,
    /// no single-colour stretch in progress, and the quiet timer starts at `t`.
    pub fn resume(&mut self, t: f64) {
        self.prev = None;
        self.recent.clear();
        self.uniform_since = None;
        self.blank_reported = false;
        self.last_change_t = t;
    }

    /// Number of saves currently in the history (after undos).
    pub fn saved_count(&self) -> usize {
        self.history.len()
    }
}
