//! ACCEPTANCE B2-B7 and B9: files, session.json, PDF, undo, rebuild.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, TimeZone};
use serde_json::Value;
use zsp_output::{
    CaptureReason, EventKind, PDF_FILE, RegionRecord, SESSION_FILE, SessionMeta, SessionWriter,
    Source, SourceKind, Trash, capture_file_name, create_session_dir, rebuild_pdf,
    session_dir_name,
};

fn started() -> DateTime<Local> {
    Local
        .with_ymd_and_hms(2026, 10, 6, 9, 5, 0)
        .single()
        .unwrap()
}

fn meta() -> SessionMeta {
    SessionMeta {
        app_version: "0.1.0".into(),
        source: Source {
            kind: SourceKind::Window,
            app: Some("zoom.us".into()),
            title: Some("Zoom Meeting".into()),
            platform: "macos".into(),
            file: None,
        },
        region: Some(RegionRecord {
            x: 0.1,
            y: 0.1,
            w: 0.8,
            h: 0.7,
        }),
        interval_s: 0.5,
    }
}

fn pattern(w: u32, h: u32, seed: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            v.extend_from_slice(&[
                ((x * 7 + y * 13 + seed) % 256) as u8,
                ((x * 3 + seed) % 256) as u8,
                ((y * 5) % 256) as u8,
                255,
            ]);
        }
    }
    v
}

fn rgb_of(rgba: &[u8]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|p| [p[0], p[1], p[2]])
        .collect()
}

/// Moves files into a folder instead of the OS trash.
struct DirTrash(PathBuf);

impl Trash for DirTrash {
    fn trash(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(&self.0)?;
        fs::rename(path, self.0.join(path.file_name().unwrap()))
    }
}

fn read_session(dir: &Path) -> Value {
    serde_json::from_str(&fs::read_to_string(dir.join(SESSION_FILE)).unwrap()).unwrap()
}

fn assert_schema_valid(instance: &Value) {
    let schema_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/session.schema.json");
    let schema: Value = serde_json::from_str(&fs::read_to_string(schema_path).unwrap()).unwrap();
    let validator = jsonschema::validator_for(&schema).expect("schema compiles");
    let errors: Vec<String> = validator
        .iter_errors(instance)
        .map(|e| format!("{e} at {}", e.instance_path()))
        .collect();
    assert!(
        errors.is_empty(),
        "session.json does not match the schema: {errors:#?}"
    );
}

fn num(o: &lopdf::Object) -> f64 {
    match o {
        lopdf::Object::Integer(i) => *i as f64,
        lopdf::Object::Real(f) => *f as f64,
        other => panic!("not a number: {other:?}"),
    }
}

/// Page sizes (width, height) in page order, from each page's own MediaBox.
fn pdf_page_sizes(path: &Path) -> Vec<(f64, f64)> {
    let doc = lopdf::Document::load(path).unwrap();
    doc.get_pages()
        .values()
        .map(|id| {
            let page = doc.get_dictionary(*id).unwrap();
            let mb = page
                .get(b"MediaBox")
                .expect("each page has its own MediaBox")
                .as_array()
                .unwrap();
            (num(&mb[2]) - num(&mb[0]), num(&mb[3]) - num(&mb[1]))
        })
        .collect()
}

/// B2: file names.
#[test]
fn b2_capture_file_names() {
    assert_eq!(capture_file_name(1, 0.0), "001_00-00-00.png");
    assert_eq!(capture_file_name(12, 3725.4), "012_01-02-05.png");
    assert_eq!(capture_file_name(999, 59.99), "999_00-00-59.png");
    assert_eq!(capture_file_name(1000, 61.0), "1000_00-01-01.png");
}

/// B6: session folder names, with -2, -3 when taken.
#[test]
fn b6_session_folder_names() {
    assert_eq!(session_dir_name(&started()), "2026-10-06_0905");
    let root = tempfile::tempdir().unwrap();
    let a = create_session_dir(&root.path().join("nested/root"), &started()).unwrap();
    let b = create_session_dir(&root.path().join("nested/root"), &started()).unwrap();
    let c = create_session_dir(&root.path().join("nested/root"), &started()).unwrap();
    assert_eq!(a.file_name().unwrap(), "2026-10-06_0905");
    assert_eq!(b.file_name().unwrap(), "2026-10-06_0905-2");
    assert_eq!(c.file_name().unwrap(), "2026-10-06_0905-3");
    assert!(a.is_dir() && b.is_dir() && c.is_dir());
}

/// B5: PNG captures are lossless.
#[test]
fn b5_png_is_lossless() {
    let root = tempfile::tempdir().unwrap();
    let mut w = SessionWriter::create(root.path(), started(), meta()).unwrap();
    let rgba = pattern(37, 23, 5);
    let path = w
        .save_capture(1, &rgba, 37, 23, 1.0, 5.0, CaptureReason::Settled)
        .unwrap();
    assert_eq!(path.file_name().unwrap(), "001_00-00-01.png");
    let back = image::open(&path).unwrap().into_rgb8();
    assert_eq!(back.dimensions(), (37, 23));
    assert_eq!(back.into_raw(), rgb_of(&rgba));
    let rec = &w.session().captures[0];
    assert_eq!(
        (rec.index, rec.width, rec.height, rec.reason),
        (1, 37, 23, CaptureReason::Settled)
    );
}

/// B3: session.json matches the schema mid-session (as left by a crash) and after finishing.
#[test]
fn b3_session_json_matches_schema() {
    let root = tempfile::tempdir().unwrap();
    let mut w = SessionWriter::create(root.path(), started(), meta()).unwrap();
    assert_schema_valid(&read_session(w.dir()));

    w.save_capture(
        1,
        &pattern(20, 10, 1),
        20,
        10,
        1.0,
        5.0,
        CaptureReason::Settled,
    )
    .unwrap();
    w.save_capture(
        2,
        &pattern(20, 10, 2),
        20,
        10,
        15.5,
        19.5,
        CaptureReason::Manual,
    )
    .unwrap();
    w.add_revisit(53.0, 1).unwrap();
    w.add_event(60.0, EventKind::Paused).unwrap();
    w.add_event(61.0, EventKind::Resumed).unwrap();
    let mid = read_session(w.dir());
    assert_schema_valid(&mid);
    assert!(mid["ended_at"].is_null());
    assert!(mid["pdf"].is_null());
    assert_eq!(mid["captures"].as_array().unwrap().len(), 2);

    let pdf = w
        .finish(
            Local
                .with_ymd_and_hms(2026, 10, 6, 10, 35, 0)
                .single()
                .unwrap(),
        )
        .unwrap();
    assert_eq!(pdf.as_deref(), Some(w.dir().join(PDF_FILE).as_path()));
    let done = read_session(w.dir());
    assert_schema_valid(&done);
    assert!(done["ended_at"].is_string());
    assert_eq!(done["pdf"]["pages"], 2);
    assert!(done["pdf"]["rebuilt_at"].is_null());
    // No temp file left behind by the atomic rewrite.
    let names: Vec<String> = fs::read_dir(w.dir())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        names
            .iter()
            .all(|n| n.ends_with(".png") || n == SESSION_FILE || n == PDF_FILE),
        "unexpected files: {names:?}"
    );
}

/// B4: one page per capture, in order, page size = image size; images embedded losslessly.
#[test]
fn b4_pdf_pages_sizes_and_lossless_images() {
    let root = tempfile::tempdir().unwrap();
    let mut w = SessionWriter::create(root.path(), started(), meta()).unwrap();
    let sizes = [(64u32, 48u32), (100, 50), (37, 91)];
    let first = pattern(64, 48, 9);
    for (i, &(sw, sh)) in sizes.iter().enumerate() {
        let rgba = if i == 0 {
            first.clone()
        } else {
            pattern(sw, sh, i as u32)
        };
        w.save_capture(
            i + 1,
            &rgba,
            sw,
            sh,
            i as f64 * 10.0,
            i as f64 * 10.0 + 4.0,
            CaptureReason::Settled,
        )
        .unwrap();
    }
    let pdf = w.finish(Local::now()).unwrap().unwrap();
    let got = pdf_page_sizes(&pdf);
    let want: Vec<(f64, f64)> = sizes.iter().map(|&(a, b)| (a as f64, b as f64)).collect();
    assert_eq!(got, want);

    // The first page's image is 8-bit RGB, Flate-compressed, byte-identical to the capture.
    let doc = lopdf::Document::load(&pdf).unwrap();
    let page_id = *doc.get_pages().values().next().unwrap();
    let (resources, _) = doc.get_page_resources(page_id).unwrap();
    let resources = resources.expect("page has a resource dictionary");
    let xobjects = resources.get(b"XObject").unwrap().as_dict().unwrap();
    let (_, xref) = xobjects.iter().next().expect("one image XObject");
    let stream = doc
        .get_object(xref.as_reference().unwrap())
        .unwrap()
        .as_stream()
        .unwrap();
    assert_eq!(
        stream.dict.get(b"Filter").unwrap().as_name().unwrap(),
        b"FlateDecode"
    );
    assert_eq!(
        stream
            .dict
            .get(b"BitsPerComponent")
            .unwrap()
            .as_i64()
            .unwrap(),
        8
    );
    assert_eq!(
        stream.dict.get(b"ColorSpace").unwrap().as_name().unwrap(),
        b"DeviceRGB"
    );
    assert_eq!(stream.decompressed_content().unwrap(), rgb_of(&first));
}

/// B7: undo moves the file to the trash (not deleted) and removes its record.
#[test]
fn b7_undo_moves_to_trash() {
    let root = tempfile::tempdir().unwrap();
    let trash_dir = root.path().join("trash");
    let trash = DirTrash(trash_dir.clone());
    let mut w = SessionWriter::create(&root.path().join("out"), started(), meta()).unwrap();
    let p1 = w
        .save_capture(1, &pattern(8, 8, 1), 8, 8, 1.0, 5.0, CaptureReason::Settled)
        .unwrap();
    let p2 = w
        .save_capture(
            2,
            &pattern(8, 8, 2),
            8,
            8,
            9.0,
            13.0,
            CaptureReason::Settled,
        )
        .unwrap();

    assert_eq!(w.undo_last(14.0, &trash).unwrap(), Some(p2.clone()));
    assert!(!p2.exists());
    assert!(trash_dir.join(p2.file_name().unwrap()).exists());
    let s = read_session(w.dir());
    assert_eq!(s["captures"].as_array().unwrap().len(), 1);
    let last = s["events"].as_array().unwrap().last().unwrap().clone();
    assert_eq!(last["type"], "undo");
    assert_eq!(last["t"], 14.0);
    assert_schema_valid(&s);

    assert_eq!(w.undo_last(15.0, &trash).unwrap(), Some(p1.clone()));
    assert_eq!(w.undo_last(16.0, &trash).unwrap(), None);
    assert_eq!(w.capture_count(), 0);
}

/// B9: rebuilding the PDF uses only the images still in the folder, in index order.
#[test]
fn b9_rebuild_pdf_from_remaining_images() {
    let root = tempfile::tempdir().unwrap();
    let mut w = SessionWriter::create(root.path(), started(), meta()).unwrap();
    let mut paths = Vec::new();
    for i in 1..=5u32 {
        let (sw, sh) = (i * 10, 10);
        paths.push(
            w.save_capture(
                i as usize,
                &pattern(sw, sh, i),
                sw,
                sh,
                i as f64 * 10.0,
                i as f64 * 10.0 + 4.0,
                CaptureReason::Settled,
            )
            .unwrap(),
        );
    }
    w.finish(Local::now()).unwrap();
    let dir = w.dir().to_path_buf();
    fs::remove_file(&paths[1]).unwrap();
    fs::remove_file(&paths[3]).unwrap();

    assert_eq!(rebuild_pdf(&dir, Local::now()).unwrap(), 3);
    assert_eq!(
        pdf_page_sizes(&dir.join(PDF_FILE)),
        vec![(10.0, 10.0), (30.0, 10.0), (50.0, 10.0)]
    );
    let s = read_session(&dir);
    assert_eq!(
        s["captures"].as_array().unwrap().len(),
        5,
        "captures are left unchanged"
    );
    assert_eq!(s["pdf"]["pages"], 3);
    assert!(s["pdf"]["rebuilt_at"].is_string());
    assert_schema_valid(&s);

    for p in [&paths[0], &paths[2], &paths[4]] {
        fs::remove_file(p).unwrap();
    }
    let before = fs::read(dir.join(PDF_FILE)).unwrap();
    let err = rebuild_pdf(&dir, Local::now()).unwrap_err();
    assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    assert_eq!(
        fs::read(dir.join(PDF_FILE)).unwrap(),
        before,
        "nothing is written when no images are left"
    );
}

/// A session without captures writes no PDF.
#[test]
fn finish_without_captures_writes_no_pdf() {
    let root = tempfile::tempdir().unwrap();
    let mut w = SessionWriter::create(root.path(), started(), meta()).unwrap();
    assert_eq!(w.finish(Local::now()).unwrap(), None);
    assert!(!w.dir().join(PDF_FILE).exists());
    let s = read_session(w.dir());
    assert!(s["pdf"].is_null());
    assert!(s["ended_at"].is_string());
    assert_schema_valid(&s);
}
