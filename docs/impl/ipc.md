# IPC contract (Rust <-> webview)

Single source of truth for the commands and events between `src-tauri` and the React UI.
The TypeScript types live in `src/ipc/types.ts` and must match the Rust structs here
(serde `rename_all = "camelCase"`). There is no code generation: change both sides together.

The webview never touches files, dialogs, the shell, or the OS directly. Everything goes
through these commands; Rust uses its plugins itself (dialog, opener, notification,
global-shortcut, positioner). So the webview capability needs only `core:default`.

## Types

```ts
type Lang = "ja" | "en";
type LangSetting = "auto" | "ja" | "en";

interface Shortcuts {            // accelerator strings in tauri-plugin-global-shortcut format
  saveNow: string;               // default macOS "Control+Alt+Super+S", Windows "Control+Alt+Shift+S"
  undo: string;                  // default macOS "Control+Alt+Super+Z", Windows "Control+Alt+Shift+Z"
  pauseResume: string;           // default macOS "Control+Alt+Super+P", Windows "Control+Alt+Shift+P"
}

interface Settings {
  language: LangSetting;         // default "auto"
  saveRoot: string;              // default: <OS Pictures folder>/ZoomSlidesPicker
  intervalS: number;             // comparison period N, 0.5-5 in steps of 0.5, default 0.5
  shortcuts: Shortcuts;
}

interface SettingsView extends Settings {
  effectiveLanguage: Lang;       // "ja" when language is "ja", or "auto" and the OS locale starts with "ja"; else "en"
}

type SessionState = "running" | "paused" | "stopped";

interface SessionView {
  state: SessionState;
  count: number;                 // captures in the session
  dir: string;                   // session folder
  lastThumb: string | null;      // data URL (PNG, at most 280 px wide) of the latest capture
  elapsedS: number;
  blank: boolean;                // single-colour screen for 10 s or more
  windowLost: boolean;           // the window is gone; the session is paused
  stopping: boolean;             // Stop was requested; the PDF is being written (until Finished clears the session)
  sourceKind: "window" | "mp4";
  sourceTitle: string;           // window title, or video file name
}

interface LastSession {          // the most recent finished session
  dir: string;
  count: number;
  hasPdf: boolean;
}

interface PermissionView {
  granted: boolean;              // always true outside macOS
  platform: "macos" | "windows" | "linux";
}

interface VideoProgress {
  done: number;                  // frames processed
  total: number | null;          // when known
}

interface AppState {
  session: SessionView | null;   // null when no session is running or paused
  lastSession: LastSession | null;
  permission: PermissionView;
  videoProgress: VideoProgress | null;   // non-null while a video is being processed
}

interface WindowEntry {
  id: number;
  app: string;
  title: string;
  width: number;                 // points
  height: number;
  thumb: string;                 // data URL (PNG, at most 240 px wide)
}

interface Snapshot {             // for the region window
  windowId: number;
  app: string;
  title: string;
  width: number;                 // pixels of the captured window
  height: number;
  image: string;                 // data URL (PNG, at most 1600 px wide)
}

interface RegionSel { x: number; y: number; w: number; h: number }   // fractions 0-1 of the window

interface SessionSummary {   // the history window (docs/impl/library.md)
  dir: string;                 // absolute session folder: the identifier
  folder: string;               // folder name, e.g. "2026-10-07_1242"
  title: string;                // customTitle, else sourceTitle
  customTitle: string | null;
  memo: string;                 // "" when none
  sourceApp: string | null;     // e.g. "Safari"; null for mp4
  sourceTitle: string;          // window title or video file name
  sourceKind: "window" | "mp4";
  startedAt: string;            // RFC 3339 from session.json
  endedAt: string | null;       // null while recording (or after a crash)
  count: number;                // captures whose PNG exists
  hasPdf: boolean;               // slides.pdf exists
  recording: boolean;           // the folder of the running or paused session
  thumbPath: string | null;     // the first existing capture PNG, null when none
}

interface CaptureView {
  index: number;
  file: string;                 // e.g. "003_00-04-12.png"
  path: string;                 // absolute path of the PNG
  tContent: number;
}

interface SessionDetail extends SessionSummary {
  captures: CaptureView[];      // in index order, only those whose PNG exists
}
```

Thumbnails: the UI never gets image bytes through commands. `thumbUrl(path)` in
`src/ipc/index.ts` returns `convertFileSrc(path, "zspthumb") + "?w=" + width` (Tauri builds
`zspthumb://localhost/...` on macOS and `http://zspthumb.localhost/...` on Windows).

```ts
type PdfQuality = "lossless" | "standard" | "light";
interface PdfEstimate { quality: PdfQuality; bytes: number }
interface PdfExport { path: string; pages: number; bytes: number }
```

In-app updates (docs/impl/updates.md has the full contract):

```ts
interface UpdateInfo { version: string; current: string }
interface UpdateCheck { current: string; available: UpdateInfo | null; skipped: boolean }
type UpdatePhase = "downloading" | "installing";
interface UpdateProgress { phase: UpdatePhase; downloaded: number; total: number | null }
type UpdateAnswer = "later" | "skip";
```

## Commands

Invoke names are snake_case; argument keys are camelCase (Tauri converts). Errors are
returned as a string message (in the effective language) and shown by the UI as is.

| Command | Args | Returns | Behaviour |
|---|---|---|---|
| `get_app_state` | | `AppState` | |
| `get_settings` | | `SettingsView` | |
| `set_settings` | `settings: Settings` | `SettingsView` | Validates (interval 0.5-5, step 0.5; shortcuts parse); saves to the app config dir; re-registers shortcuts; emits `settings`. |
| `apply_language` | | | Rebuilds the parts outside the webview in the effective language: tray menu, tray tooltip, window titles. The UI calls it on mount and whenever `effectiveLanguage` changes (docs/recipes/tauri-desktop-app.md, gotcha 7). |
| `choose_save_root` | | `string \| null` | Native folder dialog. Does not save; the UI then calls `set_settings`. |
| `list_windows` | | `WindowEntry[]` | Capturable windows (zsp-capture `list_windows`), with thumbnails. |
| `begin_region` | `windowId: number` | | Captures a snapshot of the window, closes the panel, opens the region window. |
| `get_region_snapshot` | | `Snapshot` | The snapshot taken by `begin_region`. |
| `start_session` | `windowId: number, region: RegionSel` | | Validates the region, starts capturing, closes the region window, sends the start notification. Error if a session is already running, permission is missing, or the window or save folder is unusable; the region window then stays open to show it. |
| `cancel_region` | | | Closes the region window. |
| `pause_session`, `resume_session`, `save_now`, `undo_last`, `stop_session` | | | Session controls; no-ops when they do not apply. `stop_session` sets `session.stopping` at once (and syncs), then the session writes a `standard` PDF and sends the stop notification. |
| `open_folder` | | | Opens the current session folder, else the last session folder. |
| `rebuild_pdf` | | `number` | Rebuilds the last session's PDF (zsp-output `rebuild_pdf`) at the quality recorded in its session.json (`standard` when it had no PDF); returns the page count, which also becomes `lastSession.count`. Error when there is no last session or no images. |
| `extract_video` | | | Native file dialog (mp4, mov, m4v), then processes the video like a session (full frame, video time); progress in `AppState.videoProgress`. Error when ffmpeg is not on PATH. |
| `request_permission` | | `boolean` | macOS: asks for Screen Recording permission (system prompt once); returns whether granted now. |
| `open_permission_settings` | | | macOS: opens the Screen Recording page of System Settings. |
| `open_settings` | | | Opens or focuses the settings window. |
| `close_window` | | | Closes the calling window. |
| `fit_panel` | `height: number` | | The panel reports its content height in CSS pixels (on mount and whenever it changes); Rust resizes the panel window to that height (clamped to 160-640, width stays 320) and keeps it at the tray icon. Ignored for other windows. |

The history window's commands (docs/impl/library.md has the full contract; all are
`async` and run off the main thread). `dir` must be a folder directly inside a known root
with a session.json; anything else is an error `記録が見つかりません。` / `Session not
found.`:

| Command | Args | Returns | Behaviour |
|---|---|---|---|
| `open_library` | | | Opens or focuses the library window (macOS: Regular policy). |
| `list_sessions` | | `SessionSummary[]` | Every session folder in every known root, newest `startedAt` first. Unreadable folders and folders without a parsable session.json are skipped. |
| `get_session` | `dir: string` | `SessionDetail` | |
| `open_session_folder` | `dir` | | Opens the folder with the OS (opener). |
| `open_session_pdf` | `dir` | | Opens slides.pdf; error `PDF がありません。` / `There is no PDF.` when missing. |
| `estimate_pdf` | `dir, indices: number[] \| null` | `PdfEstimate[]` | Predicted PDF size at each quality, in the order lossless, standard, light (zsp-output `estimate_pdf_bytes`: up to 8 evenly spaced images encoded, scaled by count). `indices` null: every existing capture. Error while recording or when no image exists. |
| `export_pdf` | `dir, quality: PdfQuality, indices: number[] \| null` | `PdfExport` | `indices` null: rewrites slides.pdf at `quality` and updates session.json `pdf` (`pages`, `rebuilt_at`, `quality`). Otherwise writes `選んだ画像 YYYY-MM-DD.pdf` / `Selected images YYYY-MM-DD.pdf` (effective language; the date is the session's start date; ` 2`, ` 3`... before `.pdf` when taken) in the folder, in index order, and leaves session.json alone. Only images that still exist count. Error while recording or when none exists. Emits `library-changed`. Runs off the async worker. |
| `rename_session` | `dir, title: string \| null` | | Writes `title` to meta.json (trimmed; empty or null clears it). Allowed while recording. |
| `set_session_memo` | `dir, memo: string` | | Writes `memo` to meta.json. Allowed while recording. |
| `trash_session` | `dir` | | Moves the folder to the OS trash. Error while recording. Clears `lastSession` if it pointed there. |
| `trash_captures` | `dir, indices: number[]` | `number` | Moves those PNGs to the trash, removes their records from session.json, rebuilds slides.pdf when it existed (or deletes it when no image is left); returns the number moved. Error while recording. |
| `fold_session` | `dir` | `number[]` | Dry run of `zsp_detect::fold_captures` over the session's existing PNGs (in index order); returns the indices it would not keep. Error while recording. |
| `open_capture` | `path: string` | | Opens one capture PNG with the OS default app. The path must be a capture of a known session. |

The update commands (docs/impl/updates.md; the webview only calls these, the updater plugin
has no webview permission). While an update installs, `begin_region`, `start_session`,
`extract_video` (and its tray menu item), `rebuild_pdf` (and its tray menu item),
`export_pdf`, `trash_captures`, `trash_session`, and `check_update` return `Updating. The app
restarts when it's done.` / `更新しています。終わるとアプリが再起動します。`.

| Command | Args | Returns | Behaviour |
|---|---|---|---|
| `get_app_version` | | `string` | The running version. |
| `get_update_offer` | | `UpdateInfo \| null` | The launch check's find while unanswered this run; null otherwise. |
| `check_update` | | `UpdateCheck` | Manual check; ignores the skipped version (`skipped` says whether `available` is it); keeps the found update for `install_update`. Error: the updater's message as is. |
| `answer_update` | `answer: UpdateAnswer` | | `later`: not offered again this run. `skip`: records the found version in `update.json` (one at a time); error when nothing was found. Both clear the launch offer. |
| `install_update` | | | Error when nothing was found, or a session is active (recording, paused, stopping, video). Else downloads (`update-progress`), waits for running PDF jobs, installs, restarts (macOS: never returns; Windows: the installer takes over and quits the app). On failure returns the error and clears the installing state. |

## Events (Rust -> all windows)

| Event | Payload | When |
|---|---|---|
| `app-state` | `AppState` | After every change: session status, captures, last session, permission, video progress. |
| `settings` | `SettingsView` | After `set_settings`. |
| `library-changed` | none | After anything that changes what `list_sessions` or `get_session` returns: a session starts, a capture is saved, replaced, or undone, a session finishes, a library command changes something, the save root changes (docs/impl/library.md). |
| `update-offer` | `UpdateInfo` | The launch check found a version that is not the skipped one. |
| `update-progress` | `UpdateProgress` | While installing: each downloaded chunk (at most about 10 per second), a final `downloading` figure, then once with phase `installing`. |

## Windows

Created when opened, destroyed when closed (SPEC.md > 画面と操作), except the library
window, which also opens at launch (unless a test hook variable is set -- docs/impl/shell.md
> Test hook).

| Label | URL | Size | Notes |
|---|---|---|---|
| `panel` | `index.html#/panel` | 320 wide, about 480 tall | No decorations, always on top, not in the taskbar; placed at the tray icon (positioner); closes when it loses focus. |
| `region` | `index.html#/region` | 1100 x 760, resizable | Centered. |
| `settings` | `index.html#/settings` | 560 x 560 | Normal window with a title. |
| `library` | `index.html#/library` | 1100 x 720, minimum 760 x 520, resizable | Centered, normal decorations, title `ZoomSlidesPicker`. One instance: opening it again focuses it. macOS: `ActivationPolicy::Regular` while it exists, `Accessory` once it is destroyed (docs/impl/library.md > Window). |
