//! Native strings: everything outside the webview (tray menu, tooltip, notifications,
//! window titles, command error messages) in Japanese and English (design_brief.md >
//! 主な文言, SPEC.md > 画面と操作). Rebuilt by `apply_language` (docs/recipes/
//! tauri-desktop-app.md gotcha 7).
//!
//! Most strings are copied verbatim from design_brief.md's table (and, for error
//! messages the UI already defines, from `src/i18n/{ja,en}.ts`) so the same thing reads
//! the same way in the webview and outside it. A handful have no counterpart there
//! (the start notification, the region window's OS title, and a few error paths
//! design_brief.md does not cover) -- those are written to match that table's tone and
//! are called out in the shell implementation report.

use crate::ipc_types::Lang;

pub fn tray_start(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "開始…",
        Lang::En => "Start…",
    }
}

pub fn tray_stop(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "停止",
        Lang::En => "Stop",
    }
}

pub fn tray_open_folder(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "保存先を開く",
        Lang::En => "Open save folder",
    }
}

pub fn tray_rebuild_pdf(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "PDF を作り直す",
        Lang::En => "Rebuild PDF",
    }
}

pub fn tray_extract_video(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "動画ファイルから抽出…",
        Lang::En => "Extract from video file…",
    }
}

pub fn tray_settings(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "設定…",
        Lang::En => "Settings…",
    }
}

pub fn tray_quit(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "終了",
        Lang::En => "Quit",
    }
}

/// Tray tooltip (and, on Windows -- which has no tray title -- the only place the
/// capture count is shown; SPEC.md > 使い方の流れ). Matches the panel's own wording
/// (`panel.statusRunning` / `panel.statusPaused` in src/i18n/{ja,en}.ts) so the two
/// surfaces agree.
pub fn tooltip_idle() -> &'static str {
    "ZoomSlidesPicker"
}

pub fn tooltip_running(lang: Lang, count: usize) -> String {
    match lang {
        Lang::Ja => format!("保存中：{count} 枚"),
        Lang::En => format!("Capturing: {count} slides"),
    }
}

pub fn tooltip_paused(lang: Lang, count: usize) -> String {
    match lang {
        Lang::Ja => format!("一時停止中：{count} 枚"),
        Lang::En => format!("Paused: {count} slides"),
    }
}

/// Window titles (docs/impl/ipc.md > Windows). The panel has no decorations, so this
/// title is not shown in a title bar, but still names the window (Mission Control,
/// screen readers, window switchers). Settings matches `settings.title` in
/// src/i18n/{ja,en}.ts; the region window has no counterpart there, so this is written
/// to match its in-panel heading's tone.
pub fn title_panel() -> &'static str {
    "ZoomSlidesPicker"
}

pub fn title_region(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "範囲を選ぶ",
        Lang::En => "Select the region",
    }
}

pub fn title_settings(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "設定",
        Lang::En => "Settings",
    }
}

/// design_brief.md has no wording for the start notification (only stop, blank,
/// window-lost, and the macOS permission notice); written to match their tone.
pub fn notify_start_title() -> &'static str {
    title_panel()
}

pub fn notify_start_body(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "キャプチャを開始しました。",
        Lang::En => "Started capturing.",
    }
}

/// design_brief.md > 通知（停止）, verbatim. With no captures there is no PDF
/// (zsp-output writes none), so that case gets its own line, which design_brief.md does
/// not cover.
pub fn notify_stop_body(lang: Lang, count: usize) -> String {
    match (lang, count) {
        (Lang::Ja, 0) => "保存したスライドはありません。".to_string(),
        (Lang::En, 0) => "No slides were saved.".to_string(),
        (Lang::Ja, _) => format!("{count} 枚を保存し、PDF を作りました。"),
        (Lang::En, _) => format!("Saved {count} slides and created a PDF."),
    }
}

/// design_brief.md > 通知（単色）, verbatim.
pub fn notify_blank_body(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => {
            "画面が 10 秒以上、単色のままです。共有の停止、キャプチャ防止、最小化の可能性があります。"
        }
        Lang::En => {
            "The screen has been a single colour for over 10 seconds. Sharing may have \
             stopped, capture may be blocked, or the window may be minimized."
        }
    }
}

/// design_brief.md > 通知（窓を見失った）, verbatim.
pub fn notify_window_lost_body(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "授業の窓が見つからないので、一時停止しました。",
        Lang::En => "Paused because the lecture window can't be found.",
    }
}

/// design_brief.md > 権限の案内（Mac）, verbatim -- also returned as the
/// `start_session` error message when permission is missing, and shown by
/// `permission.notice` in the webview.
pub fn permission_notice(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => {
            "画面収録の許可が必要です。システム設定で許可してから、アプリを再起動してください。"
        }
        Lang::En => {
            "Screen recording permission is needed. Allow it in System Settings, then restart the app."
        }
    }
}

/// `start_session` error: a session is already running (docs/impl/ipc.md). Not covered
/// by design_brief.md's wording table.
pub fn error_session_already_running(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "すでにキャプチャ中です。",
        Lang::En => "A capture is already running.",
    }
}

/// `set_settings` error: the interval is out of range. Matches `settings.intervalError`
/// in src/i18n/{ja,en}.ts and the IPC mock verbatim.
pub fn error_interval_range(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "比較の周期は 0.5〜5 秒、0.5 刻みで指定してください。",
        Lang::En => "The check interval must be between 0.5 and 5 seconds, in steps of 0.5.",
    }
}

/// `set_settings` error: a shortcut string does not parse. Not covered by
/// design_brief.md's wording table (which only shows the recorder UI, not this failure).
pub fn error_shortcut_invalid(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "ショートカットの形式が正しくありません。",
        Lang::En => "The shortcut format is invalid.",
    }
}

/// `rebuild_pdf` error: no last session, or it has no images. Matches the IPC mock
/// verbatim.
pub fn error_no_last_session_images(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "直前のセッションの画像がありません。",
        Lang::En => "There are no images from the last session.",
    }
}

/// `extract_video` error: ffmpeg/ffprobe are not on PATH (SPEC.md > 構成: not bundled).
pub fn error_ffmpeg_missing(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "ffmpeg が見つかりません。インストールしてから、もう一度お試しください。",
        Lang::En => "ffmpeg was not found. Install it and try again.",
    }
}

/// Starting a session: the session folder could not be created under the save root (a
/// removed drive, no write access). Not covered by design_brief.md.
pub fn error_save_folder(lang: Lang, detail: &str) -> String {
    match lang {
        Lang::Ja => format!("保存先に書き込めません：{detail}"),
        Lang::En => format!("Can't write to the save folder: {detail}"),
    }
}

/// `extract_video`: ffmpeg could not open the chosen file. Not covered by design_brief.md.
pub fn error_video_open(lang: Lang, detail: &str) -> String {
    match lang {
        Lang::Ja => format!("動画を読み込めません：{detail}"),
        Lang::En => format!("Can't read the video: {detail}"),
    }
}

/// Generic capture-layer failure (`list_windows`, `begin_region`): the underlying OS
/// call failed. `detail` is appended for diagnosis (SPEC.md gives no wording for this
/// rare path).
pub fn error_capture_failed(lang: Lang, detail: &str) -> String {
    match lang {
        Lang::Ja => format!("キャプチャに失敗しました：{detail}"),
        Lang::En => format!("Capture failed: {detail}"),
    }
}

/// Tray menu's new first item (docs/impl/library.md > Rust side): matches `library.open`
/// in src/i18n/{ja,en}.ts, so the wording agrees wherever it appears.
pub fn tray_history(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "履歴…",
        Lang::En => "History…",
    }
}

/// Every library command taking `dir`: it doesn't resolve to a session folder directly
/// inside a known root with a parsable session.json. Matches `library.notFound` in
/// src/i18n/{ja,en}.ts.
pub fn library_not_found(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "記録が見つかりません。",
        Lang::En => "Session not found.",
    }
}

/// `open_session_pdf`: there is no slides.pdf in the session folder. Matches
/// `library.noPdfError` in src/i18n/{ja,en}.ts.
pub fn library_no_pdf(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "PDF がありません。",
        Lang::En => "There is no PDF.",
    }
}

/// `rebuild_session_pdf`, `trash_session`, `trash_captures`, `fold_session`: the session
/// is still recording. Matches `library.recordingLocked` in src/i18n/{ja,en}.ts, which the
/// window shows inline instead -- this is the defensive command-level copy of the same
/// wording for the rare case a stale UI still sends the request.
pub fn library_recording_locked(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "記録中のため、整理は停止してからできます。",
        Lang::En => "Stop recording to tidy up this session.",
    }
}

/// `rebuild_session_pdf`: the session has no capture images left to build a PDF from
/// (zsp-output's `rebuild_pdf` returns `InvalidInput`). Not covered by
/// docs/impl/library.md's string table.
pub fn library_no_images(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "画像がないため、PDF を作れません。",
        Lang::En => "There are no images to build a PDF from.",
    }
}

/// `trash_captures`: one of the given indices doesn't exist (zsp-output's
/// `trash_captures` returns `NotFound`). Not covered by docs/impl/library.md's string
/// table.
pub fn library_bad_index(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "指定した画像が見つかりません。",
        Lang::En => "One of the selected images was not found.",
    }
}

/// `open_capture`: the path is not a capture PNG of a known session. Not covered by
/// docs/impl/library.md's string table.
pub fn library_capture_not_found(lang: Lang) -> &'static str {
    match lang {
        Lang::Ja => "画像が見つかりません。",
        Lang::En => "The image was not found.",
    }
}
