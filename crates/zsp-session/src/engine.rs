//! Running a session in a thread.

use std::io;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::JoinHandle;

use crate::{Control, Notice, RgbaImage, Session, Status};

#[derive(Debug)]
pub enum SourceError {
    /// The window is gone or minimized; the session pauses and waits for it.
    Lost(String),
    /// Anything else; the session stops.
    Failed(String),
}

/// Where frames come from.
pub trait FrameSource: Send {
    /// The next frame. Live sources capture now and return `t` with the frame; recorded
    /// sources return their own time. `Ok(None)` means the input has ended.
    fn next_frame(&mut self, t: f64) -> Result<Option<(f64, RgbaImage)>, SourceError>;

    /// Live sources run in real time (one frame per interval); recorded ones as fast as
    /// frames decode.
    fn is_live(&self) -> bool;
}

/// Control and observe a running session.
pub struct EngineHandle {
    tx: mpsc::Sender<Control>,
    status: Arc<Mutex<Status>>,
    join: Option<JoinHandle<io::Result<()>>>,
}

impl EngineHandle {
    /// Queue a control; applied before the next frame. Ignored once the session has ended.
    pub fn send(&self, c: Control) {
        let _ = self.tx.send(c);
    }

    pub fn status(&self) -> Status {
        self.status.lock().unwrap().clone()
    }

    /// Wait for the session thread to end (after `Stop`, or at the end of a recorded input).
    pub fn join(mut self) -> io::Result<()> {
        match self.join.take() {
            Some(h) => h
                .join()
                .unwrap_or_else(|_| Err(io::Error::other("session thread panicked"))),
            None => Ok(()),
        }
    }
}

/// Receives every notice together with the status after it (called on the session thread).
pub type Notify = Box<dyn Fn(&Notice, &Status) + Send>;

/// Run `session` on `source` in a new thread.
///
/// - Live source: one frame every `interval_s` of wall-clock time; `t` is seconds since
///   `spawn`. While paused, no frames are taken, except that while the window is lost the
///   source is polled every 2 s so that its return is noticed (`Notice::WindowBack`).
/// - Recorded source: frames as fast as they decode; at the end of input the session
///   finishes by itself.
/// - `SourceError::Lost` -> `Session::on_lost`; `SourceError::Failed` -> finish and end.
/// - Controls are applied between frames; `Stop` finishes and ends the thread.
/// - `notify` gets every notice together with the status after it; the shared status is
///   updated after every frame and control.
pub fn spawn(
    source: Box<dyn FrameSource>,
    session: Session,
    interval_s: f64,
    notify: Notify,
) -> EngineHandle {
    let _ = (source, session, interval_s, notify);
    todo!()
}
