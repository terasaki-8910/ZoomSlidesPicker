//! Session engine for ZoomSlidesPicker: frames in, captures out.
//!
//! [`Session`] is the single-threaded core: it analyzes each (already cropped) frame, runs
//! the detector, follows its Hold/Save protocol (zsp-detect docs), and writes through
//! zsp-output. Memory: at most two full-size frames, the held frame of the pending state
//! and the latest frame (SPEC.md > 資源の目標).
//!
//! [`spawn`] runs a session on a [`FrameSource`] in a thread and applies [`Control`]s
//! between frames.

use std::io;
use std::path::PathBuf;

use chrono::{DateTime, Local};
use serde::Serialize;
use zsp_output::{SessionWriter, Trash};

pub use zsp_capture::RgbaImage;

mod engine;
mod sources;

pub use engine::{EngineHandle, FrameSource, Notify, SourceError, spawn};
pub use sources::{VideoSource, WindowSource};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Running,
    Paused,
    Stopped,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Status {
    pub state: State,
    /// Captures currently in the session (after undos).
    pub count: usize,
    pub dir: PathBuf,
    /// The most recent capture still in the session, if any.
    pub last_capture: Option<PathBuf>,
    /// Session time of the last frame or control seen.
    pub elapsed_s: f64,
    /// The latest frame was single-colour and the stretch has been reported.
    pub blank: bool,
    /// Capture failed and the window has not come back yet.
    pub window_lost: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Control {
    Pause,
    Resume,
    SaveNow,
    Undo,
    Stop,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Notice {
    Saved {
        index: usize,
        path: PathBuf,
    },
    Undone {
        index: usize,
        path: PathBuf,
    },
    /// A single-colour stretch reached 10 s (reported once per stretch).
    Blank,
    WindowLost,
    WindowBack,
    Finished {
        dir: PathBuf,
        count: usize,
        pdf: Option<PathBuf>,
    },
}

/// The single-threaded session core.
pub struct Session {
    _private: (),
}

impl Session {
    pub fn new(
        writer: SessionWriter,
        cfg: zsp_detect::Config,
        trash: Box<dyn Trash + Send>,
    ) -> Self {
        let _ = (writer, cfg, trash);
        todo!()
    }

    /// A captured, cropped frame at session time `t` (seconds since the start).
    ///
    /// Running: analyze, step the detector, and apply its events in order. `Hold` keeps this
    /// frame as the held frame; `Save` (settled or end) writes the held frame;
    /// `Revisit` and `Blank` are recorded in session.json.
    /// Paused: the frame is ignored, except that the first frame after a lost window records
    /// `window_back` (once) and returns [`Notice::WindowBack`]; the session stays paused until
    /// [`Control::Resume`].
    /// Stopped: ignored.
    pub fn on_frame(&mut self, t: f64, frame: RgbaImage) -> io::Result<Vec<Notice>> {
        let _ = (t, frame);
        todo!()
    }

    /// Capture failed (window closed or minimized). While running: pause (dropping the held
    /// frame), record `window_lost`, return [`Notice::WindowLost`]. Repeated failures in the
    /// same episode record nothing more.
    pub fn on_lost(&mut self, t: f64) -> io::Result<Vec<Notice>> {
        let _ = t;
        todo!()
    }

    /// Apply a control at session time `t`.
    /// - `Pause`: detector pause, drop the held frame, record `paused`.
    /// - `Resume`: detector resume at `t`, record `resumed`, clear `window_lost`.
    /// - `SaveNow` (running, with a latest frame): manual save of the latest frame.
    /// - `Undo`: undo the latest capture in the detector and move its PNG to the trash
    ///   ([`Notice::Undone`]).
    /// - `Stop`: same as [`Session::finish`] with the current local time.
    ///
    /// Controls that do not apply in the current state (e.g. `Resume` while running) do nothing.
    pub fn control(&mut self, t: f64, c: Control) -> io::Result<Vec<Notice>> {
        let _ = (t, c);
        todo!()
    }

    /// End the session: detector finish (a pending state is saved with reason end), then
    /// write slides.pdf and the final session.json. Returns [`Notice::Finished`].
    /// Calling it again does nothing.
    pub fn finish(&mut self, t: f64, ended_at: DateTime<Local>) -> io::Result<Vec<Notice>> {
        let _ = (t, ended_at);
        todo!()
    }

    pub fn status(&self) -> Status {
        todo!()
    }
}
