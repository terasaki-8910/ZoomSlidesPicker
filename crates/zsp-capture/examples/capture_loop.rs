//! Captures one window repeatedly and prints each result (debugging aid).
//! `cargo run --release -p zsp-capture --example capture_loop -- <window id> <seconds>`
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let id: u32 = args[1].parse().unwrap();
    let secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(40);
    let cap = zsp_capture::WindowCapture::open(id).expect("open");
    println!("open ok: {:?}", cap.info());
    let start = Instant::now();
    let mut n = 0;
    while start.elapsed() < Duration::from_secs(secs) {
        let t0 = Instant::now();
        match cap.capture() {
            Ok(img) => {
                if n % 20 == 0 {
                    println!("{:6.1}s ok {}x{} in {} ms", start.elapsed().as_secs_f64(), img.width(), img.height(), t0.elapsed().as_millis());
                }
            }
            Err(e) => println!("{:6.1}s ERR {e} (after {} ms)", start.elapsed().as_secs_f64(), t0.elapsed().as_millis()),
        }
        n += 1;
        std::thread::sleep(Duration::from_millis(500));
    }
}
