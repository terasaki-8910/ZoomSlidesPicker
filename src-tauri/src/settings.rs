//! Settings persistence: JSON in the app config dir (docs/impl/shell.md > Behaviour).
//! Defaults and validation match docs/impl/ipc.md exactly.

use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager, Runtime};

use crate::i18n;
use crate::ipc_types::{Lang, LangSetting, Settings, SettingsView, Shortcuts};

const SETTINGS_FILE: &str = "settings.json";

pub fn default_shortcuts() -> Shortcuts {
    if cfg!(target_os = "macos") {
        Shortcuts {
            save_now: "Control+Alt+Super+S".into(),
            undo: "Control+Alt+Super+Z".into(),
            pause_resume: "Control+Alt+Super+P".into(),
        }
    } else {
        Shortcuts {
            save_now: "Control+Alt+Shift+S".into(),
            undo: "Control+Alt+Shift+Z".into(),
            pause_resume: "Control+Alt+Shift+P".into(),
        }
    }
}

/// `<OS Pictures folder>/ZoomSlidesPicker`, or `./ZoomSlidesPicker` when the OS reports
/// no Pictures folder.
pub fn default_save_root() -> PathBuf {
    dirs::picture_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("ZoomSlidesPicker")
}

pub fn default_settings() -> Settings {
    Settings {
        language: LangSetting::Auto,
        save_root: default_save_root().to_string_lossy().into_owned(),
        interval_s: 0.5,
        shortcuts: default_shortcuts(),
    }
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path()
        .app_config_dir()
        .ok()
        .map(|dir| dir.join(SETTINGS_FILE))
}

/// Loads settings.json from the app config dir, falling back to defaults when it is
/// missing, unreadable, or fails to parse (never panics on a corrupt file).
pub fn load<R: Runtime>(app: &AppHandle<R>) -> Settings {
    settings_path(app)
        .and_then(|path| fs::read_to_string(path).ok())
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_else(default_settings)
}

pub fn save<R: Runtime>(app: &AppHandle<R>, settings: &Settings) -> Result<(), String> {
    let path = settings_path(app).ok_or("no app config directory")?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(settings).map_err(|e| e.to_string())?;
    fs::write(&path, text).map_err(|e| e.to_string())
}

/// "ja" when `language` is "ja", or "auto" and the OS locale starts with "ja"; else "en"
/// (docs/impl/ipc.md > SettingsView.effectiveLanguage).
pub fn effective_language(language: LangSetting) -> Lang {
    match language {
        LangSetting::Ja => Lang::Ja,
        LangSetting::En => Lang::En,
        LangSetting::Auto => {
            let is_ja = sys_locale::get_locale()
                .map(|l| l.to_lowercase().starts_with("ja"))
                .unwrap_or(false);
            if is_ja { Lang::Ja } else { Lang::En }
        }
    }
}

pub fn view(settings: Settings) -> SettingsView {
    let effective_language = effective_language(settings.language);
    SettingsView {
        language: settings.language,
        save_root: settings.save_root,
        interval_s: settings.interval_s,
        shortcuts: settings.shortcuts,
        effective_language,
    }
}

/// Interval 0.5-5 in steps of 0.5; each shortcut must parse as a
/// tauri-plugin-global-shortcut accelerator (docs/impl/ipc.md > set_settings).
pub fn validate(settings: &Settings, lang: Lang) -> Result<(), String> {
    let steps = settings.interval_s * 2.0;
    let in_range = (0.5..=5.0).contains(&settings.interval_s);
    if !in_range || (steps.round() - steps).abs() > 1e-9 {
        return Err(i18n::error_interval_range(lang).to_string());
    }
    for accel in [
        &settings.shortcuts.save_now,
        &settings.shortcuts.undo,
        &settings.shortcuts.pause_resume,
    ] {
        if accel
            .parse::<tauri_plugin_global_shortcut::Shortcut>()
            .is_err()
        {
            return Err(i18n::error_shortcut_invalid(lang).to_string());
        }
    }
    Ok(())
}
