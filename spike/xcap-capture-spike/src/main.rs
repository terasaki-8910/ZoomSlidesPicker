use std::{
    env, fs,
    num::ParseIntError,
    path::PathBuf,
    thread,
    time::{Duration, Instant},
};

use xcap::{image::RgbaImage, Monitor, Window};

// macOS only: the CoreGraphics framework does not exist on Windows.
#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGPreflightScreenCaptureAccess() -> bool;
}

#[cfg(target_os = "macos")]
fn screen_capture_granted() -> bool {
    unsafe { CGPreflightScreenCaptureAccess() }
}

// Windows has no screen-recording permission prompt; report true so the output line stays uniform.
#[cfg(not(target_os = "macos"))]
fn screen_capture_granted() -> bool {
    true
}

const SAVE_LIMIT: usize = 40;
const CHANGE_MAD: f64 = 1.0;

// Luma sampled on a grid (every `step` px). Cheap enough to run at 2 Hz.
fn luma_grid(img: &RgbaImage, step: u32) -> Vec<u8> {
    let (w, h) = img.dimensions();
    let mut out = Vec::new();
    for y in (0..h).step_by(step as usize) {
        for x in (0..w).step_by(step as usize) {
            let p = img.get_pixel(x, y).0;
            out.push(((p[0] as u32 * 3 + p[1] as u32 * 6 + p[2] as u32) / 10) as u8);
        }
    }
    out
}

fn mad(a: &[u8], b: &[u8]) -> f64 {
    let total: u64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| (*x as i64 - *y as i64).unsigned_abs())
        .sum();
    total as f64 / a.len().max(1) as f64
}

fn mean(a: &[u8]) -> f64 {
    a.iter().map(|&v| v as f64).sum::<f64>() / a.len().max(1) as f64
}

fn sample_loop(
    label: &str,
    seconds: u64,
    interval: Duration,
    out_dir: &PathBuf,
    mut grab: impl FnMut() -> Result<RgbaImage, String>,
) -> Result<(), String> {
    fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let started = Instant::now();
    let mut prev: Option<Vec<u8>> = None;
    let mut times_ms: Vec<f64> = Vec::new();
    let mut changed = 0usize;
    let mut saved = 0usize;
    println!("t_s,capture_ms,mean_luma,mad_vs_prev,size");
    while started.elapsed() < Duration::from_secs(seconds) {
        let tick = Instant::now();
        let img = grab()?;
        let cap_ms = tick.elapsed().as_secs_f64() * 1000.0;
        let grid = luma_grid(&img, 8);
        let d = prev.as_ref().map(|p| mad(p, &grid));
        let is_change = d.is_some_and(|v| v > CHANGE_MAD);
        if is_change {
            changed += 1;
        }
        if (prev.is_none() || is_change) && saved < SAVE_LIMIT {
            img.save(out_dir.join(format!("{label}_{saved:04}.png")))
                .map_err(|e| e.to_string())?;
            saved += 1;
        }
        println!(
            "{:.2},{:.1},{:.1},{},{}x{}",
            started.elapsed().as_secs_f64(),
            cap_ms,
            mean(&grid),
            d.map_or("-".to_string(), |v| format!("{v:.2}")),
            img.width(),
            img.height()
        );
        times_ms.push(cap_ms);
        prev = Some(grid);
        let spent = tick.elapsed();
        if spent < interval {
            thread::sleep(interval - spent);
        }
    }
    if times_ms.is_empty() {
        return Err("no frames".into());
    }
    let mut sorted = times_ms.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pct = |q: f64| sorted[((sorted.len() as f64 - 1.0) * q).round() as usize];
    let avg = times_ms.iter().sum::<f64>() / times_ms.len() as f64;
    eprintln!(
        "summary[{label}]: frames={} changed={} saved={} capture_ms avg={avg:.1} p50={:.1} p95={:.1} max={:.1}",
        times_ms.len(),
        changed,
        saved,
        pct(0.5),
        pct(0.95),
        pct(1.0)
    );
    Ok(())
}

fn list() {
    let granted = screen_capture_granted();
    println!("CGPreflightScreenCaptureAccess={granted}");
    for m in Monitor::all().unwrap_or_default() {
        println!(
            "monitor name={:?} x={:?} y={:?} w={:?} h={:?} scale={:?} primary={:?}",
            m.name().ok(),
            m.x().ok(),
            m.y().ok(),
            m.width().ok(),
            m.height().ok(),
            m.scale_factor().ok(),
            m.is_primary().ok()
        );
    }
    for w in Window::all().unwrap_or_default() {
        let title = w.title().unwrap_or_default();
        if title.is_empty() {
            continue;
        }
        println!(
            "window app={:?} title={:?} x={:?} y={:?} w={:?} h={:?} minimized={:?}",
            w.app_name().ok(),
            title,
            w.x().ok(),
            w.y().ok(),
            w.width().ok(),
            w.height().ok(),
            w.is_minimized().ok()
        );
    }
}

fn arg_u32(a: &[String], i: usize) -> Result<u32, String> {
    a.get(i)
        .ok_or_else(|| format!("missing arg {i}"))?
        .parse()
        .map_err(|e: ParseIntError| e.to_string())
}

// region X Y W H SECONDS INTERVAL_MS OUT_DIR   (X,Y in points, relative to primary monitor)
fn region(a: &[String]) -> Result<(), String> {
    let (x, y, w, h) = (arg_u32(a, 2)?, arg_u32(a, 3)?, arg_u32(a, 4)?, arg_u32(a, 5)?);
    let secs = arg_u32(a, 6)? as u64;
    let interval = Duration::from_millis(arg_u32(a, 7)? as u64);
    let out = PathBuf::from(a.get(8).ok_or("missing out dir")?);
    let monitor = Monitor::all()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|m| m.is_primary().unwrap_or(false))
        .ok_or("no primary monitor")?;
    sample_loop("region", secs, interval, &out, || {
        monitor.capture_region(x, y, w, h).map_err(|e| e.to_string())
    })
}

// window TITLE_SUBSTRING SECONDS INTERVAL_MS OUT_DIR
fn window(a: &[String]) -> Result<(), String> {
    let needle = a.get(2).ok_or("missing title")?.clone();
    let secs = arg_u32(a, 3)? as u64;
    let interval = Duration::from_millis(arg_u32(a, 4)? as u64);
    let out = PathBuf::from(a.get(5).ok_or("missing out dir")?);
    let win = Window::all()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|w| {
            w.title().is_ok_and(|t| t.contains(&needle)) && !w.is_minimized().unwrap_or(true)
        })
        .ok_or("window not found (title substring, not minimized)")?;
    eprintln!(
        "window: app={:?} x={:?} y={:?} w={:?} h={:?}",
        win.app_name().ok(),
        win.x().ok(),
        win.y().ok(),
        win.width().ok(),
        win.height().ok()
    );
    sample_loop("window", secs, interval, &out, || {
        win.capture_image().map_err(|e| e.to_string())
    })
}

fn main() {
    let a: Vec<String> = env::args().collect();
    let result = match a.get(1).map(String::as_str) {
        Some("list") | None => {
            list();
            Ok(())
        }
        Some("region") => region(&a),
        Some("window") => window(&a),
        Some(other) => Err(format!("unknown command: {other}")),
    };
    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
