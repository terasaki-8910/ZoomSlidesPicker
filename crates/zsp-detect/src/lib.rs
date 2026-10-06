//! Slide-change detection for ZoomSlidesPicker.
//!
//! Pure logic, no I/O. The capture layer turns each frame into an analysis [`Grid`]
//! and [`Detail`] with [`analyze`]; [`Detector`] decides which frames to save.
//!
//! The reference implementation is `samples/proto/detect.py` (Python). This crate must
//! make the same decisions on the same input: the fixtures under `fixtures/` were produced
//! by that implementation, and the tests compare against them. SPEC.md > 判定 describes the
//! rules in prose.

mod analyze;
mod detector;

pub use analyze::{Detail, GRID_W, Grid, analyze, grid_shape};
pub use detector::{Config, Detector, Event, SaveReason};
