//! The save decision. Port of `Detector` in samples/proto/detect.py, plus manual save,
//! undo, and pause (SPEC.md > 判定 11-13), live pictures (判定 3), brief states (判定 5),
//! and additions told apart from sharpening (判定 6), which the reference does not have.
//!
//! Live pictures: the reference masks a cell while it changes in most of the last few
//! samples, and compares it again as soon as it stops. A speaker's camera tile in a recorded
//! lecture changes a few cells every few seconds, never often enough to be masked, so each
//! of those changes looked like a new slide. Here, once per `event_gap_s` (whatever the
//! sampling interval), the cells changed since the last look are grouped into clusters; a
//! cluster of a few cells (`live_min_cells`) in a small box (`live_tile_share`) touches
//! that box, unless much of the grid changed (`live_event_share`). A cell becomes a live
//! picture after `live_events` strong touches (`live_strong_cells`) within
//! `live_events_within_s`, or after touches of any size kept up for `live_sustain_s`; it
//! then stays out of every comparison while it keeps changing at least once per
//! `live_window_s`. A pointer is two small blobs (where it was, where it is) that stays on
//! a line for seconds, so it makes no text live. A slide change (`slide_change_share`)
//! clears the touches of cells that are not live yet, and ends the live pictures it changed
//! that are younger than `live_settled_s` (a pointer that dwelt on the old slide's text),
//! keeping the older ones (a camera tile). A build or a slide change changes a cell once,
//! and a playing video changes it every sample, so neither is one: the look after a video
//! stops is still saved (R1-5 b), once it has stayed for `motion_quiet_s` or the slide
//! changes.
//!
//! Removed and added content is judged against the background around each changed area,
//! not the whole grid's median, which lands between black and white when the region has
//! black bars beside the slide.
//!
//! Protocol for the caller (the session engine):
//! - Call [`Detector::step`] once per sample, in time order, and apply the returned events
//!   in order.
//! - [`Event::Hold`]: keep the full-size frame of this sample as the frame to save for the
//!   pending state, replacing any frame held before.
//! - [`Event::Save`] with reason `Settled` or `End`: write the held frame. A `Save` is always
//!   reported before a `Hold` that replaces the held frame in the same step.
//! - [`Event::Replace`]: write the held frame as the new version of capture `index` (same
//!   ordering rule as `Save`). The capture keeps its index; its old file goes to the trash.
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
    /// A state left within this long of settling, by going back to the state saved last,
    /// is not saved: player controls shown for a moment, a notification, a slide flicked
    /// past and back.
    pub transient_s: f64,
    /// Per-cell |difference| (0-255) that counts as a changed cell (strictly greater).
    pub cell_thresh: i32,
    /// Share of compared cells changed => a real change (strictly greater).
    pub change_thresh: f64,
    /// Share of cells that moved back toward the background => a different slide.
    pub removed_thresh: f64,
    /// Share of cells where content appeared on empty background => an addition (a build),
    /// not the same slide getting sharper.
    pub added_thresh: f64,
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
    /// A cell that moved in a sustained way within this long and changes again (with
    /// nothing else changing) is a live picture, and stays one while it changes at least
    /// this often.
    pub live_window_s: f64,
    /// A pending state that differs from the last save only where something moved within
    /// `live_window_s`, or a small box changed within this long, is saved at the final
    /// quiet only once it is this old (a camera tile may turn live meanwhile).
    pub motion_quiet_s: f64,
    /// Motion is remembered (for the wait above) only while the moving cells are at most
    /// this share of the grid: a long transition or a scroll moves everything.
    pub live_max_share: f64,
    /// Touches are looked for over spans of at least this long (whatever the sampling
    /// interval), and touches of a cell closer than this count once.
    pub event_gap_s: f64,
    /// Over each span, the changed cells are grouped into clusters (cells at most two apart
    /// join); a cluster of at least `live_min_cells` cells whose box is at most
    /// `live_tile_share` of the grid touches the cells of its box, unless more than
    /// `live_event_share` of the grid changed over the span (a slide change, a transition).
    /// A cluster of at least `live_strong_cells` is a strong touch.
    pub live_min_cells: usize,
    pub live_strong_cells: usize,
    pub live_tile_share: f64,
    pub live_event_share: f64,
    /// A cell becomes a live picture with `live_events` strong touches within
    /// `live_events_within_s` (2 or 3), or once its touches of any size have gone on for
    /// `live_sustain_s` (a run: no two touches further apart than `live_run_gap_s`) with at
    /// least `live_sustain_events` of them. A camera tile keeps changing for minutes; a
    /// pointer is two small blobs (where it was, where it is) and dwells on a line for
    /// seconds, so it makes no text live.
    pub live_events: usize,
    pub live_events_within_s: f64,
    pub live_sustain_events: usize,
    pub live_sustain_s: f64,
    pub live_run_gap_s: f64,
    /// A sample where more than this share of the grid changed (a slide change) clears the
    /// change history of cells that are not live.
    pub slide_change_share: f64,
    /// Live pictures at least this old survive a slide change that changes them.
    pub live_settled_s: f64,
    /// Grow live pictures by this many cells (their changes are sparse within the tile).
    pub live_pad: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            interval_s: 0.5,
            settle_s: 1.0,
            final_quiet_s: 5.0,
            transient_s: 5.0,
            cell_thresh: 12,
            change_thresh: 0.002,
            removed_thresh: 0.001,
            added_thresh: 0.001,
            motion_window: 6,
            motion_ratio: 0.5,
            mask_pad: 1,
            uniform_std: 2.0,
            blank_report_s: 10.0,
            sharper_ratio: 1.10,
            refine_ratio: 1.20,
            live_window_s: 120.0,
            motion_quiet_s: 15.0,
            live_max_share: 0.25,
            event_gap_s: 1.0,
            live_min_cells: 4,
            live_strong_cells: 15,
            live_tile_share: 0.06,
            live_event_share: 0.015,
            live_events: 3,
            live_events_within_s: 10.0,
            live_sustain_events: 4,
            live_sustain_s: 30.0,
            live_run_gap_s: 30.0,
            slide_change_share: 0.05,
            live_settled_s: 60.0,
            live_pad: 1,
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
    /// The settled state only adds to saved state `index` (text or lines added, nothing
    /// removed; R3-2): write the held frame as the new version of that capture, replacing
    /// its file. `held_t` as for `Save`. Its earlier versions still count as that capture
    /// for revisits.
    Replace {
        index: usize,
        t_content: f64,
        t_save: f64,
        held_t: f64,
        reason: SaveReason,
    },
    /// The settled state equals the earlier saved state `matches` (its index); nothing saved.
    /// Not reported for a state equal to the latest baseline (the screen stayed on it).
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

/// How far around a cluster of changed cells its background is looked for.
const BG_PAD: usize = 3;

/// What a change did, as shares of the kept cells that changed by more than `thresh`:
/// `removed`, moved toward the background around them (content taken away); `added`, were
/// background and are not any more (content added on an empty area). 0.0 when no cell is
/// kept.
///
/// The background is local: the median of `old` over the kept cells of each cluster's box
/// grown by `BG_PAD`. (The reference uses the median of the whole grid, which lands between
/// black and white when the region has black bars beside the slide, so the faint edges of
/// added text looked like removed text.)
struct ChangeKinds {
    removed: f64,
    added: f64,
}

fn change_kinds(
    old: &[f64],
    new: &[f64],
    keep: &[bool],
    w: usize,
    h: usize,
    thresh: f64,
) -> ChangeKinds {
    let kept = keep.iter().filter(|&&k| k).count();
    if kept == 0 {
        return ChangeKinds {
            removed: 0.0,
            added: 0.0,
        };
    }
    let changed: Vec<bool> = (0..old.len())
        .map(|i| keep[i] && (new[i] - old[i]).abs() > thresh)
        .collect();
    let (labels, boxes) = label_clusters(&changed, w, h);
    let bgs: Vec<f64> = boxes
        .iter()
        .map(|b| {
            let (x0, y0) = (b.x0.saturating_sub(BG_PAD), b.y0.saturating_sub(BG_PAD));
            let (x1, y1) = ((b.x1 + BG_PAD).min(w - 1), (b.y1 + BG_PAD).min(h - 1));
            let mut vals: Vec<f64> = (y0..=y1)
                .flat_map(|y| (x0..=x1).map(move |x| y * w + x))
                .filter(|&i| keep[i])
                .map(|i| old[i])
                .collect();
            median_f64(&mut vals)
        })
        .collect();
    let (mut removed, mut added) = (0usize, 0usize);
    for i in (0..old.len()).filter(|&i| changed[i]) {
        let bg = bgs[labels[i]];
        let (o, n) = ((old[i] - bg).abs(), (new[i] - bg).abs());
        if n < o {
            removed += 1;
        }
        if o <= thresh && n > thresh {
            added += 1;
        }
    }
    ChangeKinds {
        removed: removed as f64 / kept as f64,
        added: added as f64 / kept as f64,
    }
}

fn grid_f64(g: &Grid) -> Vec<f64> {
    g.data.iter().map(|&v| v as f64).collect()
}

/// [`change_kinds`] on cells. Reference: `removed_frac` (removed only, whole-grid
/// background).
fn cell_change_kinds(old: &Grid, new: &Grid, keep: &[bool], cell_thresh: i32) -> ChangeKinds {
    let (o, n) = (grid_f64(old), grid_f64(new));
    change_kinds(&o, &n, keep, old.width, old.height, cell_thresh as f64)
}

/// The removed share of [`change_kinds`] on 2x2 blocks of cells (block means; a block is kept when all its
/// cells are): blur halos average out at this scale, removed text does not. Reference:
/// `coarse_removed_frac`.
fn coarse_removed_frac(old: &Grid, new: &Grid, keep: &[bool], cell_thresh: i32) -> f64 {
    let (w, h) = (old.width, old.height);
    let (w2, h2) = (w / 2, h / 2);
    let mut o = Vec::with_capacity(w2 * h2);
    let mut n = Vec::with_capacity(w2 * h2);
    let mut k = Vec::with_capacity(w2 * h2);
    for by in 0..h2 {
        for bx in 0..w2 {
            let (mut o_sum, mut n_sum, mut kept) = (0.0, 0.0, true);
            for dy in 0..2 {
                for dx in 0..2 {
                    let idx = (by * 2 + dy) * w + (bx * 2 + dx);
                    o_sum += old.data[idx] as f64;
                    n_sum += new.data[idx] as f64;
                    kept &= keep[idx];
                }
            }
            o.push(o_sum / 4.0);
            n.push(n_sum / 4.0);
            k.push(kept);
        }
    }
    change_kinds(&o, &n, &k, w2, h2, cell_thresh as f64).removed
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

/// Sample times jitter by a few milliseconds; spans this much shorter still count.
const TIME_EPS: f64 = 0.05;

/// Bounding box and size of one cluster of changed cells.
struct Cluster {
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    cells: usize,
}

/// Clusters of the set cells of `mask` (`width` x `height`): cells at most two apart (in
/// both directions) belong to the same cluster.
fn clusters(mask: &[bool], width: usize, height: usize) -> Vec<Cluster> {
    label_clusters(mask, width, height).1
}

/// [`clusters`], with each set cell's cluster index (`usize::MAX` for unset cells).
fn label_clusters(mask: &[bool], width: usize, height: usize) -> (Vec<usize>, Vec<Cluster>) {
    let mut label = vec![usize::MAX; mask.len()];
    let mut out = Vec::new();
    let mut stack = Vec::new();
    for start in 0..mask.len() {
        if !mask[start] || label[start] != usize::MAX {
            continue;
        }
        let id = out.len();
        let (sx, sy) = (start % width, start / width);
        let mut b = Cluster {
            x0: sx,
            y0: sy,
            x1: sx,
            y1: sy,
            cells: 0,
        };
        label[start] = id;
        stack.push(start);
        while let Some(i) = stack.pop() {
            let (x, y) = (i % width, i / width);
            b.cells += 1;
            (b.x0, b.y0, b.x1, b.y1) = (b.x0.min(x), b.y0.min(y), b.x1.max(x), b.y1.max(y));
            for ny in y.saturating_sub(2)..=(y + 2).min(height - 1) {
                for nx in x.saturating_sub(2)..=(x + 2).min(width - 1) {
                    let n = ny * width + nx;
                    if mask[n] && label[n] == usize::MAX {
                        label[n] = id;
                        stack.push(n);
                    }
                }
            }
        }
        out.push(b);
    }
    (label, out)
}

/// The cells a step compares, after motion and live pictures are taken out.
struct Masks {
    /// Not moving and not live: compared.
    keep: Vec<bool>,
    /// `keep`, minus cells one touch away from turning live (touched twice within
    /// `live_events_within_s`): used to tell a new state from the pending one and from the
    /// saved ones, so a camera tile that is not live yet does not save the slide again.
    keep_firm: Vec<bool>,
    /// `keep`, minus cells that moved within `live_window_s` or were touched within
    /// `motion_quiet_s` (maybe a live picture that is not one yet).
    keep_still: Vec<bool>,
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
    /// Earlier versions of replaced captures, with the capture's index: a return to one is
    /// a revisit of that capture.
    aliases: Vec<(usize, Grid)>,
    /// The saved capture the baseline shows (1-based), or `None` when it is a rejected state.
    last_saved_index: Option<usize>,
    /// States removed by `undo_last`: never saved again in this session.
    rejected: Vec<Grid>,
    /// The not-yet-committed state, if any.
    pending: Option<Pending>,
    /// When the current single-colour stretch started, if any.
    uniform_since: Option<f64>,
    /// Whether the current single-colour stretch has already been reported.
    blank_reported: bool,
    /// Per cell: the last time it was in the motion mask (`NEG_INFINITY` if never).
    last_moving_t: Vec<f64>,
    /// Per cell: a live picture until this time (`NEG_INFINITY` if not).
    live_until: Vec<f64>,
    /// Per cell: since when it has been a live picture (meaningful while it is one).
    live_since: Vec<f64>,
    /// Per cell: the last time a sample touched it.
    last_touch_t: Vec<f64>,
    /// Per cell: when its latest two counted touches happened (latest first).
    event_t: Vec<[f64; 2]>,
    /// Per cell: when its latest two strong touches happened (latest first).
    strong_t: Vec<[f64; 2]>,
    /// Per cell: when the current run of touches began, and how many it has had.
    run_start: Vec<f64>,
    run_count: Vec<u32>,
    /// The grid touches were last looked for on, and when.
    touch_ref: Option<(f64, Vec<u8>)>,
    /// Cells compared in the latest step (not moving, not live); `finish` uses it.
    last_keep: Option<Vec<bool>>,
    /// `keep_still` of the latest step; `finish` uses it.
    last_keep_still: Option<Vec<bool>>,
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
            aliases: Vec::new(),
            last_saved_index: None,
            rejected: Vec::new(),
            pending: None,
            uniform_since: None,
            blank_reported: false,
            last_moving_t: Vec::new(),
            live_until: Vec::new(),
            live_since: Vec::new(),
            last_touch_t: Vec::new(),
            event_t: Vec::new(),
            strong_t: Vec::new(),
            run_start: Vec::new(),
            run_count: Vec::new(),
            touch_ref: None,
            last_keep: None,
            last_keep_still: None,
        }
    }

    pub fn config(&self) -> &Config {
        &self.cfg
    }

    /// Commit `pending`: nothing if it equals the current baseline (only masked or live
    /// cells differ), a `Revisit` if it matches a saved capture (or an earlier version of
    /// one), nothing if it matches a rejected one (it still becomes the new comparison
    /// baseline), a `Replace` if it only adds to the saved capture the baseline shows
    /// (`may_replace`: not when the novelty is only where something moved, R1-5 b), and a
    /// `Save` otherwise. Reference: `Detector._commit`, plus the baseline check and
    /// `Replace`.
    fn commit(
        &mut self,
        pending: Pending,
        t_save: f64,
        keep: &[bool],
        reason: SaveReason,
        may_replace: bool,
        events: &mut Vec<Event>,
    ) {
        let thresh = self.cfg.cell_thresh;
        let change_thresh = self.cfg.change_thresh;
        let same =
            |g: &Grid| changed_frac(&pending.grid.data, &g.data, keep, thresh) <= change_thresh;
        if self.last_saved.as_ref().is_some_and(same) {
            return;
        }
        let versions = self
            .history
            .iter()
            .enumerate()
            .rev()
            .map(|(i, g)| (i + 1, g))
            .chain(self.aliases.iter().rev().map(|(i, g)| (*i, g)));
        let mut revisit = None;
        for (index, g) in versions {
            if same(g) {
                revisit = Some((index, g.clone()));
                break;
            }
        }
        if let Some((matches, g)) = revisit {
            events.push(Event::Revisit {
                t_content: pending.t,
                matches,
            });
            self.last_saved = Some(g);
            self.last_saved_index = Some(matches);
            return;
        }
        if let Some(rej) = self.rejected.iter().rev().find(|g| same(g)) {
            self.last_saved = Some(rej.clone());
            self.last_saved_index = None;
            return;
        }
        if may_replace && let (Some(index), Some(base)) = (self.last_saved_index, &self.last_saved)
        {
            let kinds = cell_change_kinds(base, &pending.grid, keep, thresh);
            if kinds.removed < self.cfg.removed_thresh && kinds.added >= self.cfg.added_thresh {
                events.push(Event::Replace {
                    index,
                    t_content: pending.t,
                    t_save,
                    held_t: pending.held_t,
                    reason,
                });
                let old = std::mem::replace(&mut self.history[index - 1], pending.grid.clone());
                self.aliases.push((index, old));
                self.last_saved = Some(pending.grid);
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
        self.last_saved_index = Some(index);
        self.history.push(pending.grid);
    }

    /// Whether `grid` differs from the baseline only where something moved or was touched
    /// recently (`keep_still` leaves those out): a video that stopped, or a camera tile.
    fn motion_only(&self, grid: &Grid, keep_still: &[bool]) -> bool {
        self.last_saved.as_ref().is_some_and(|base| {
            changed_frac(&grid.data, &base.data, keep_still, self.cfg.cell_thresh)
                <= self.cfg.change_thresh
        })
    }

    /// Start a new pending state and report it as held. Reference: `Detector._start_pending`.
    ///
    /// When the new state differs from the baseline outside recently moving cells, the slide
    /// content itself changed: motion seen so far belonged to the old content, so it is
    /// forgotten, except where something is moving or live now. (Otherwise the area of a
    /// video that stopped would turn live as soon as the next slide builds there.)
    fn start_pending(
        &mut self,
        t: f64,
        grid: Grid,
        sharp: f64,
        masks: &Masks,
        events: &mut Vec<Event>,
    ) {
        let new_content = match &self.last_saved {
            None => true,
            Some(base) => {
                changed_frac(
                    &grid.data,
                    &base.data,
                    &masks.keep_still,
                    self.cfg.cell_thresh,
                ) > self.cfg.change_thresh
            }
        };
        if new_content {
            for (last, &kept) in self.last_moving_t.iter_mut().zip(&masks.keep) {
                if kept {
                    *last = f64::NEG_INFINITY;
                }
            }
        }
        self.pending = Some(Pending {
            t,
            grid,
            held_t: t,
            held_sharp: sharp,
        });
        events.push(Event::Hold { t });
    }

    /// Count a touch of cell `c` at `t` (once per `event_gap_s`), and make the cell a live
    /// picture when its strong touches come close together or its run of touches is long.
    fn touch(&mut self, c: usize, t: f64, strong: bool) {
        let cfg = &self.cfg;
        let since = t - self.last_touch_t[c];
        self.last_touch_t[c] = t;
        if since < cfg.event_gap_s - TIME_EPS {
            return;
        }
        if since > cfg.live_run_gap_s {
            self.run_start[c] = t;
            self.run_count[c] = 0;
        }
        self.run_count[c] += 1;
        self.event_t[c] = [t, self.event_t[c][0]];
        let mut live = self.run_count[c] as usize >= cfg.live_sustain_events
            && t - self.run_start[c] >= cfg.live_sustain_s;
        if strong {
            let [latest, before] = self.strong_t[c];
            let first_of_run = if cfg.live_events <= 2 { latest } else { before };
            live |= t - first_of_run <= cfg.live_events_within_s;
            self.strong_t[c] = [t, latest];
        }
        if live {
            if t > self.live_until[c] {
                self.live_since[c] = t;
            }
            self.live_until[c] = self.live_until[c].max(t + cfg.live_window_s);
        }
    }

    /// Updates the motion memory and the live pictures for the sample at `t`, and returns
    /// the cells to compare. `moving` is the dilated motion mask of this sample.
    fn update_motion(
        &mut self,
        t: f64,
        cur: &Grid,
        changed: &[bool],
        moving_raw: &[bool],
        moving: &[bool],
    ) -> Masks {
        let cfg = self.cfg.clone();
        let len = cur.data.len();
        if self.last_moving_t.len() != len {
            self.last_moving_t = vec![f64::NEG_INFINITY; len];
            self.live_until = vec![f64::NEG_INFINITY; len];
            self.live_since = vec![f64::NEG_INFINITY; len];
            self.last_touch_t = vec![f64::NEG_INFINITY; len];
            self.event_t = vec![[f64::NEG_INFINITY; 2]; len];
            self.strong_t = vec![[f64::NEG_INFINITY; 2]; len];
            self.run_start = vec![f64::NEG_INFINITY; len];
            self.run_count = vec![0; len];
            self.touch_ref = None;
        }

        let moving_share = moving.iter().filter(|&&m| m).count() as f64 / len as f64;
        if moving_share <= cfg.live_max_share {
            for (last, &m) in self.last_moving_t.iter_mut().zip(moving) {
                if m {
                    *last = t;
                }
            }
        }
        let recent: Vec<bool> = self
            .last_moving_t
            .iter()
            .map(|&last| t - last <= cfg.live_window_s)
            .collect();

        let changed_count = changed.iter().filter(|&&c| c).count();
        let changed_share = changed_count as f64 / len as f64;
        if changed_share > cfg.slide_change_share {
            // A slide change: earlier touches belonged to the old slide, and so did young
            // live pictures it replaced (a pointer dwelling on text).
            for (c, &ch) in changed.iter().enumerate() {
                let live = t <= self.live_until[c];
                if (live && ch && t - self.live_since[c] < cfg.live_settled_s) || !live {
                    if live {
                        self.live_until[c] = f64::NEG_INFINITY;
                    }
                    self.event_t[c] = [f64::NEG_INFINITY; 2];
                    self.strong_t[c] = [f64::NEG_INFINITY; 2];
                }
            }
        }
        // A live picture stays live while it keeps changing.
        for (until, &ch) in self.live_until.iter_mut().zip(changed) {
            if ch && t <= *until {
                *until = t + cfg.live_window_s;
            }
        }
        // Touches, looked for once per `event_gap_s` on what changed since the last look.
        let due = self
            .touch_ref
            .as_ref()
            .is_some_and(|(ref_t, _)| t - ref_t >= cfg.event_gap_s - TIME_EPS);
        if due {
            let (_, ref_data) = self.touch_ref.take().unwrap();
            let span: Vec<bool> = cur
                .data
                .iter()
                .zip(&ref_data)
                .map(|(&a, &b)| (a as i32 - b as i32).abs() > cfg.cell_thresh)
                .collect();
            let span_share = span.iter().filter(|&&c| c).count() as f64 / len as f64;
            if span_share <= cfg.live_event_share {
                let (w, h) = (cur.width, cur.height);
                // 0: untouched, 1: touched, 2: touched strongly.
                let mut strength = vec![0u8; len];
                for b in clusters(&span, w, h) {
                    if b.cells < cfg.live_min_cells {
                        continue;
                    }
                    let (x0, y0) = (b.x0.saturating_sub(1), b.y0.saturating_sub(1));
                    let (x1, y1) = ((b.x1 + 1).min(w - 1), (b.y1 + 1).min(h - 1));
                    let area = ((x1 - x0 + 1) * (y1 - y0 + 1)) as f64 / len as f64;
                    if area > cfg.live_tile_share {
                        continue;
                    }
                    let level = if b.cells >= cfg.live_strong_cells {
                        2
                    } else {
                        1
                    };
                    for y in y0..=y1 {
                        for v in &mut strength[y * w + x0..=y * w + x1] {
                            *v = (*v).max(level);
                        }
                    }
                }
                for (c, &level) in strength.iter().enumerate() {
                    if level > 0 {
                        self.touch(c, t, level == 2);
                    }
                }
            }
        }
        if due || self.touch_ref.is_none() {
            self.touch_ref = Some((t, cur.data.clone()));
        }
        let live: Vec<bool> = self.live_until.iter().map(|&u| t <= u).collect();
        let live = dilate(&live, cur.width, cur.height, cfg.live_pad);

        let excluded: Vec<bool> = (0..len).map(|c| moving_raw[c] || live[c]).collect();
        let excluded = dilate(&excluded, cur.width, cur.height, cfg.mask_pad);
        let keep: Vec<bool> = excluded.iter().map(|&e| !e).collect();
        let keep_firm = (0..len)
            .map(|c| keep[c] && t - self.event_t[c][1] > cfg.live_events_within_s)
            .collect();
        let keep_still = (0..len)
            .map(|c| keep[c] && !recent[c] && t - self.last_touch_t[c] > cfg.motion_quiet_s)
            .collect();
        Masks {
            keep,
            keep_firm,
            keep_still,
        }
    }

    /// Process one sample (`Detector.step` in the reference).
    ///
    /// Additions to the reference:
    /// - Emit `Hold { t }` whenever the reference starts or replaces the pending state, or
    ///   updates `held_t` to a sharper frame.
    /// - When committing, a pending state equal to an undone state (see [`Self::undo_last`])
    ///   is dropped silently: no `Save`, no `Revisit`; it becomes the comparison baseline
    ///   (`last_saved` in the reference) so it is not proposed again until the screen changes.
    /// - Live pictures are left out of every comparison, and a pending state that differs
    ///   from the baseline only where something moved recently commits at the final quiet
    ///   only once it is `motion_quiet_s` old (module docs).
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
        self.recent.push_back(diff_mask.clone());
        if self.recent.len() > cfg.motion_window {
            self.recent.pop_front();
        }
        let moving_raw = motion_mask(&self.recent, len, cfg.motion_ratio);
        let moving = dilate(&moving_raw, grid.width, grid.height, cfg.mask_pad);
        let masks = self.update_motion(t, grid, &diff_mask, &moving_raw, &moving);
        let keep = &masks.keep;
        let firm = &masks.keep_firm;

        if changed_frac(&grid.data, &prev_data, keep, cfg.cell_thresh) > cfg.change_thresh {
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
            .zip(keep)
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
                            changed_frac(&grid.data, &ls.data, firm, cfg.cell_thresh)
                                > cfg.change_thresh
                        }
                    };
                    if differs {
                        self.start_pending(t, grid.clone(), sharp, &masks, &mut events);
                    }
                }
                Some(mut p) => {
                    let differs = changed_frac(&grid.data, &p.grid.data, firm, cfg.cell_thresh)
                        > cfg.change_thresh;
                    if differs {
                        let kinds = cell_change_kinds(&p.grid, grid, keep, cfg.cell_thresh);
                        let sharpened = sharp >= p.held_sharp * cfg.refine_ratio
                            && coarse_removed_frac(&p.grid, grid, keep, cfg.cell_thresh)
                                < cfg.removed_thresh
                            && kinds.added < cfg.added_thresh;
                        if sharpened {
                            p.grid = grid.clone();
                            p.held_t = t;
                            p.held_sharp = sharp;
                            events.push(Event::Hold { t });
                            self.pending = Some(p);
                        } else if kinds.removed >= cfg.removed_thresh {
                            let back_to_base = self.last_saved.as_ref().is_some_and(|base| {
                                changed_frac(&grid.data, &base.data, firm, cfg.cell_thresh)
                                    <= cfg.change_thresh
                            });
                            if back_to_base && t - p.t < cfg.transient_s {
                                // A brief overlay or a slide flicked past: dropped.
                                return self.end_step(grid, masks, events);
                            }
                            let may_replace = !self.motion_only(&p.grid, &masks.keep_still);
                            self.commit(p, t, firm, SaveReason::Settled, may_replace, &mut events);
                            let differs2 = match &self.last_saved {
                                None => true,
                                Some(ls) => {
                                    changed_frac(&grid.data, &ls.data, firm, cfg.cell_thresh)
                                        > cfg.change_thresh
                                }
                            };
                            if differs2 {
                                self.start_pending(t, grid.clone(), sharp, &masks, &mut events);
                            }
                        } else {
                            self.start_pending(t, grid.clone(), sharp, &masks, &mut events);
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

        if let Some(p) = &self.pending
            && quiet_for >= cfg.final_quiet_s
        {
            // Differs from the baseline only where something moved recently: a video that
            // stopped, or a camera tile that paused and may move again. Wait longer.
            let motion_only = self.motion_only(&p.grid, &masks.keep_still);
            if !motion_only || t - p.t >= cfg.motion_quiet_s {
                let p = self.pending.take().unwrap();
                self.commit(p, t, firm, SaveReason::Settled, !motion_only, &mut events);
            }
        }

        self.end_step(grid, masks, events)
    }

    /// Remember this sample for the next step and return its events.
    fn end_step(&mut self, grid: &Grid, masks: Masks, events: Vec<Event>) -> Vec<Event> {
        self.prev = Some(grid.clone());
        self.last_keep = Some(masks.keep_firm);
        self.last_keep_still = Some(masks.keep_still);
        events
    }

    /// End of input: commit a pending state with reason `End` (`Detector.finish`).
    /// Compares on the cells of the latest step, so a live picture alone saves nothing.
    pub fn finish(&mut self, t: f64) -> Vec<Event> {
        let mut events = Vec::new();
        if let Some(p) = self.pending.take() {
            let all = vec![true; p.grid.data.len()];
            let keep = self.last_keep.take().unwrap_or_else(|| all.clone());
            let keep_still = self.last_keep_still.take().unwrap_or(all);
            let may_replace = !self.motion_only(&p.grid, &keep_still);
            self.commit(p, t, &keep, SaveReason::End, may_replace, &mut events);
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
        self.last_saved_index = Some(index);
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
        // Earlier versions of the undone capture are rejected with it.
        let (gone, kept): (Vec<_>, Vec<_>) = std::mem::take(&mut self.aliases)
            .into_iter()
            .partition(|(i, _)| *i == index);
        self.aliases = kept;
        self.rejected.extend(gone.into_iter().map(|(_, g)| g));
        self.last_saved = self.history.last().cloned();
        self.last_saved_index = self.history.last().map(|_| self.history.len());
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
        self.touch_ref = None;
    }

    /// Diagnostics: per cell, (live now, touches within the window, last touch time).
    #[doc(hidden)]
    pub fn debug_cells(&self, t: f64) -> Vec<(bool, f64, f64)> {
        (0..self.live_until.len())
            .map(|c| {
                (
                    t <= self.live_until[c],
                    self.event_t[c][0],
                    self.live_since[c],
                )
            })
            .collect()
    }

    /// Number of saves currently in the history (after undos).
    pub fn saved_count(&self) -> usize {
        self.history.len()
    }
}
