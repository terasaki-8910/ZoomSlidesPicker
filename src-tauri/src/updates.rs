//! In-app updates (docs/impl/updates.md): the launch check, the manual check, the
//! answers (later / skip), and the install, plus the guards that keep an install from
//! colliding with a session or a PDF job.
//!
//! The webview has no updater permission (capabilities stay `core:default`); Rust drives
//! `tauri-plugin-updater` and restarts the app itself.
//!
//! Lock order: `Shell` first, then `Gate` (the start commands check "installing" while
//! they hold the shell lock, and `install_update` checks "no session" the same way, so
//! neither can slip past the other). `Gate` never takes the shell lock.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::i18n;
use crate::ipc_types::{Lang, UpdateAnswer, UpdateCheck, UpdateInfo, UpdatePhase, UpdateProgress};
use crate::settings;
use crate::state::SharedShell;

const UPDATE_FILE: &str = "update.json";

/// `update-progress` goes out at most this often while downloading.
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);

/// Test hook variables that keep the launch check from running: it would otherwise
/// reach out to GitHub in every automated run (docs/impl/updates.md > Rust).
const NO_CHECK_VARS: [&str; 3] = [
    "ZSP_AUTOSTART_TITLE",
    "ZSP_OPEN_WINDOW",
    "ZSP_NO_UPDATE_CHECK",
];

// ---------------------------------------------------------------------------------------
// The skipped version (update.json)
// ---------------------------------------------------------------------------------------

/// update.json's shape. Separate from settings.json, whose `Settings` part the webview
/// writes back whole.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SkippedFile {
    skipped_version: String,
}

fn update_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(UPDATE_FILE))
}

/// The skipped version in `path`; none when the file is missing, unreadable, corrupt, or
/// holds an empty version (never panics).
fn read_skipped(path: &Path) -> Option<String> {
    let file: SkippedFile = fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())?;
    let version = file.skipped_version.trim();
    (!version.is_empty()).then(|| version.to_string())
}

/// Records `version` as the skipped one (one version at a time: this replaces the old).
fn write_skipped(path: &Path, version: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(&SkippedFile {
        skipped_version: version.to_string(),
    })
    .map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

fn normalize(version: &str) -> &str {
    let v = version.trim();
    v.strip_prefix('v').unwrap_or(v)
}

fn same_version(a: &str, b: &str) -> bool {
    normalize(a) == normalize(b)
}

/// The offer for a found `version`, or none when it is the skipped one. A newer version
/// than the skipped one is offered as usual (the updater only reports versions newer than
/// the running one, so "not the skipped one" is all that is left to decide).
fn offer_for(version: &str, current: &str, skipped: Option<&str>) -> Option<UpdateInfo> {
    if skipped.is_some_and(|s| same_version(s, version)) {
        return None;
    }
    Some(UpdateInfo {
        version: version.to_string(),
        current: current.to_string(),
    })
}

/// Whether the launch check runs: never in debug builds, and not when a test hook
/// variable is set. `is_set` answers "is this environment variable set".
fn launch_check_enabled(debug_build: bool, is_set: impl Fn(&str) -> bool) -> bool {
    !debug_build && !NO_CHECK_VARS.iter().any(|v| is_set(v))
}

// ---------------------------------------------------------------------------------------
// The install gate: "installing" flag + running PDF jobs
// ---------------------------------------------------------------------------------------

#[derive(Default)]
struct GateState {
    installing: bool,
    /// PDF jobs running now (`export_pdf`, `rebuild_pdf`, `trash_captures`,
    /// `trash_session`).
    jobs: usize,
}

/// Why `begin_install` said no.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    /// A session is active (recording, paused, stopping, or a video being processed).
    Busy,
    /// An install is already running.
    InProgress,
}

impl Refusal {
    fn message(self, lang: Lang) -> &'static str {
        match self {
            Refusal::Busy => i18n::update_busy(lang),
            Refusal::InProgress => i18n::update_in_progress(lang),
        }
    }
}

#[derive(Default)]
struct Gate {
    state: Mutex<GateState>,
    idle: Condvar,
}

impl Gate {
    fn installing(&self) -> bool {
        self.state.lock().unwrap().installing
    }

    /// Registers a running PDF job; false while installing (the job must not start).
    fn try_begin_job(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.installing {
            return false;
        }
        state.jobs += 1;
        true
    }

    fn end_job(&self) {
        let mut state = self.state.lock().unwrap();
        state.jobs = state.jobs.saturating_sub(1);
        if state.jobs == 0 {
            self.idle.notify_all();
        }
    }

    /// Sets "installing", unless a session is active or an install is already running.
    fn begin_install(&self, session_active: bool) -> Result<(), Refusal> {
        if session_active {
            return Err(Refusal::Busy);
        }
        let mut state = self.state.lock().unwrap();
        if state.installing {
            return Err(Refusal::InProgress);
        }
        state.installing = true;
        Ok(())
    }

    fn end_install(&self) {
        self.state.lock().unwrap().installing = false;
    }

    /// Blocks until no PDF job is running. Call from a blocking thread.
    fn wait_jobs(&self) {
        let mut state = self.state.lock().unwrap();
        while state.jobs > 0 {
            state = self.idle.wait(state).unwrap();
        }
    }
}

/// Lets `update-progress` through at most once per interval.
struct Throttle {
    interval: Duration,
    last: Option<Instant>,
}

impl Throttle {
    fn new(interval: Duration) -> Self {
        Self {
            interval,
            last: None,
        }
    }

    fn due(&mut self, now: Instant) -> bool {
        match self.last {
            Some(last) if now.duration_since(last) < self.interval => false,
            _ => {
                self.last = Some(now);
                true
            }
        }
    }
}

// ---------------------------------------------------------------------------------------
// Managed state
// ---------------------------------------------------------------------------------------

#[derive(Default)]
pub struct Updates {
    /// The update the launch check or the latest manual check found, kept for
    /// `install_update`.
    found: Mutex<Option<Update>>,
    /// The launch check's find while unanswered this run.
    offer: Mutex<Option<UpdateInfo>>,
    gate: Gate,
}

fn lang_of<R: Runtime>(app: &AppHandle<R>) -> Lang {
    let language = app.state::<SharedShell>().lock().unwrap().settings.language;
    settings::effective_language(language)
}

fn info_of(update: &Update) -> UpdateInfo {
    UpdateInfo {
        version: update.version.clone(),
        current: update.current_version.clone(),
    }
}

// ---------------------------------------------------------------------------------------
// Guards used by the other modules
// ---------------------------------------------------------------------------------------

/// Refuses with the localized `update_in_progress` message while an update installs.
/// Safe to call while holding the shell lock (lock order, see the module docs).
pub fn ensure_not_installing<R: Runtime>(app: &AppHandle<R>, lang: Lang) -> Result<(), String> {
    if app.state::<Updates>().gate.installing() {
        Err(i18n::update_in_progress(lang).to_string())
    } else {
        Ok(())
    }
}

/// A running PDF job: `install_update` waits for these before it installs.
pub struct PdfJob<R: Runtime>(AppHandle<R>);

impl<R: Runtime> Drop for PdfJob<R> {
    fn drop(&mut self) {
        self.0.state::<Updates>().gate.end_job();
    }
}

/// Registers a PDF job, or refuses with `update_in_progress` while an update installs.
/// Hold the result for as long as the job runs. Never call with the shell lock held.
pub fn begin_pdf_job<R: Runtime>(app: &AppHandle<R>, lang: Lang) -> Result<PdfJob<R>, String> {
    if app.state::<Updates>().gate.try_begin_job() {
        Ok(PdfJob(app.clone()))
    } else {
        Err(i18n::update_in_progress(lang).to_string())
    }
}

// ---------------------------------------------------------------------------------------
// Launch check
// ---------------------------------------------------------------------------------------

/// Spawned from `setup` after the library window is opened. Silent on failure (stderr
/// only): the user may simply be offline.
pub fn spawn_launch_check<R: Runtime>(app: AppHandle<R>) {
    if !launch_check_enabled(cfg!(debug_assertions), |v| env::var_os(v).is_some()) {
        return;
    }
    tauri::async_runtime::spawn(async move {
        if let Err(e) = launch_check(&app).await {
            eprintln!("update check failed: {e}");
        }
    });
}

async fn launch_check<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let found = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let Some(update) = found else {
        return Ok(());
    };
    let skipped = update_path(app).and_then(|p| read_skipped(&p));
    let Some(info) = offer_for(&update.version, &update.current_version, skipped.as_deref()) else {
        return Ok(()); // the skipped version: dropped
    };
    let updates = app.state::<Updates>();
    *updates.found.lock().unwrap() = Some(update);
    *updates.offer.lock().unwrap() = Some(info.clone());
    let _ = app.emit("update-offer", &info);
    Ok(())
}

// ---------------------------------------------------------------------------------------
// Commands (docs/impl/updates.md > Commands)
// ---------------------------------------------------------------------------------------

#[tauri::command]
pub fn get_app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

#[tauri::command]
pub fn get_update_offer(app: AppHandle) -> Option<UpdateInfo> {
    app.state::<Updates>().offer.lock().unwrap().clone()
}

/// Manual check. Ignores the skipped version (asking is the way back to a skipped
/// update); `skipped` says whether `available` is the skipped one.
#[tauri::command]
pub async fn check_update(app: AppHandle) -> Result<UpdateCheck, String> {
    ensure_not_installing(&app, lang_of(&app))?;
    let current = app.package_info().version.to_string();
    let found = app
        .updater()
        .map_err(|e| e.to_string())?
        .check()
        .await
        .map_err(|e| e.to_string())?;
    let updates = app.state::<Updates>();
    let Some(update) = found else {
        *updates.found.lock().unwrap() = None;
        return Ok(UpdateCheck {
            current,
            available: None,
            skipped: false,
        });
    };
    let skipped = update_path(&app)
        .and_then(|p| read_skipped(&p))
        .is_some_and(|s| same_version(&s, &update.version));
    let available = info_of(&update);
    *updates.found.lock().unwrap() = Some(update);
    Ok(UpdateCheck {
        current,
        available: Some(available),
        skipped,
    })
}

#[tauri::command]
pub fn answer_update(app: AppHandle, answer: UpdateAnswer) -> Result<(), String> {
    let lang = lang_of(&app);
    let updates = app.state::<Updates>();
    if answer == UpdateAnswer::Skip {
        let version = updates
            .found
            .lock()
            .unwrap()
            .as_ref()
            .map(|u| u.version.clone())
            .or_else(|| {
                updates
                    .offer
                    .lock()
                    .unwrap()
                    .as_ref()
                    .map(|o| o.version.clone())
            })
            .ok_or_else(|| i18n::update_nothing_found(lang).to_string())?;
        let path = update_path(&app).ok_or("no app config directory")?;
        write_skipped(&path, &version)?;
    }
    // Both answers: this run does not offer it again.
    *updates.offer.lock().unwrap() = None;
    Ok(())
}

/// Downloads, installs, and restarts. Refuses while a session is active (the UI disables
/// the button then; this is the guard). On macOS it never returns on success; on Windows
/// the installer takes over and the process exits inside `Update::install`.
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), String> {
    let lang = lang_of(&app);
    let update = app
        .state::<Updates>()
        .found
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| i18n::update_nothing_found(lang).to_string())?;

    {
        // Under the shell lock, so a start command cannot slip in between the session
        // check and the flag (the start commands check the flag under the same lock).
        let shell = app.state::<SharedShell>();
        let shell = shell.lock().unwrap();
        app.state::<Updates>()
            .gate
            .begin_install(shell.session.is_some())
            .map_err(|r| r.message(lang).to_string())?;
    }

    match run_install(&app, &update).await {
        Ok(()) => {
            // Reached on macOS only (Windows exited inside `install`).
            //
            // tauri 2.12.1 `AppHandle::restart` (app.rs:606): called off the main thread
            // (this is an async-runtime worker), it requests an exit with
            // `RESTART_EXIT_CODE` and sleeps; `RunEvent::Exit` then relaunches the
            // process. `ExitRequestApi::prevent_exit` is a no-op for that code (app.rs:90),
            // so lib.rs's `ExitRequested` handler cannot block the restart. No session can
            // be left to stop there: starting one is refused while installing.
            app.restart()
        }
        Err(e) => {
            app.state::<Updates>().gate.end_install();
            Err(e)
        }
    }
}

async fn run_install<R: Runtime>(app: &AppHandle<R>, update: &Update) -> Result<(), String> {
    // (downloaded, total) so far; the finish event reads it after `download` has
    // consumed the chunk callback.
    let seen = Mutex::new((0u64, None::<u64>));
    let throttle = Mutex::new(Throttle::new(PROGRESS_INTERVAL));
    let emit = |downloaded: u64, total: Option<u64>| {
        let _ = app.emit(
            "update-progress",
            UpdateProgress {
                phase: UpdatePhase::Downloading,
                downloaded,
                total,
            },
        );
    };

    let bytes = update
        .download(
            |chunk, total| {
                let (downloaded, total) = {
                    let mut seen = seen.lock().unwrap();
                    seen.0 += chunk as u64;
                    seen.1 = total;
                    *seen
                };
                if throttle.lock().unwrap().due(Instant::now()) {
                    emit(downloaded, total);
                }
            },
            || {},
        )
        .await
        .map_err(|e| e.to_string())?;

    // The last chunk may have been throttled away: send the final figure.
    let (downloaded, total) = *seen.lock().unwrap();
    emit(downloaded, total);
    let _ = app.emit(
        "update-progress",
        UpdateProgress {
            phase: UpdatePhase::Installing,
            downloaded,
            total,
        },
    );

    // PDF jobs already running finish first: the install must not cut one off. New ones
    // are refused, since "installing" is set.
    let waiter = app.clone();
    tauri::async_runtime::spawn_blocking(move || waiter.state::<Updates>().gate.wait_jobs())
        .await
        .map_err(|e| e.to_string())?;

    // Windows: `Update::install` runs the NSIS installer (ShellExecuteW) and then calls
    // `std::process::exit(0)` (tauri-plugin-updater 2.13.3 updater.rs:979), skipping
    // tauri's `RunEvent::Exit`. The installer restarts the app (`restart_after_install`
    // defaults to true). Safe for sessions: none can be active (guarded in
    // `install_update`, and starting one is refused while installing).
    // macOS: replaces the .app bundle and returns; the caller restarts.
    let update = update.clone();
    tauri::async_runtime::spawn_blocking(move || update.install(bytes))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------------------
// Tests (ACCEPTANCE B17)
// ---------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skipped_version_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nested").join(UPDATE_FILE);
        write_skipped(&path, "0.1.8").unwrap();
        assert_eq!(read_skipped(&path).as_deref(), Some("0.1.8"));
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.contains("\"skippedVersion\""), "{text}");
        // One version at a time: a second skip replaces the first.
        write_skipped(&path, "0.2.0").unwrap();
        assert_eq!(read_skipped(&path).as_deref(), Some("0.2.0"));
    }

    #[test]
    fn missing_or_corrupt_file_means_no_skip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(UPDATE_FILE);
        assert_eq!(read_skipped(&path), None);
        for bad in [
            "",
            "not json",
            "{}",
            "{\"skippedVersion\": 3}",
            "{\"skippedVersion\": \"  \"}",
        ] {
            fs::write(&path, bad).unwrap();
            assert_eq!(read_skipped(&path), None, "{bad:?}");
        }
    }

    #[test]
    fn launch_drops_the_skipped_version_and_offers_a_newer_one() {
        assert_eq!(offer_for("0.1.8", "0.1.7", Some("0.1.8")), None);
        assert_eq!(offer_for("v0.1.8", "0.1.7", Some("0.1.8")), None);
        let newer = offer_for("0.1.9", "0.1.7", Some("0.1.8")).unwrap();
        assert_eq!(newer.version, "0.1.9");
        assert_eq!(newer.current, "0.1.7");
        assert!(offer_for("0.1.8", "0.1.7", None).is_some());
    }

    #[test]
    fn launch_check_is_off_in_debug_builds_and_under_test_hooks() {
        assert!(launch_check_enabled(false, |_| false));
        assert!(!launch_check_enabled(true, |_| false));
        for var in NO_CHECK_VARS {
            assert!(!launch_check_enabled(false, |v| v == var), "{var}");
        }
    }

    #[test]
    fn install_is_refused_while_a_session_is_active() {
        let gate = Gate::default();
        assert_eq!(gate.begin_install(true), Err(Refusal::Busy));
        assert!(!gate.installing());
        assert_eq!(gate.begin_install(false), Ok(()));
        assert!(gate.installing());
        assert_eq!(gate.begin_install(false), Err(Refusal::InProgress));
        gate.end_install();
        assert!(!gate.installing());
        assert_eq!(gate.begin_install(false), Ok(()));
    }

    #[test]
    fn refusals_are_localized() {
        assert_eq!(
            Refusal::Busy.message(Lang::En),
            "You can't update while recording. Stop to update."
        );
        assert_eq!(
            Refusal::InProgress.message(Lang::Ja),
            "更新しています。終わるとアプリが再起動します。"
        );
    }

    #[test]
    fn jobs_are_refused_while_installing_and_install_waits_for_running_ones() {
        let gate = std::sync::Arc::new(Gate::default());
        assert!(gate.try_begin_job());
        gate.begin_install(false).unwrap();
        assert!(!gate.try_begin_job(), "start refused while installing");

        let waiter = {
            let gate = gate.clone();
            std::thread::spawn(move || gate.wait_jobs())
        };
        std::thread::sleep(Duration::from_millis(50));
        assert!(!waiter.is_finished(), "waits for the running job");
        gate.end_job();
        waiter.join().unwrap();

        gate.end_install();
        assert!(gate.try_begin_job());
    }

    #[test]
    fn progress_is_throttled() {
        let mut t = Throttle::new(Duration::from_millis(100));
        let t0 = Instant::now();
        assert!(t.due(t0));
        assert!(!t.due(t0 + Duration::from_millis(40)));
        assert!(!t.due(t0 + Duration::from_millis(99)));
        assert!(t.due(t0 + Duration::from_millis(100)));
        assert!(!t.due(t0 + Duration::from_millis(150)));
    }

    #[test]
    fn wire_format_is_camel_case() {
        let progress = UpdateProgress {
            phase: UpdatePhase::Downloading,
            downloaded: 5,
            total: None,
        };
        assert_eq!(
            serde_json::to_value(progress).unwrap(),
            serde_json::json!({"phase": "downloading", "downloaded": 5, "total": null})
        );
        let check = UpdateCheck {
            current: "0.1.7".into(),
            available: None,
            skipped: false,
        };
        assert_eq!(
            serde_json::to_value(check).unwrap(),
            serde_json::json!({"current": "0.1.7", "available": null, "skipped": false})
        );
        let answer: UpdateAnswer = serde_json::from_str("\"skip\"").unwrap();
        assert_eq!(answer, UpdateAnswer::Skip);
    }
}
