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
use zsp_detect::{Detector, Event, Grid, SaveReason};
use zsp_output::{CaptureReason, EventKind, SessionWriter, Trash};

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
    writer: SessionWriter,
    detector: Detector,
    trash: Box<dyn Trash + Send>,
    state: State,
    /// The frame held for the pending state, if any (at most one of the two full-size
    /// frames this session keeps).
    held: Option<RgbaImage>,
    /// The most recently analyzed frame and its grid, for `Control::SaveNow` (the other of
    /// the two full-size frames this session keeps).
    latest: Option<(RgbaImage, Grid)>,
    /// Session time of the last frame or control seen.
    elapsed_s: f64,
    blank: bool,
    window_lost: bool,
}

impl Session {
    pub fn new(
        writer: SessionWriter,
        cfg: zsp_detect::Config,
        trash: Box<dyn Trash + Send>,
    ) -> Self {
        Self {
            writer,
            detector: Detector::new(cfg),
            trash,
            state: State::Running,
            held: None,
            latest: None,
            elapsed_s: 0.0,
            blank: false,
            window_lost: false,
        }
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
        self.elapsed_s = t;
        match self.state {
            State::Stopped => Ok(Vec::new()),
            State::Paused => {
                if self.window_lost {
                    self.window_lost = false;
                    self.writer.add_event(t, EventKind::WindowBack)?;
                    Ok(vec![Notice::WindowBack])
                } else {
                    Ok(Vec::new())
                }
            }
            State::Running => {
                let (w, h) = frame.dimensions();
                let (grid, detail) = zsp_detect::analyze(frame.as_raw(), w, h);
                if population_std_dev(&grid.data) >= self.detector.config().uniform_std {
                    self.blank = false;
                }
                let events = self.detector.step(t, &grid, &detail);
                let notices = self.apply_events(events, Some(&frame))?;
                self.latest = Some((frame, grid));
                Ok(notices)
            }
        }
    }

    /// Capture failed (window closed or minimized). While running: pause (dropping the held
    /// frame), record `window_lost`, return [`Notice::WindowLost`]. Repeated failures in the
    /// same episode record nothing more.
    pub fn on_lost(&mut self, t: f64) -> io::Result<Vec<Notice>> {
        self.elapsed_s = t;
        if self.state != State::Running {
            return Ok(Vec::new());
        }
        self.detector.pause();
        self.held = None;
        self.state = State::Paused;
        self.window_lost = true;
        self.writer.add_event(t, EventKind::WindowLost)?;
        Ok(vec![Notice::WindowLost])
    }

    /// Apply a control at session time `t`.
    /// - `Pause`: detector pause, drop the held frame, record `paused`.
    /// - `Resume`: detector resume at `t`, record `resumed`, clear `window_lost`.
    /// - `SaveNow` (running, with a latest frame): manual save of the latest frame.
    /// - `Undo` (running or paused): undo the latest capture in the detector and move its PNG
    ///   to the trash ([`Notice::Undone`]). After finishing it does nothing (the PDF exists).
    /// - `Stop`: same as [`Session::finish`] with the current local time.
    ///
    /// Controls that do not apply in the current state (e.g. `Resume` while running) do nothing.
    pub fn control(&mut self, t: f64, c: Control) -> io::Result<Vec<Notice>> {
        self.elapsed_s = t;
        match c {
            Control::Pause => {
                if self.state != State::Running {
                    return Ok(Vec::new());
                }
                self.detector.pause();
                self.held = None;
                self.state = State::Paused;
                self.writer.add_event(t, EventKind::Paused)?;
                Ok(Vec::new())
            }
            Control::Resume => {
                if self.state != State::Paused {
                    return Ok(Vec::new());
                }
                self.detector.resume(t);
                self.state = State::Running;
                self.window_lost = false;
                self.writer.add_event(t, EventKind::Resumed)?;
                Ok(Vec::new())
            }
            Control::SaveNow => {
                if self.state != State::Running {
                    return Ok(Vec::new());
                }
                let Some((frame, grid)) = &self.latest else {
                    return Ok(Vec::new());
                };
                let Event::Save {
                    index,
                    t_content,
                    t_save,
                    reason,
                    ..
                } = self.detector.save_now(t, grid)
                else {
                    unreachable!("Detector::save_now always returns Event::Save")
                };
                let (w, h) = frame.dimensions();
                let path = self.writer.save_capture(
                    index,
                    frame.as_raw(),
                    w,
                    h,
                    t_content,
                    t_save,
                    to_capture_reason(reason),
                )?;
                Ok(vec![Notice::Saved { index, path }])
            }
            Control::Undo => {
                // After finishing, the PDF already contains the capture: undo no longer applies.
                if self.state == State::Stopped {
                    return Ok(Vec::new());
                }
                let Some(index) = self.detector.undo_last() else {
                    return Ok(Vec::new());
                };
                let Some(path) = self.writer.undo_last(t, self.trash.as_ref())? else {
                    return Ok(Vec::new());
                };
                Ok(vec![Notice::Undone { index, path }])
            }
            Control::Stop => self.finish(t, Local::now()),
        }
    }

    /// End the session: detector finish (a pending state is saved with reason end), then
    /// write slides.pdf and the final session.json. Returns [`Notice::Finished`].
    /// Calling it again does nothing.
    pub fn finish(&mut self, t: f64, ended_at: DateTime<Local>) -> io::Result<Vec<Notice>> {
        if self.state == State::Stopped {
            return Ok(Vec::new());
        }
        self.elapsed_s = t;
        let events = self.detector.finish(t);
        let mut notices = self.apply_events(events, None)?;

        let pdf = self.writer.finish(ended_at)?;
        self.state = State::Stopped;
        notices.push(Notice::Finished {
            dir: self.writer.dir().to_path_buf(),
            count: self.writer.capture_count(),
            pdf,
        });
        Ok(notices)
    }

    pub fn status(&self) -> Status {
        Status {
            state: self.state,
            count: self.writer.capture_count(),
            dir: self.writer.dir().to_path_buf(),
            last_capture: self
                .writer
                .session()
                .captures
                .last()
                .map(|c| self.writer.dir().join(&c.file)),
            elapsed_s: self.elapsed_s,
            blank: self.blank,
            window_lost: self.window_lost,
        }
    }

    /// Apply detector events in order, following the Hold/Save/Revisit/Blank protocol
    /// (zsp_detect::detector module docs). `frame` is the current sample's frame, needed for
    /// `Hold`; `on_frame` always has one, `finish` never emits a `Hold` so it passes `None`.
    fn apply_events(
        &mut self,
        events: Vec<Event>,
        frame: Option<&RgbaImage>,
    ) -> io::Result<Vec<Notice>> {
        let mut notices = Vec::new();
        for ev in events {
            match ev {
                Event::Hold { .. } => {
                    self.held = Some(
                        frame
                            .expect("Hold is only emitted while on_frame is stepping the detector")
                            .clone(),
                    );
                }
                Event::Save {
                    index,
                    t_content,
                    t_save,
                    reason,
                    ..
                } => {
                    let held = self
                        .held
                        .take()
                        .expect("a Save is always preceded by a Hold for the held state");
                    let (w, h) = held.dimensions();
                    let path = self.writer.save_capture(
                        index,
                        held.as_raw(),
                        w,
                        h,
                        t_content,
                        t_save,
                        to_capture_reason(reason),
                    )?;
                    notices.push(Notice::Saved { index, path });
                }
                Event::Revisit { t_content, matches } => {
                    self.writer.add_revisit(t_content, matches)?;
                }
                Event::Blank { t } => {
                    self.writer.add_event(t, EventKind::Blank)?;
                    self.blank = true;
                    notices.push(Notice::Blank);
                }
            }
        }
        Ok(notices)
    }
}

/// Population standard deviation (ddof 0), matching the private `std_dev` in
/// `zsp_detect::detector` that decides single-colour frames there. Used here only to clear
/// `Status::blank` once a frame is no longer single-colour; the detector itself reports
/// `Event::Blank` only when a stretch starts or has lasted long enough.
fn population_std_dev(data: &[u8]) -> f64 {
    let n = data.len() as f64;
    let mean = data.iter().map(|&v| v as f64).sum::<f64>() / n;
    let var = data.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n;
    var.sqrt()
}

fn to_capture_reason(reason: SaveReason) -> CaptureReason {
    match reason {
        SaveReason::Settled => CaptureReason::Settled,
        SaveReason::End => CaptureReason::End,
        SaveReason::Manual => CaptureReason::Manual,
    }
}
