//! Test hooks for the real-machine checks: ACCEPTANCE D1/D3, and looking at the real
//! windows (docs/impl/shell.md > Test hook). No-ops unless the variables are set.
//!
//! - `ZSP_AUTOSTART_TITLE=<substr>`: starts a session on the first window whose title
//!   contains it (whole window, or `ZSP_AUTOSTART_REGION=x,y,w,h` as fractions of it).
//!   With `ZSP_OPEN_WINDOW=region`, opens the region window for that window instead.
//! - `ZSP_AUTOSTART_SECONDS=<n>`: quits after n seconds (stopping the session first).
//! - `ZSP_SAVE_ROOT=<dir>`: the save root for this run only (not written to the settings).
//! - `ZSP_OPEN_WINDOW=panel|settings|region`: opens that window at launch.

use std::env;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::ipc_types::RegionSel;
use crate::state::SharedShell;
use crate::windows;

const RETRY_ATTEMPTS: u32 = 10;
const RETRY_DELAY: Duration = Duration::from_millis(300);

/// Called once from `setup()`; does its work on another thread (finding the window may
/// take a few seconds of retries).
pub fn spawn(app: AppHandle) {
    let title = env::var("ZSP_AUTOSTART_TITLE").ok();
    let open = env::var("ZSP_OPEN_WINDOW").ok();
    if title.is_none() && open.is_none() {
        return;
    }
    thread::spawn(move || run(&app, title.as_deref(), open.as_deref()));
}

fn run(app: &AppHandle, title: Option<&str>, open: Option<&str>) {
    if let Ok(root) = env::var("ZSP_SAVE_ROOT") {
        app.state::<SharedShell>()
            .lock()
            .unwrap()
            .settings
            .save_root = root;
    }

    if let Some(title) = title {
        let Some(window_id) = find_window(title) else {
            eprintln!(
                "ZSP_AUTOSTART_TITLE={title:?}: no matching window found after {RETRY_ATTEMPTS} attempts"
            );
            return;
        };
        if open == Some("region") {
            if let Err(e) = crate::capture_cmds::begin_region(app, window_id) {
                eprintln!("ZSP_OPEN_WINDOW=region: {e}");
            }
        } else {
            // Whole window, per docs/impl/shell.md > Test hook, unless a region is given.
            let region = env::var("ZSP_AUTOSTART_REGION")
                .ok()
                .and_then(|s| parse_region(&s))
                .unwrap_or(RegionSel {
                    x: 0.0,
                    y: 0.0,
                    w: 1.0,
                    h: 1.0,
                });
            if let Err(e) = crate::session_start::start(app, window_id, region) {
                eprintln!("ZSP_AUTOSTART_TITLE: failed to start a session: {e}");
                return;
            }
        }
    }

    match open {
        Some("panel") => windows::open_panel(app),
        Some("settings") => windows::open_settings(app),
        _ => {}
    }

    let seconds = env::var("ZSP_AUTOSTART_SECONDS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok());
    if let Some(seconds) = seconds {
        let app = app.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(seconds));
            app.exit(0);
        });
    }
}

/// `x,y,w,h` as fractions of the window.
fn parse_region(s: &str) -> Option<RegionSel> {
    let v: Vec<f64> = s
        .split(',')
        .map(|p| p.trim().parse().ok())
        .collect::<Option<_>>()?;
    match v[..] {
        [x, y, w, h] => Some(RegionSel { x, y, w, h }),
        _ => None,
    }
}

/// The id of the first capturable window whose title contains `substr`. Retries
/// briefly (up to 3 s) since the target window may still be opening when this app
/// launches.
fn find_window(substr: &str) -> Option<u32> {
    for attempt in 0..RETRY_ATTEMPTS {
        if let Ok(windows) = zsp_capture::list_windows()
            && let Some(w) = windows.iter().find(|w| w.title.contains(substr))
        {
            return Some(w.id);
        }
        if attempt + 1 < RETRY_ATTEMPTS {
            thread::sleep(RETRY_DELAY);
        }
    }
    None
}
