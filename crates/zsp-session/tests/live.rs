//! Engine-loop behaviour on a live source: pacing, pause/resume, stop, and a lost window.
//! The fake source just counts calls (no real capture), and the interval is short (0.05 s),
//! so the whole file runs in well under 10 s.

use std::io;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Local;
use zsp_output::{RegionRecord, SessionMeta, SessionWriter, Source, SourceKind, Trash};
use zsp_session::{
    Control, EngineHandle, FAILED_IN_A_ROW_BEFORE_LOST, FrameSource, Notice, RgbaImage, Session,
    SourceError, State, Status, spawn,
};

struct NoopTrash;

impl Trash for NoopTrash {
    fn trash(&self, _path: &Path) -> io::Result<()> {
        Ok(())
    }
}

fn meta() -> SessionMeta {
    SessionMeta {
        app_version: "0.1.0".into(),
        source: Source {
            kind: SourceKind::Window,
            app: Some("test".into()),
            title: Some("t".into()),
            platform: "macos".into(),
            file: None,
        },
        region: Some(RegionRecord {
            x: 0.0,
            y: 0.0,
            w: 1.0,
            h: 1.0,
        }),
        interval_s: 0.05,
    }
}

fn new_session(root: &Path) -> Session {
    let writer = SessionWriter::create(&root.join("out"), Local::now(), meta()).unwrap();
    Session::new(writer, zsp_detect::Config::default(), Box::new(NoopTrash))
}

/// A fixed, slightly non-uniform image; its content does not matter to these tests.
fn fake_frame() -> RgbaImage {
    RgbaImage::from_fn(64, 36, |x, y| {
        if (x + y) % 7 == 0 {
            image::Rgba([40, 40, 40, 255])
        } else {
            image::Rgba([220, 220, 220, 255])
        }
    })
}

/// Counts calls to `next_frame`; returns `SourceError::Lost` on every call when `lost` is set.
struct FakeLiveSource {
    calls: Arc<AtomicUsize>,
    lost: bool,
    frame: RgbaImage,
}

impl FrameSource for FakeLiveSource {
    fn next_frame(&mut self, t: f64) -> Result<Option<(f64, RgbaImage)>, SourceError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        if self.lost {
            Err(SourceError::Lost("fake window lost".into()))
        } else {
            Ok(Some((t, self.frame.clone())))
        }
    }

    fn is_live(&self) -> bool {
        true
    }
}

/// Poll `handle.status()` until `pred` holds or `timeout` elapses; returns the last status
/// seen either way, so a timeout still fails the caller's assertion with a clear message.
fn wait_for_status(
    handle: &EngineHandle,
    pred: impl Fn(&Status) -> bool,
    timeout: Duration,
) -> Status {
    let start = Instant::now();
    loop {
        let st = handle.status();
        if pred(&st) || start.elapsed() >= timeout {
            return st;
        }
        thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn live_source_paces_frames_and_reacts_to_pause_resume_stop() {
    let root = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let source = FakeLiveSource {
        calls: calls.clone(),
        lost: false,
        frame: fake_frame(),
    };
    let session = new_session(root.path());
    let handle = spawn(Box::new(source), session, 0.05, Box::new(|_, _| {}));

    thread::sleep(Duration::from_millis(220));
    let before_pause = calls.load(Ordering::SeqCst);
    assert!(
        (2..=20).contains(&before_pause),
        "expected roughly one call per 0.05s interval over 0.22s, got {before_pause}"
    );

    handle.send(Control::Pause);
    let st = wait_for_status(
        &handle,
        |s| s.state == State::Paused,
        Duration::from_secs(1),
    );
    assert_eq!(st.state, State::Paused);
    let at_pause = calls.load(Ordering::SeqCst);

    thread::sleep(Duration::from_millis(150));
    let after_pause_wait = calls.load(Ordering::SeqCst);
    assert!(
        after_pause_wait <= at_pause + 1,
        "no frames should be requested while paused: {at_pause} -> {after_pause_wait}"
    );

    handle.send(Control::Resume);
    let st = wait_for_status(
        &handle,
        |s| s.state == State::Running,
        Duration::from_secs(1),
    );
    assert_eq!(st.state, State::Running);

    thread::sleep(Duration::from_millis(150));
    let after_resume = calls.load(Ordering::SeqCst);
    assert!(
        after_resume > after_pause_wait,
        "frames should be requested again after resume: {after_pause_wait} -> {after_resume}"
    );

    handle.send(Control::Stop);
    handle.join().unwrap();
}

#[test]
fn lost_source_pauses_the_session_and_reports_it() {
    let root = tempfile::tempdir().unwrap();
    let source = FakeLiveSource {
        calls: Arc::new(AtomicUsize::new(0)),
        lost: true,
        frame: fake_frame(),
    };
    let session = new_session(root.path());
    let notices: Arc<Mutex<Vec<Notice>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = notices.clone();
    let handle = spawn(
        Box::new(source),
        session,
        0.05,
        Box::new(move |n, _| sink.lock().unwrap().push(n.clone())),
    );

    let st = wait_for_status(&handle, |s| s.window_lost, Duration::from_secs(2));
    assert!(st.window_lost, "window_lost never became true");
    assert_eq!(st.state, State::Paused);

    handle.send(Control::Stop);
    handle.join().unwrap();

    let seen = notices.lock().unwrap().clone();
    assert!(seen.contains(&Notice::WindowLost), "{seen:?}");
}

/// Fails `failures` times (or forever with `usize::MAX`), then returns frames.
struct FlakySource {
    calls: Arc<AtomicUsize>,
    failures: usize,
    frame: RgbaImage,
}

impl FrameSource for FlakySource {
    fn next_frame(&mut self, t: f64) -> Result<Option<(f64, RgbaImage)>, SourceError> {
        let n = self.calls.fetch_add(1, Ordering::SeqCst);
        if n < self.failures {
            Err(SourceError::Failed("fake capture failure".into()))
        } else {
            Ok(Some((t, self.frame.clone())))
        }
    }

    fn is_live(&self) -> bool {
        true
    }
}

/// A capture that fails for a moment (a window being resized) does not end the session:
/// it keeps running and takes frames again once the source recovers.
#[test]
fn a_few_failed_captures_do_not_end_the_session() {
    let root = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let source = FlakySource {
        calls: calls.clone(),
        failures: (FAILED_IN_A_ROW_BEFORE_LOST - 5) as usize,
        frame: fake_frame(),
    };
    let notices: Arc<Mutex<Vec<Notice>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = notices.clone();
    let handle = spawn(
        Box::new(source),
        new_session(root.path()),
        0.05,
        Box::new(move |n, _| sink.lock().unwrap().push(n.clone())),
    );
    let failures = (FAILED_IN_A_ROW_BEFORE_LOST - 5) as usize;
    let start = Instant::now();
    while calls.load(Ordering::SeqCst) < failures + 5 && start.elapsed() < Duration::from_secs(5) {
        thread::sleep(Duration::from_millis(10));
    }
    let st = handle.status();
    assert_eq!(st.state, State::Running, "still running after the failures");
    assert!(!st.window_lost);
    handle.send(Control::Stop);
    handle.join().unwrap();
    let seen = notices.lock().unwrap().clone();
    assert!(!seen.contains(&Notice::WindowLost), "{seen:?}");
    assert_eq!(
        seen.iter()
            .filter(|n| matches!(n, Notice::Finished { .. }))
            .count(),
        1,
        "finished once, by Stop: {seen:?}"
    );
}

/// Captures that keep failing make the window count as lost (paused and reported), never a
/// silent end of the session.
#[test]
fn failing_captures_count_as_a_lost_window() {
    let root = tempfile::tempdir().unwrap();
    let source = FlakySource {
        calls: Arc::new(AtomicUsize::new(0)),
        failures: usize::MAX,
        frame: fake_frame(),
    };
    let notices: Arc<Mutex<Vec<Notice>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = notices.clone();
    let handle = spawn(
        Box::new(source),
        new_session(root.path()),
        0.05,
        Box::new(move |n, _| sink.lock().unwrap().push(n.clone())),
    );
    let st = wait_for_status(&handle, |s| s.window_lost, Duration::from_secs(5));
    assert!(st.window_lost, "window_lost never became true");
    assert_eq!(st.state, State::Paused);
    let seen = notices.lock().unwrap().clone();
    assert!(seen.contains(&Notice::WindowLost), "{seen:?}");
    assert!(
        !seen.iter().any(|n| matches!(n, Notice::Finished { .. })),
        "not finished by failures: {seen:?}"
    );
    handle.send(Control::Stop);
    handle.join().unwrap();
}

/// After a pause, frames resume at the normal cadence instead of catching up on the ticks
/// missed while paused.
#[test]
fn resume_does_not_catch_up_missed_frames() {
    let root = tempfile::tempdir().unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let source = FakeLiveSource {
        calls: calls.clone(),
        lost: false,
        frame: fake_frame(),
    };
    let handle = spawn(
        Box::new(source),
        new_session(root.path()),
        0.05,
        Box::new(|_, _| {}),
    );
    thread::sleep(Duration::from_millis(150));
    handle.send(Control::Pause);
    wait_for_status(
        &handle,
        |s| s.state == State::Paused,
        Duration::from_secs(1),
    );
    thread::sleep(Duration::from_millis(600)); // 12 intervals missed
    let at_resume = calls.load(Ordering::SeqCst);
    handle.send(Control::Resume);
    thread::sleep(Duration::from_millis(150)); // about 3 intervals
    let delta = calls.load(Ordering::SeqCst) - at_resume;
    assert!(
        delta <= 6,
        "frames after resume should follow the interval, not catch up: {delta} in 0.15 s"
    );
    handle.send(Control::Stop);
    handle.join().unwrap();
}

/// Dropping the handle without Stop (the app quitting) still finishes the session.
#[test]
fn dropping_the_handle_finishes_the_session() {
    let root = tempfile::tempdir().unwrap();
    let source = FakeLiveSource {
        calls: Arc::new(AtomicUsize::new(0)),
        lost: false,
        frame: fake_frame(),
    };
    let handle = spawn(
        Box::new(source),
        new_session(root.path()),
        0.05,
        Box::new(|_, _| {}),
    );
    let dir = handle.status().dir.clone();
    thread::sleep(Duration::from_millis(100));
    drop(handle);
    let start = Instant::now();
    loop {
        let text = std::fs::read_to_string(dir.join("session.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        if json["ended_at"].is_string() {
            break;
        }
        assert!(
            start.elapsed() < Duration::from_secs(3),
            "session was not finished after the handle was dropped"
        );
        thread::sleep(Duration::from_millis(20));
    }
}
