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
| `start_session` | `windowId: number, region: RegionSel` | | Validates the region, closes the region window, starts capturing, sends the start notification. Error if a session is already running or permission is missing. |
| `cancel_region` | | | Closes the region window. |
| `pause_session`, `resume_session`, `save_now`, `undo_last`, `stop_session` | | | Session controls; no-ops when they do not apply. `stop_session` writes the PDF and sends the stop notification. |
| `open_folder` | | | Opens the current session folder, else the last session folder. |
| `rebuild_pdf` | | `number` | Rebuilds the last session's PDF (zsp-output `rebuild_pdf`); returns the page count. Error when there is no last session or no images. |
| `extract_video` | | | Native file dialog (mp4, mov, m4v), then processes the video like a session (full frame, video time); progress in `AppState.videoProgress`. Error when ffmpeg is not on PATH. |
| `request_permission` | | `boolean` | macOS: asks for Screen Recording permission (system prompt once); returns whether granted now. |
| `open_permission_settings` | | | macOS: opens the Screen Recording page of System Settings. |
| `open_settings` | | | Opens or focuses the settings window. |
| `close_window` | | | Closes the calling window. |
| `fit_panel` | `height: number` | | The panel reports its content height in CSS pixels (on mount and whenever it changes); Rust resizes the panel window to that height (clamped to 160-640, width stays 320) and keeps it at the tray icon. Ignored for other windows. |

## Events (Rust -> all windows)

| Event | Payload | When |
|---|---|---|
| `app-state` | `AppState` | After every change: session status, captures, last session, permission, video progress. |
| `settings` | `SettingsView` | After `set_settings`. |

## Windows

Created when opened, destroyed when closed (SPEC.md > 画面と操作). No window opens at launch.

| Label | URL | Size | Notes |
|---|---|---|---|
| `panel` | `index.html#/panel` | 320 wide, about 480 tall | No decorations, always on top, not in the taskbar; placed at the tray icon (positioner); closes when it loses focus. |
| `region` | `index.html#/region` | 1100 x 760, resizable | Centered. |
| `settings` | `index.html#/settings` | 560 x 560 | Normal window with a title. |
