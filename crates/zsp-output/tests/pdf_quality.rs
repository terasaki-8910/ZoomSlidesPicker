//! ACCEPTANCE B14-B16: PDF quality, estimates, and exporting a chosen set.

use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Local, TimeZone};
use serde_json::Value;
use zsp_output::{
    CaptureReason, PdfQuality, SESSION_FILE, SessionMeta, SessionWriter, Source, SourceKind,
    estimate_pdf_bytes, export_pdf, rebuild_pdf, write_pdf,
};

fn started() -> DateTime<Local> {
    Local
        .with_ymd_and_hms(2026, 10, 13, 15, 13, 0)
        .single()
        .unwrap()
}

fn meta() -> SessionMeta {
    SessionMeta {
        app_version: "0.1.0".into(),
        source: Source {
            kind: SourceKind::Window,
            app: None,
            title: None,
            platform: "macos".into(),
            file: None,
        },
        region: None,
        interval_s: 0.5,
    }
}

/// White slide with a gradient band, black text-like bars and coloured blocks (hard edges).
fn slide(w: u32, h: u32, seed: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity((w * h * 4) as usize);
    for y in 0..h {
        for x in 0..w {
            let (mut r, mut g, mut b) = (255u8, 255u8, 255u8);
            if y < h / 8 {
                r = (x * 255 / w) as u8;
                g = 90;
                b = 160;
            } else if (y / 6 + seed).is_multiple_of(2)
                && x > w / 10
                && x < w * 6 / 10
                && (x / 3) % 4 != 3
            {
                (r, g, b) = (10, 10, 10);
            } else if y > h / 2 && x < w / 2 {
                // A smooth photo-like area: lossless compresses it poorly, JPEG well.
                let t = |n: u32| {
                    let m = n % 512;
                    (if m < 256 { m } else { 511 - m }) as u8
                };
                let noise = (x.wrapping_mul(2654435761) ^ y.wrapping_mul(40503)) % 9;
                let n = |v: u8| v.saturating_add(noise as u8).saturating_sub(4);
                (r, g, b) = (
                    n(t(x + y + seed)),
                    n(t(x * 2 / 3 + 40)),
                    n(t(y * 3 + x / 2)),
                );
            } else if x > w * 7 / 10 && x < w * 9 / 10 && y > h / 3 && y < h * 2 / 3 {
                (r, g, b) = (200, 30 + (seed * 20 % 100) as u8, 40);
            }
            v.extend_from_slice(&[r, g, b, 255]);
        }
    }
    v
}

fn session(w: u32, h: u32, n: usize) -> (tempfile::TempDir, SessionWriter) {
    let root = tempfile::tempdir().unwrap();
    let mut wr = SessionWriter::create(root.path(), started(), meta()).unwrap();
    for i in 0..n {
        wr.save_capture(
            i + 1,
            &slide(w, h, i as u32),
            w,
            h,
            i as f64 * 10.0,
            i as f64 * 10.0 + 4.0,
            CaptureReason::Settled,
        )
        .unwrap();
    }
    (root, wr)
}

fn pngs(wr: &SessionWriter) -> Vec<PathBuf> {
    wr.session()
        .captures
        .iter()
        .map(|c| wr.dir().join(&c.file))
        .collect()
}

fn num(o: &lopdf::Object) -> f64 {
    match o {
        lopdf::Object::Integer(i) => *i as f64,
        lopdf::Object::Real(f) => *f as f64,
        other => panic!("not a number: {other:?}"),
    }
}

struct PageInfo {
    media: (f64, f64),
    image: (i64, i64),
    filter: Vec<u8>,
    stream: Vec<u8>,
}

fn pages(path: &Path) -> Vec<PageInfo> {
    let doc = lopdf::Document::load(path).unwrap();
    doc.get_pages()
        .values()
        .map(|id| {
            let page = doc.get_dictionary(*id).unwrap();
            let mb = page.get(b"MediaBox").unwrap().as_array().unwrap();
            let (resources, _) = doc.get_page_resources(*id).unwrap();
            let xo = resources
                .unwrap()
                .get(b"XObject")
                .unwrap()
                .as_dict()
                .unwrap();
            let (_, r) = xo.iter().next().unwrap();
            let st = doc
                .get_object(r.as_reference().unwrap())
                .unwrap()
                .as_stream()
                .unwrap();
            PageInfo {
                media: (num(&mb[2]) - num(&mb[0]), num(&mb[3]) - num(&mb[1])),
                image: (
                    st.dict.get(b"Width").unwrap().as_i64().unwrap(),
                    st.dict.get(b"Height").unwrap().as_i64().unwrap(),
                ),
                filter: st.dict.get(b"Filter").unwrap().as_name().unwrap().to_vec(),
                stream: st.content.clone(),
            }
        })
        .collect()
}

#[test]
fn b14_standard_is_smaller_than_lossless_and_pages_match_images() {
    let (_root, wr) = session(800, 450, 3);
    let imgs = pngs(&wr);
    let dir = wr.dir();
    let lossless = dir.join("a.pdf");
    let standard = dir.join("b.pdf");
    let light = dir.join("c.pdf");
    write_pdf(&imgs, &lossless, PdfQuality::Lossless).unwrap();
    write_pdf(&imgs, &standard, PdfQuality::Standard).unwrap();
    write_pdf(&imgs, &light, PdfQuality::Light).unwrap();
    assert!(fs::metadata(&standard).unwrap().len() < fs::metadata(&lossless).unwrap().len());

    for (path, filter) in [
        (&lossless, "FlateDecode"),
        (&standard, "DCTDecode"),
        (&light, "DCTDecode"),
    ] {
        let ps = pages(path);
        assert_eq!(ps.len(), 3);
        for p in ps {
            assert_eq!(p.media, (p.image.0 as f64, p.image.1 as f64));
            assert_eq!(p.filter, filter.as_bytes());
        }
    }
}

#[test]
fn b14_light_is_at_most_1920_wide_and_keeps_aspect() {
    let (_root, wr) = session(2560, 1440, 1);
    let out = wr.dir().join("l.pdf");
    write_pdf(&pngs(&wr), &out, PdfQuality::Light).unwrap();
    let p = &pages(&out)[0];
    assert_eq!(p.media, (1920.0, 1080.0));
    assert_eq!(p.image, (1920, 1080));

    // Narrower than the limit stays as it is.
    let (_root2, wr2) = session(640, 360, 1);
    let out2 = wr2.dir().join("l.pdf");
    write_pdf(&pngs(&wr2), &out2, PdfQuality::Light).unwrap();
    assert_eq!(pages(&out2)[0].media, (640.0, 360.0));
}

#[test]
fn b14_standard_jpeg_decodes_close_to_source() {
    let (_root, wr) = session(640, 360, 1);
    let out = wr.dir().join("s.pdf");
    write_pdf(&pngs(&wr), &out, PdfQuality::Standard).unwrap();
    let p = &pages(&out)[0];
    let decoded = image::load_from_memory_with_format(&p.stream, image::ImageFormat::Jpeg)
        .unwrap()
        .into_rgb8();
    let source = image::open(&pngs(&wr)[0]).unwrap().into_rgb8();
    assert_eq!(decoded.dimensions(), source.dimensions());
    let sum: u64 = decoded
        .as_raw()
        .iter()
        .zip(source.as_raw())
        .map(|(a, b)| a.abs_diff(*b) as u64)
        .sum();
    let mae = sum as f64 / source.as_raw().len() as f64;
    assert!(mae < 3.0, "mean abs error per channel {mae}");
}

#[test]
fn b15_estimate_is_within_30_percent_of_written_size() {
    let (_root, wr) = session(1280, 720, 12);
    let imgs = pngs(&wr);
    for q in [
        PdfQuality::Lossless,
        PdfQuality::Standard,
        PdfQuality::Light,
    ] {
        let out = wr.dir().join("e.pdf");
        write_pdf(&imgs, &out, q).unwrap();
        let actual = fs::metadata(&out).unwrap().len() as f64;
        let est = estimate_pdf_bytes(&imgs, q).unwrap() as f64;
        assert!(
            (est - actual).abs() / actual < 0.3,
            "{q:?}: est {est} actual {actual}"
        );
    }
    assert_eq!(estimate_pdf_bytes(&[], PdfQuality::Standard).unwrap(), 0);
}

#[test]
fn b16_export_selection_names_file_and_leaves_session_json() {
    let (_root, mut wr) = session(200, 100, 5);
    wr.finish(Local::now()).unwrap();
    let dir = wr.dir().to_path_buf();
    let before = fs::read_to_string(dir.join(SESSION_FILE)).unwrap();

    let ex = export_pdf(
        &dir,
        PdfQuality::Light,
        Some(&[4, 2, 99]),
        Local::now(),
        "Selected images",
    )
    .unwrap();
    assert_eq!(ex.path, dir.join("Selected images 2026-10-13.pdf"));
    assert_eq!(ex.pages, 2);
    assert_eq!(ex.bytes, fs::metadata(&ex.path).unwrap().len());
    assert_eq!(pages(&ex.path).len(), 2);
    assert_eq!(fs::read_to_string(dir.join(SESSION_FILE)).unwrap(), before);

    let ex2 = export_pdf(
        &dir,
        PdfQuality::Light,
        Some(&[1]),
        Local::now(),
        "Selected images",
    )
    .unwrap();
    assert_eq!(ex2.path, dir.join("Selected images 2026-10-13 2.pdf"));
    let ex3 = export_pdf(
        &dir,
        PdfQuality::Light,
        Some(&[1]),
        Local::now(),
        "Selected images",
    )
    .unwrap();
    assert_eq!(ex3.path, dir.join("Selected images 2026-10-13 3.pdf"));

    // Nothing selected exists: error, nothing written.
    let err = export_pdf(&dir, PdfQuality::Light, Some(&[77]), Local::now(), "x").unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
}

#[test]
fn b16_export_whole_session_rewrites_pdf_and_records_quality() {
    let (_root, mut wr) = session(200, 100, 3);
    wr.finish(Local::now()).unwrap();
    let dir = wr.dir().to_path_buf();
    let v: Value =
        serde_json::from_str(&fs::read_to_string(dir.join(SESSION_FILE)).unwrap()).unwrap();
    assert_eq!(v["pdf"]["quality"], "standard");

    let ex = export_pdf(
        &dir,
        PdfQuality::Lossless,
        None,
        Local::now(),
        "Selected images",
    )
    .unwrap();
    assert_eq!(ex.path, dir.join("slides.pdf"));
    assert_eq!(ex.pages, 3);
    let v: Value =
        serde_json::from_str(&fs::read_to_string(dir.join(SESSION_FILE)).unwrap()).unwrap();
    assert_eq!(v["pdf"]["quality"], "lossless");
    assert_eq!(v["pdf"]["pages"], 3);
    assert!(v["pdf"]["rebuilt_at"].is_string());
    assert_eq!(pages(&ex.path)[0].filter, b"FlateDecode");

    // rebuild_pdf keeps the recorded quality.
    rebuild_pdf(&dir, Local::now()).unwrap();
    assert_eq!(pages(&dir.join("slides.pdf"))[0].filter, b"FlateDecode");
}

#[test]
fn b14_old_session_json_without_quality_reads_as_lossless() {
    let (_root, mut wr) = session(64, 48, 1);
    wr.finish(Local::now()).unwrap();
    let dir = wr.dir();
    let mut v: Value =
        serde_json::from_str(&fs::read_to_string(dir.join(SESSION_FILE)).unwrap()).unwrap();
    v["pdf"].as_object_mut().unwrap().remove("quality");
    // Still schema-valid without the field.
    let schema: Value = serde_json::from_str(
        &fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/session.schema.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(jsonschema::validator_for(&schema).unwrap().is_valid(&v));
    fs::write(dir.join(SESSION_FILE), v.to_string()).unwrap();
    let read = zsp_output::read_session(dir).unwrap();
    assert_eq!(read.pdf.unwrap().quality, PdfQuality::Lossless);
}
