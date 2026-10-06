//! Running a session in a thread.

use std::io;
use std::sync::mpsc::RecvTimeoutError;
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use chrono::Local;

use crate::{Control, Notice, RgbaImage, Session, State, Status};

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
    let (tx, rx) = mpsc::channel();
    let status = Arc::new(Mutex::new(session.status()));
    let thread_status = Arc::clone(&status);

    let join = thread::spawn(move || run(source, session, interval_s, rx, thread_status, notify));

    EngineHandle {
        tx,
        status,
        join: Some(join),
    }
}

/// Store the session's current status and notify for each notice in order, all with that
/// same status (the shared status is updated after every frame and control; `notify` gets
/// the status after it for every notice).
fn publish(session: &Session, status: &Arc<Mutex<Status>>, notify: &Notify, notices: Vec<Notice>) {
    let st = session.status();
    *status.lock().unwrap() = st.clone();
    for n in &notices {
        notify(n, &st);
    }
}

/// Apply one control and publish the resulting status. Returns whether the thread should
/// end now (a `Stop`, which `Session::control` turns into `finish`).
fn handle_control(
    session: &mut Session,
    t: f64,
    c: Control,
    status: &Arc<Mutex<Status>>,
    notify: &Notify,
) -> io::Result<bool> {
    let is_stop = c == Control::Stop;
    let notices = session.control(t, c)?;
    publish(session, status, notify, notices);
    Ok(is_stop)
}

fn run(
    source: Box<dyn FrameSource>,
    session: Session,
    interval_s: f64,
    rx: mpsc::Receiver<Control>,
    status: Arc<Mutex<Status>>,
    notify: Notify,
) -> io::Result<()> {
    if source.is_live() {
        run_live(source, session, interval_s, rx, &status, &notify)
    } else {
        run_recorded(source, session, rx, &status, &notify)
    }
}

/// Decode and feed frames as fast as they come; controls are checked between frames.
fn run_recorded(
    mut source: Box<dyn FrameSource>,
    mut session: Session,
    rx: mpsc::Receiver<Control>,
    status: &Arc<Mutex<Status>>,
    notify: &Notify,
) -> io::Result<()> {
    let mut t = 0.0_f64;
    loop {
        while let Ok(c) = rx.try_recv() {
            if handle_control(&mut session, t, c, status, notify)? {
                return Ok(());
            }
        }

        match source.next_frame(t) {
            Ok(Some((frame_t, frame))) => {
                t = frame_t;
                let notices = session.on_frame(t, frame)?;
                publish(&session, status, notify, notices);
            }
            Ok(None) => {
                let notices = session.finish(t, Local::now())?;
                publish(&session, status, notify, notices);
                return Ok(());
            }
            Err(SourceError::Lost(_)) => {
                let notices = session.on_lost(t)?;
                publish(&session, status, notify, notices);
            }
            Err(SourceError::Failed(_)) => {
                let notices = session.finish(t, Local::now())?;
                publish(&session, status, notify, notices);
                return Ok(());
            }
        }
    }
}

/// One frame per `interval_s` of wall-clock time; controls are reacted to immediately
/// through the same wait. While paused (and the window is not lost), no frames are
/// requested at all; while the window is lost, the source is polled every 2 s.
fn run_live(
    mut source: Box<dyn FrameSource>,
    mut session: Session,
    interval_s: f64,
    rx: mpsc::Receiver<Control>,
    status: &Arc<Mutex<Status>>,
    notify: &Notify,
) -> io::Result<()> {
    let start = Instant::now();
    let interval = Duration::from_secs_f64(interval_s.max(0.0));
    let mut next_tick = Duration::ZERO;

    loop {
        let snapshot = session.status();

        if snapshot.state == State::Paused && !snapshot.window_lost {
            match rx.recv() {
                Ok(c) => {
                    let t = start.elapsed().as_secs_f64();
                    if handle_control(&mut session, t, c, status, notify)? {
                        return Ok(());
                    }
                }
                Err(_) => return Ok(()),
            }
            continue;
        }

        let wait = if snapshot.state == State::Paused {
            Duration::from_secs_f64(2.0)
        } else {
            next_tick.saturating_sub(start.elapsed())
        };

        match rx.recv_timeout(wait) {
            Ok(c) => {
                let t = start.elapsed().as_secs_f64();
                if handle_control(&mut session, t, c, status, notify)? {
                    return Ok(());
                }
                continue;
            }
            Err(RecvTimeoutError::Disconnected) => return Ok(()),
            Err(RecvTimeoutError::Timeout) => {}
        }

        let t = start.elapsed().as_secs_f64();
        match source.next_frame(t) {
            Ok(Some((frame_t, frame))) => {
                let notices = session.on_frame(frame_t, frame)?;
                publish(&session, status, notify, notices);
                next_tick += interval;
            }
            Ok(None) => {
                let notices = session.finish(t, Local::now())?;
                publish(&session, status, notify, notices);
                return Ok(());
            }
            Err(SourceError::Lost(_)) => {
                let notices = session.on_lost(t)?;
                publish(&session, status, notify, notices);
            }
            Err(SourceError::Failed(_)) => {
                let notices = session.finish(t, Local::now())?;
                publish(&session, status, notify, notices);
                return Ok(());
            }
        }
    }
}
