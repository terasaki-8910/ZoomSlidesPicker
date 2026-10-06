//! Session-level behaviour: held frames, manual save, undo, pause, lost windows,
//! single-colour reports, and the threaded engine on a recorded source.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::Local;
use zsp_output::{RegionRecord, SessionMeta, SessionWriter, Source, SourceKind, Trash};
use zsp_session::{Control, FrameSource, Notice, RgbaImage, Session, SourceError, State, spawn};

const W: u32 = 320;
const H: u32 = 180;

/// A white slide with dark bars; `kind` picks the layout so slides differ clearly.
fn slide(kind: u8) -> RgbaImage {
    RgbaImage::from_fn(W, H, |x, y| {
        let bar = match kind {
            b'A' => (20..140).contains(&x) && (y % 30) < 8 && y > 20,
            b'B' => (180..300).contains(&x) && (y % 24) < 10 && y > 10,
            _ => false,
        };
        let title = (20..200).contains(&x) && (4..14).contains(&y) && kind != b'G';
        if kind == b'G' {
            image::Rgba([128, 128, 128, 255])
        } else if bar || title {
            image::Rgba([25, 25, 30, 255])
        } else {
            image::Rgba([250, 250, 250, 255])
        }
    })
}

struct DirTrash(PathBuf);

impl Trash for DirTrash {
    fn trash(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(&self.0)?;
        fs::rename(path, self.0.join(path.file_name().unwrap()))
    }
}

fn meta() -> SessionMeta {
    SessionMeta {
        app_version: "0.1.0".into(),
        source: Source { kind: SourceKind::Window, app: Some("test".into()), title: Some("t".into()), platform: "macos".into(), file: None },
        region: Some(RegionRecord { x: 0.0, y: 0.0, w: 1.0, h: 1.0 }),
        interval_s: 0.5,
    }
}

fn new_session(root: &Path) -> Session {
    let writer = SessionWriter::create(&root.join("out"), Local::now(), meta()).unwrap();
    Session::new(writer, zsp_detect::Config::default(), Box::new(DirTrash(root.join("trash"))))
}

/// Feed `img` every 0.5 s for t in [from, to); collect notices.
fn feed(s: &mut Session, img: &RgbaImage, from: f64, to: f64, out: &mut Vec<Notice>) {
    let mut t = from;
    while t < to - 1e-9 {
        out.extend(s.on_frame(t, img.clone()).unwrap());
        t += 0.5;
    }
}

fn saved(notices: &[Notice]) -> Vec<(usize, PathBuf)> {
    notices
        .iter()
        .filter_map(|n| match n {
            Notice::Saved { index, path } => Some((*index, path.clone())),
            _ => None,
        })
        .collect()
}

fn rgb(img: &RgbaImage) -> Vec<u8> {
    img.pixels().flat_map(|p| [p[0], p[1], p[2]]).collect()
}

fn png_rgb(path: &Path) -> Vec<u8> {
    image::open(path).unwrap().into_rgb8().into_raw()
}

fn session_json(dir: &Path) -> serde_json::Value {
    serde_json::from_str(&fs::read_to_string(dir.join("session.json")).unwrap()).unwrap()
}

#[test]
fn saves_each_slide_and_finishes_with_pdf() {
    let root = tempfile::tempdir().unwrap();
    let mut s = new_session(root.path());
    let mut n = Vec::new();
    feed(&mut s, &slide(b'A'), 0.0, 8.0, &mut n);
    feed(&mut s, &slide(b'B'), 8.0, 16.0, &mut n);
    n.extend(s.finish(16.0, Local::now()).unwrap());

    let saves = saved(&n);
    assert_eq!(saves.len(), 2, "{n:?}");
    assert_eq!(png_rgb(&saves[0].1), rgb(&slide(b'A')));
    assert_eq!(png_rgb(&saves[1].1), rgb(&slide(b'B')));
    let finished = n.iter().find_map(|x| match x {
        Notice::Finished { dir, count, pdf } => Some((dir.clone(), *count, pdf.clone())),
        _ => None,
    });
    let (dir, count, pdf) = finished.expect("Finished notice");
    assert_eq!(count, 2);
    assert!(pdf.expect("pdf written").exists());
    assert_eq!(session_json(&dir)["captures"].as_array().unwrap().len(), 2);
    assert_eq!(s.status().state, State::Stopped);
    assert!(s.finish(16.0, Local::now()).unwrap().is_empty(), "finish is idempotent");
}

/// The held frame is written, not the frame on screen at save time: A is committed when B
/// replaces it (content removed), at which point the current frame is B.
#[test]
fn writes_the_held_frame() {
    let root = tempfile::tempdir().unwrap();
    let mut s = new_session(root.path());
    let mut n = Vec::new();
    feed(&mut s, &slide(b'A'), 0.0, 3.0, &mut n);
    feed(&mut s, &slide(b'B'), 3.0, 5.0, &mut n);
    let saves = saved(&n);
    assert_eq!(saves.len(), 1, "A is committed when B settles: {n:?}");
    assert_eq!(png_rgb(&saves[0].1), rgb(&slide(b'A')));
}

#[test]
fn save_now_writes_the_latest_frame_once() {
    let root = tempfile::tempdir().unwrap();
    let mut s = new_session(root.path());
    let mut n = Vec::new();
    feed(&mut s, &slide(b'A'), 0.0, 2.0, &mut n);
    n.extend(s.control(2.0, Control::SaveNow).unwrap());
    feed(&mut s, &slide(b'A'), 2.0, 14.0, &mut n);
    let saves = saved(&n);
    assert_eq!(saves.len(), 1, "{n:?}");
    assert_eq!(png_rgb(&saves[0].1), rgb(&slide(b'A')));
    let j = session_json(&s.status().dir);
    assert_eq!(j["captures"][0]["reason"], "manual");
    assert_eq!(s.status().count, 1);
}

#[test]
fn undo_moves_the_file_to_trash_and_does_not_resave() {
    let root = tempfile::tempdir().unwrap();
    let mut s = new_session(root.path());
    let mut n = Vec::new();
    feed(&mut s, &slide(b'A'), 0.0, 8.0, &mut n);
    let first = saved(&n);
    assert_eq!(first.len(), 1);
    let undo = s.control(8.0, Control::Undo).unwrap();
    assert_eq!(undo, vec![Notice::Undone { index: 1, path: first[0].1.clone() }]);
    assert!(!first[0].1.exists());
    assert!(root.path().join("trash").join(first[0].1.file_name().unwrap()).exists());
    let mut later = Vec::new();
    feed(&mut s, &slide(b'A'), 8.0, 20.0, &mut later);
    assert!(saved(&later).is_empty(), "{later:?}");
    let j = session_json(&s.status().dir);
    assert_eq!(j["captures"].as_array().unwrap().len(), 0);
    assert!(j["events"].as_array().unwrap().iter().any(|e| e["type"] == "undo"));
    assert_eq!(s.status().count, 0);
    assert_eq!(s.status().last_capture, None);
}

#[test]
fn pause_drops_the_pending_slide_and_ignores_frames() {
    let root = tempfile::tempdir().unwrap();
    let mut s = new_session(root.path());
    let mut n = Vec::new();
    feed(&mut s, &slide(b'A'), 0.0, 2.0, &mut n);
    n.extend(s.control(2.0, Control::Pause).unwrap());
    assert_eq!(s.status().state, State::Paused);
    feed(&mut s, &slide(b'A'), 2.0, 9.0, &mut n);
    assert!(saved(&n).is_empty(), "nothing is saved while paused: {n:?}");
    n.extend(s.control(9.0, Control::Resume).unwrap());
    feed(&mut s, &slide(b'B'), 9.0, 18.0, &mut n);
    n.extend(s.finish(18.0, Local::now()).unwrap());
    let saves = saved(&n);
    assert_eq!(saves.len(), 1, "only B: {n:?}");
    assert_eq!(png_rgb(&saves[0].1), rgb(&slide(b'B')));
    let events: Vec<String> = session_json(&s.status().dir)["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["type"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(events, vec!["paused", "resumed"]);
}

#[test]
fn lost_window_pauses_once_and_reports_its_return() {
    let root = tempfile::tempdir().unwrap();
    let mut s = new_session(root.path());
    let mut n = Vec::new();
    feed(&mut s, &slide(b'A'), 0.0, 8.0, &mut n);
    assert_eq!(s.on_lost(8.0).unwrap(), vec![Notice::WindowLost]);
    assert!(s.on_lost(8.5).unwrap().is_empty(), "one report per episode");
    let st = s.status();
    assert_eq!((st.state, st.window_lost), (State::Paused, true));

    assert_eq!(s.on_frame(10.0, slide(b'B')).unwrap(), vec![Notice::WindowBack]);
    let st = s.status();
    assert_eq!((st.state, st.window_lost), (State::Paused, false), "stays paused until resumed");
    assert!(s.on_frame(10.5, slide(b'B')).unwrap().is_empty());
    let events: Vec<String> = session_json(&st.dir)["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["type"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(events, vec!["window_lost", "window_back"]);
}

#[test]
fn single_colour_stretch_is_reported_once_and_not_saved() {
    let root = tempfile::tempdir().unwrap();
    let mut s = new_session(root.path());
    let mut n = Vec::new();
    feed(&mut s, &slide(b'G'), 0.0, 25.0, &mut n);
    n.extend(s.finish(25.0, Local::now()).unwrap());
    assert!(saved(&n).is_empty(), "{n:?}");
    assert_eq!(n.iter().filter(|x| **x == Notice::Blank).count(), 1);
    let j = session_json(&s.status().dir);
    assert_eq!(j["events"].as_array().unwrap().iter().filter(|e| e["type"] == "blank").count(), 1);
}

/// A recorded source: frames as fast as they decode, then the session finishes by itself.
struct VecSource(Vec<(f64, RgbaImage)>);

impl FrameSource for VecSource {
    fn next_frame(&mut self, _t: f64) -> Result<Option<(f64, RgbaImage)>, SourceError> {
        Ok(if self.0.is_empty() { None } else { Some(self.0.remove(0)) })
    }
    fn is_live(&self) -> bool {
        false
    }
}

#[test]
fn spawn_runs_a_recorded_source_to_the_end() {
    let root = tempfile::tempdir().unwrap();
    let s = new_session(root.path());
    let mut frames = Vec::new();
    for i in 0..32 {
        let t = i as f64 * 0.5;
        frames.push((t, slide(if t < 8.0 { b'A' } else { b'B' })));
    }
    let seen: Arc<Mutex<Vec<Notice>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let handle = spawn(Box::new(VecSource(frames)), s, 0.5, Box::new(move |n, _| sink.lock().unwrap().push(n.clone())));
    let status_dir = handle.status().dir.clone();
    handle.join().unwrap();
    let seen = seen.lock().unwrap();
    assert_eq!(saved(&seen).len(), 2, "{seen:?}");
    assert!(seen.iter().any(|n| matches!(n, Notice::Finished { count: 2, .. })));
    assert!(status_dir.join("slides.pdf").exists());
}
