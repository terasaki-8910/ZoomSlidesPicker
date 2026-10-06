//! Test hook for ACCEPTANCE D1/D3 only (docs/impl/shell.md > Test hook). A no-op unless
//! `ZSP_AUTOSTART_TITLE` is set.

use std::env;
use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::ipc_types::RegionSel;
use crate::state::SharedShell;

const RETRY_ATTEMPTS: u32 = 10;
const RETRY_DELAY: Duration = Duration::from_millis(300);

/// Runs once at startup, from `setup()`.
pub fn run(app: &AppHandle) {
    let Ok(title_substr) = env::var("ZSP_AUTOSTART_TITLE") else {
        return;
    };

    if let Ok(root) = env::var("ZSP_SAVE_ROOT") {
        app.state::<SharedShell>()
            .lock()
            .unwrap()
            .settings
            .save_root = root;
    }

    let Some(window_id) = find_window(&title_substr) else {
        eprintln!(
            "ZSP_AUTOSTART_TITLE={title_substr:?}: no matching window found after {RETRY_ATTEMPTS} attempts"
        );
        return;
    };

    // Whole window, per docs/impl/shell.md > Test hook.
    let full = RegionSel {
        x: 0.0,
        y: 0.0,
        w: 1.0,
        h: 1.0,
    };
    if let Err(e) = crate::session_start::start(app, window_id, full) {
        eprintln!("ZSP_AUTOSTART_TITLE: failed to start a session: {e}");
        return;
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
