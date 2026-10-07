//! `extract_video` (docs/impl/ipc.md; docs/impl/shell.md > Behaviour): processes an mp4
//! file like a session (full frame, video time), at the settings interval.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_dialog::DialogExt;
use zsp_output::{OsTrash, SessionMeta, SessionWriter, Source, SourceKind as OutKind};
use zsp_session::{Session, VideoSource};

use crate::ipc_types::{Lang, SourceKind};
use crate::state::{self, ActiveSession, SharedShell};
use crate::{i18n, notify, settings, ui};

/// `ffprobe -v error -show_entries format=duration -of csv=p=0 <file>`: the container
/// duration in seconds, when ffprobe can report one (docs/impl/shell.md: "when
/// available" -- any failure here just leaves `VideoProgress.total` null).
fn probe_duration_s(path: &Path) -> Option<f64> {
    let output = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "csv=p=0",
        ])
        .arg(path)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    String::from_utf8_lossy(&output.stdout).trim().parse().ok()
}

fn current_lang<R: Runtime>(app: &AppHandle<R>) -> Lang {
    let shell = app.state::<SharedShell>();
    let language = shell.lock().unwrap().settings.language;
    settings::effective_language(language)
}

/// `extract_video`: native file dialog, then starts a session on the chosen video
/// (docs/impl/ipc.md). Calls the dialog's blocking API, so every caller must be off the
/// main thread (the `async` command in `commands.rs`; `extract_video_from_menu` below).
pub fn extract_video<R: Runtime>(app: &AppHandle<R>) -> Result<(), String> {
    let lang = current_lang(app);
    if !zsp_capture::mp4::ffmpeg_available() {
        return Err(i18n::error_ffmpeg_missing(lang).to_string());
    }

    let picked = app
        .dialog()
        .file()
        .add_filter("Video", &["mp4", "mov", "m4v"])
        .blocking_pick_file();
    let Some(file_path) = picked else {
        return Ok(()); // the user closed the dialog without choosing a file
    };
    let path = file_path
        .into_path()
        .map_err(|e| i18n::error_video_open(lang, &e.to_string()))?;
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();

    // ffprobe and ffmpeg start outside the shell lock.
    let interval_s = app
        .state::<SharedShell>()
        .lock()
        .unwrap()
        .settings
        .interval_s;
    let total_frames = probe_duration_s(&path).map(|d| ((d / interval_s).floor() as u64) + 1);
    let source = VideoSource::open(&path, interval_s)
        .map_err(|e| i18n::error_video_open(lang, &e.to_string()))?;

    let shell_state = app.state::<SharedShell>();
    // Held from the "no session yet" check until the new session is stored (as in
    // session_start). Nothing below waits for the main thread.
    let mut shell = shell_state.lock().unwrap();
    if shell.session.is_some() {
        return Err(i18n::error_session_already_running(lang).to_string());
    }

    let root = PathBuf::from(&shell.settings.save_root);
    let meta = SessionMeta {
        app_version: shell.app_version.clone(),
        source: Source {
            kind: OutKind::Mp4,
            app: None,
            title: None,
            platform: state::platform_str().to_string(),
            file: Some(file_name.clone()),
        },
        region: None, // full frame (SPEC.md: no region UI in mp4 mode)
        interval_s,
    };
    let writer = SessionWriter::create(&root, chrono::Local::now(), meta)
        .map_err(|e| i18n::error_save_folder(lang, &e.to_string()))?;
    let dir = writer.dir().to_path_buf();
    let session = Session::new(
        writer,
        zsp_detect::Config {
            interval_s,
            ..Default::default()
        },
        Box::new(OsTrash),
    );
    let handle = zsp_session::spawn(
        Box::new(source),
        session,
        interval_s,
        notify::make_notify(app.clone()),
    );

    shell.session = Some(ActiveSession {
        handle,
        dir,
        source_kind: SourceKind::Mp4,
        source_title: file_name,
        interval_s,
        thumb: None,
        video_total_frames: total_frames,
    });
    drop(shell);

    ui::sync(app);
    notify::notify_started(app, lang);
    crate::library::notify_changed(app);
    Ok(())
}

/// The tray menu's "動画ファイルから抽出…": same as the command, but with nowhere to
/// show an error, and moved off the main thread (where menu events arrive) since the
/// dialog's blocking API must not run there.
pub fn extract_video_from_menu<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn_blocking(move || {
        let _ = extract_video(&app);
    });
}
