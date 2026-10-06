//! ACCEPTANCE A11: analysis plus one detector step on a 2000x1400 frame takes at most 20 ms
//! (median), in a release build on the development machine.
//! Run: cargo test --release -p zsp-detect --test timing

use std::time::{Duration, Instant};

use zsp_detect::{Config, Detector, analyze};

#[test]
#[cfg_attr(
    debug_assertions,
    ignore = "timing is only meaningful in a release build"
)]
fn a11_analyze_and_step_within_20ms() {
    let (w, h) = (2000u32, 1400u32);
    let mut rgba = vec![255u8; (w * h * 4) as usize];
    // Dark text-like bars and a busy "video" rectangle.
    for (i, px) in rgba.chunks_exact_mut(4).enumerate() {
        let (x, y) = ((i as u32) % w, (i as u32) / w);
        let text = (y / 40) % 3 == 0 && (x / 7) % 5 != 0 && x < 1200;
        let video = x > 1300 && y > 800;
        if text {
            px[..3].copy_from_slice(&[30, 30, 30]);
        } else if video {
            let v = ((x * 31 + y * 17) % 251) as u8;
            px[..3].copy_from_slice(&[v, v / 2, 255 - v]);
        }
    }

    let mut det = Detector::new(Config::default());
    let mut times: Vec<Duration> = Vec::new();
    for i in 0..31 {
        // Change the video area a little each sample.
        let k = (i * 977) % ((w * h) as usize);
        rgba[k * 4] = rgba[k * 4].wrapping_add(97);
        let t0 = Instant::now();
        let (g, d) = analyze(&rgba, w, h);
        let _ = det.step(i as f64 * 0.5, &g, &d);
        times.push(t0.elapsed());
    }
    times.sort();
    let median = times[times.len() / 2];
    assert!(
        median <= Duration::from_millis(20),
        "median {median:?} > 20 ms"
    );
}
