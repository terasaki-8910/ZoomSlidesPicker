# Tauri shell implementation spec (`src-tauri/`)

The shell connects the session engine (`crates/zsp-session`) to the tray, the windows, and the
React UI. Read with: SPEC.md (使い方の流れ, キャプチャ, 画面と操作, 構成), docs/impl/ipc.md (the
contract, implement every command and event exactly), design_brief.md (strings),
docs/recipes/tauri-desktop-app.md (gotchas 3, 4, 7 apply).

## Crate and config

- `src-tauri/` package `zoom-slides-picker` (lib `zoom_slides_picker_lib`), added to the root
  workspace `members`. Dependencies: tauri 2 (features `tray-icon`, `image-png`), tauri-build 2,
  tauri-plugin-global-shortcut 2, tauri-plugin-notification 2, tauri-plugin-dialog 2,
  tauri-plugin-opener 2, tauri-plugin-positioner 2 (feature `tray-icon`), the workspace crates,
  serde, serde_json, chrono, sys-locale, dirs, base64. Stay on Tauri 2 (crates.io also has a
  3.0 alpha; do not use it).
- `tauri.conf.json`: productName `ZoomSlidesPicker`, identifier
  `dev.terasaki8910.zoomslidespicker`, `app.windows: []` (no window at launch), a CSP that
  allows `data:` images (`default-src 'self'; img-src 'self' data:; style-src 'self'
  'unsafe-inline'`), dev server and build commands for the Vite frontend at the repo root,
  `bundle.icon` listed explicitly (32x32.png, 128x128.png, 128x128@2x.png, icon.icns, icon.ico;
  recipe gotcha 4). Add `"packageManager"` to package.json (recipe gotcha 3).
- `capabilities/default.json`: windows `panel`, `region`, `settings`; permissions `core:default`
  only. Rust calls the plugins itself; the webview gets no plugin permissions.
- Icons: a script `scripts/make-icons.py` (Python, Pillow; the repo's `.venv` has it) draws
  a plain monochrome app icon source (1024 x 1024: a slide rectangle, no text) and three tray
  glyphs (stopped: outline, running: filled, paused: outline with two bars): macOS template
  images (black on transparent, 32 x 32 and 64 x 64) and, for Windows and Linux, which have
  no template images and may have a dark or light taskbar, the same glyphs in white on an
  accent badge (32 x 32). Generate the bundle icons with `pnpm tauri icon`. Commit the
  outputs.

## Frontend touch points

The React UI is done (`src/`, running on `src/ipc/mock.ts` until now). Two small changes
belong to this task because they span both sides:
- `vite.config.ts`: `server: { port: 1420, strictPort: true }`, matching `devUrl`.
- `fit_panel` (docs/impl/ipc.md): add the wrapper to `src/ipc/index.ts` (no-op in the mock),
  and make the panel view report its content height with a `ResizeObserver` on its root
  element. The panel window is otherwise a fixed 480 px and mostly empty when stopped.
After these, `pnpm lint`, `pnpm test`, and `sh scripts/ui-check.sh` must still pass.

## Behaviour

- macOS: `ActivationPolicy::Accessory` (no Dock icon).
- Tray: icon by state (template images on macOS). Left click toggles the panel; right click
  shows the menu: 開始… (opens the panel), 停止, 保存先を開く, PDF を作り直す,
  動画ファイルから抽出…, 設定…, 終了. Items that do not apply are disabled. While a session
  runs, macOS shows the capture count as the tray title; Windows shows it in the tooltip
  (titles are unsupported on Windows). Menu, tooltip, and window titles follow the effective
  language and are rebuilt by `apply_language`.
- Windows: created when opened, destroyed when closed (docs/impl/ipc.md > Windows). The panel
  is placed at the tray icon with the positioner plugin and closes when it loses focus.
- Settings: JSON in the app config dir; defaults in docs/impl/ipc.md. Effective language:
  `sys-locale` starts with `ja` -> ja, else en (unless set explicitly).
- Shortcuts: register the three accelerators from settings at startup and after every
  `set_settings` (unregister the old ones first); they map to `save_now`, `undo_last`, and
  pause/resume, and do nothing without a session.
- Sessions: `start_session` creates a `SessionWriter` under the save root (meta: app version,
  source window app and title, platform, region, interval), a `Session` with
  `zsp_detect::Config { interval_s, ..Default::default() }` and `OsTrash`, a `WindowSource`,
  and calls `zsp_session::spawn`. Notices update the state and drive notifications:
  start, `Blank`, `WindowLost`, `Finished` (count and PDF). The last capture's thumbnail
  (at most 280 px wide, data URL) is computed once per capture and cached. While a session
  runs, emit `app-state` at least once per second (elapsed time) and after every notice.
- `extract_video`: dialog (mp4, mov, m4v), `VideoSource` at the settings interval, source kind
  mp4 with the file name; progress from the engine status (frames = elapsed / interval + 1,
  total from the video duration via ffprobe when available).
- Permission (macOS): `permission.granted` from `zsp_capture::permission::screen_capture_granted`;
  `start_session` refuses without it and sends the permission notification.
- Quit: stop a running session first (so the PDF is written), then exit.
- Errors returned to the UI are short strings in the effective language.

## Threads

- Commands without `async` run on the main thread, as do tray, menu, and global-shortcut
  handlers. Tray and menu calls made from any other thread block until the main thread has
  run them. So no thread may hold the shell state lock while making such a call: it would
  deadlock against the main thread waiting for the same lock. `ui::sync` (src-tauri/src/ui.rs)
  is the one place that emits `app-state` and updates the tray; it runs on the main thread
  (posted there without waiting) and must be called without the lock held.
- The session thread (the `Notify` callback) never waits for the main thread, so the main
  thread may join a session thread.
- `async` commands: those that block (dialogs, the permission prompt, captures, writing a
  PDF) and `open_settings` (creating a window in a synchronous command deadlocks on
  Windows, WebView2).

## Test hook (for ACCEPTANCE D1 and D3, and for looking at the real windows)

When the environment variable `ZSP_AUTOSTART_TITLE` is set, the app starts a session at launch
on the first window whose title contains it (whole window). With `ZSP_AUTOSTART_SECONDS=n`
it stops after n seconds and quits. `ZSP_SAVE_ROOT=<dir>` overrides the save root (so tests
never write into the user's Pictures folder). `ZSP_OPEN_WINDOW=panel|settings|region` opens
that window at launch (`region` takes the `ZSP_AUTOSTART_TITLE` window and starts no
session), since the tray icon cannot be clicked from a script. Nothing else changes;
document these in README.md under development notes.

## Checks

- `cargo build -p zoom-slides-picker` and `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo fmt --all --check`
- `pnpm build` then `pnpm tauri build --debug --bundles app` (macOS .app)
- Smoke test with the hook: open `spike/fixtures/stage.html` in a Chrome app window
  (`open -na "Google Chrome" --args --user-data-dir=<tmp> --app=file://<abs path> --window-size=1000,700`),
  run the built binary directly from the terminal (`target/debug/zoom-slides-picker`, not
  `open` on the .app: run from the terminal it uses the terminal's Screen Recording
  permission, while the .app would ask for its own) with
  `ZSP_AUTOSTART_TITLE=ZSP-STAGE ZSP_AUTOSTART_SECONDS=40 ZSP_SAVE_ROOT=<tmp dir>`, and check
  that a session folder with captures, session.json, and slides.pdf appears there. Close the
  Chrome window afterwards (kill that Chrome instance by its --user-data-dir).
