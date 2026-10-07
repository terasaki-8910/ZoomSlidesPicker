//! Slide-change detection for ZoomSlidesPicker.
//!
//! Pure logic, no I/O. The capture layer turns each frame into an analysis [`Grid`]
//! and [`Detail`] with [`analyze`]; [`Detector`] decides which frames to save.
//!
//! The reference implementation is `samples/proto/detect.py` (Python). This crate makes the
//! same decisions on the same input, except for the additions listed in the `detector`
//! module docs (live pictures, brief states, replacing a build step, manual save, undo,
//! pause) and [`fold_captures`]: the fixtures under `fixtures/`
//! and their expectations come from `samples/fixture/`, and the tests compare against them.
//! SPEC.md > 判定 describes the rules in prose.

mod analyze;
mod detector;
mod fold;

pub use analyze::{Detail, GRID_W, Grid, analyze, grid_shape};
pub use detector::{Config, Detector, Event, SaveReason};
pub use fold::fold_captures;
