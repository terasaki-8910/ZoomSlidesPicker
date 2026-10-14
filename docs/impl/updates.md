# In-app updates implementation spec

The user's request (2026-10-14):
> ZoomSlidesPickerにもアプリ内の自動更新を入れてください。グローバルのCLAUDE.mdの仕様（起動時に確認、今すぐ更新/後で/この版をスキップ、設定に「アップデートを確認」、署名つき、作業中は再起動しない）とvoice-transcriptのSPEC.md > In-app updatesに合わせて。

Decided (DECISIONS.md, 2026-10-14 アプリ内の自動更新): update files live in a separate public
repository, `terasaki-8910/ZoomSlidesPicker-releases` (this repository stays private, and a
private repository's release assets cannot be fetched without signing in); the check runs at
launch only, plus the manual check in Settings; the dialog shows the new and the current
version only (no release notes); the signing key's backup is the user's.

This file is the contract for both sides. docs/impl/ipc.md stays the contract for everything
else; the types, commands, and events below are added to it when this lands.

## Config (done on main)

- `src-tauri/tauri.conf.json`: `bundle.createUpdaterArtifacts: true`;
  `plugins.updater.pubkey` (the minisign public key of `~/.tauri/zoomslidespicker-updater.key`)
  and `plugins.updater.endpoints`:
  `https://github.com/terasaki-8910/ZoomSlidesPicker-releases/releases/latest/download/latest.json`.
  The updater fetches only that endpoint and refuses any bundle not signed by that key.
- `src-tauri/tauri.local.conf.json` turns updater artifacts off, for builds without the
  private key: `pnpm tauri build --config src-tauri/tauri.local.conf.json`.
- Crate: `tauri-plugin-updater` 2.x, pinned with `~` to a release whose `tauri` requirement
  the app's tauri (2.12.x) already meets. Adding it must not change the versions of `tauri`,
  `tauri-runtime-wry`, `wry`, or `tao` in Cargo.lock (check the lockfile diff). No
  `tauri-plugin-process` and no `@tauri-apps/plugin-*` packages: Rust drives the updater and
  restarts the app itself, so the webview still has only `core:default`
  (`src-tauri/capabilities/default.json` unchanged).

## Types (src/ipc/types.ts and src-tauri/src/ipc_types.rs)

```ts
interface UpdateInfo { version: string; current: string }
interface UpdateCheck { current: string; available: UpdateInfo | null; skipped: boolean }
type UpdatePhase = "downloading" | "installing";
interface UpdateProgress { phase: UpdatePhase; downloaded: number; total: number | null }
type UpdateAnswer = "later" | "skip";
```

## Commands

| Command | Args | Returns | Behaviour |
|---|---|---|---|
| `get_app_version` | none | `string` | The running version (`package_info().version`). |
| `get_update_offer` | none | `UpdateInfo \| null` | The launch check's find while it is unanswered this run; null otherwise (nothing found, check failed or still running, answered, or skipped version). |
| `check_update` | none | `UpdateCheck` | Manual check. Ignores the skipped version (asking is the way back to a skipped update); `skipped` says whether `available` is the skipped one. Keeps the found update for `install_update`. Error: the updater's message, as is. |
| `answer_update` | `answer: UpdateAnswer` | `null` | `later`: this run does not offer it again (the next launch does). `skip`: records the found version as skipped (one version at a time) and does not offer it again at launch; a newer version is offered as usual. Both clear the launch offer. |
| `install_update` | none | `null` (macOS: never returns, the app restarts) | Error if nothing was found, or if a session is active (`Shell.session` is some: recording, paused, stopping, or a video being processed) -- the UI disables the button then, this is the guard. Otherwise sets "installing", downloads with `update-progress` events, waits for running PDF jobs (below) to end, installs, and restarts: macOS via `AppHandle::restart()`; Windows' install hands over to the installer, which quits the app and starts the new version. On failure: clears "installing" and returns the error. |

While installing, these refuse to start, returning the localized `update_in_progress`
message: `begin_region`, `start_session`, `extract_video` (and its tray menu item), and the
PDF jobs `export_pdf`, `rebuild_pdf`, `trash_captures`, `trash_session`. The PDF jobs hold a
guard while they run, and `install_update` waits for the ones already running before it
installs.

## Events (Rust -> all windows)

| Event | Payload | When |
|---|---|---|
| `update-offer` | `UpdateInfo` | The launch check found a version that is not the skipped one. |
| `update-progress` | `UpdateProgress` | While installing: on each downloaded chunk (throttled to about 10 per second), then once with phase `installing`. |

## Rust (src-tauri/src/updates.rs)

- Managed state: the found `tauri_plugin_updater::Update` (from the launch check or the
  latest manual check), the unanswered launch offer, and the installing flag.
- The skipped version lives in `update.json` in the app config dir
  (`{ "skippedVersion": "0.1.8" }`), not in settings.json, whose `Settings` part the webview
  writes back whole.
- Launch check: spawned from `setup`, after the library window is opened. Skipped in debug
  builds and when a test hook variable is set (`ZSP_AUTOSTART_TITLE`, `ZSP_OPEN_WINDOW`), and
  when `ZSP_NO_UPDATE_CHECK` is set. A failed check (offline, GitHub unreachable) stays
  silent (stderr only). A find whose version is the skipped one is dropped.
- `AppHandle::restart()` goes through `RunEvent::ExitRequested` with a code in tauri 2.12;
  with no session left (guarded above) the existing handler lets the exit through. Check this
  in tauri's source rather than assuming.

## UI

Strings (both languages, src/i18n and src-tauri/src/i18n.rs where Rust returns them):

| Key | ja | en |
|---|---|---|
| `update.title` | 新しい版があります | A new version is available |
| `update.body` | ZoomSlidesPicker {version} が出ています。 | ZoomSlidesPicker {version} is available. |
| `update.current` | いまの版は {current} です。 | You have {current}. |
| `update.now` | 今すぐ更新 | Update now |
| `update.later` | 後で | Later |
| `update.skip` | この版をスキップ | Skip this version |
| `update.busy` | 記録中は更新できません。停止すると更新できます。 | You can't update while recording. Stop to update. |
| `update.updatingTitle` | 更新しています | Updating |
| `update.downloading` | ダウンロード中… {percent}% | Downloading… {percent}% |
| `update.downloadingNoTotal` | ダウンロード中… | Downloading… |
| `update.installing` | インストール中… | Installing… |
| `update.restartNote` | 終わるとアプリが再起動します。 | The app restarts when it's done. |
| `update.failed` | 更新できませんでした：{error} | Couldn't update: {error} |
| `update.close` | 閉じる | Close |
| `settings.version` | バージョン {version} | Version {version} |
| `settings.checkUpdate` | アップデートを確認 | Check for updates |
| `settings.checking` | 確認しています… | Checking… |
| `settings.upToDate` | 最新の版です。 | You're on the latest version. |
| `settings.updateAvailable` | {version} があります。 | {version} is available. |
| `settings.updateSkipped` | {version} はスキップしています。 | You skipped {version}. |
| `settings.checkFailed` | 確認できませんでした：{error} | Couldn't check: {error} |
| Rust `update_in_progress` | 更新しています。終わるとアプリが再起動します。 | Updating. The app restarts when it's done. |

Update dialog (`src/components/UpdateDialog.tsx`, on the existing `Modal`):

- Offer: title `update.title`; `update.body` and `update.current` on two lines; buttons, left
  to right: `update.skip` (left, secondary), then `update.later` (secondary) and `update.now`
  (primary) on the right. Escape and a click outside mean Later.
- Busy (a session is active, from `app-state`: `session !== null || videoProgress !== null`):
  `update.now` disabled, `update.busy` shown under the text. Live: it re-enables when the
  session ends.
- Installing: title `update.updatingTitle`; `update.downloading` with the percent (or
  `update.downloadingNoTotal` when the total is unknown), then `update.installing`; and
  `update.restartNote`. No buttons; Escape and a click outside do nothing (the app restarts
  as soon as the install finishes; a deliberate exception to light dismiss, for this
  progress state only).
- Failed: `update.failed` and a `update.close` button.

Where it shows:

- Library window: on mount it calls `get_update_offer` and listens for `update-offer`; an
  offer opens the dialog. Later -> `answer_update("later")`; Skip ->
  `answer_update("skip")`; Update now -> `install_update`.
- Settings > 一般: a row with `settings.version` and a `settings.checkUpdate` button
  (`settings.checking`, disabled, while checking). Result line under it: nothing found ->
  `settings.upToDate`; found -> the dialog opens in the settings window, and the line says
  `settings.updateSkipped` if it is the skipped version, else `settings.updateAvailable`;
  error -> `settings.checkFailed`. In this dialog, Later just closes it, Skip calls
  `answer_update("skip")`.

## Release side (main session, not the implementation agents)

`scripts/release.sh <version>` builds the DMG and the Windows installer with the signing key
(`TAURI_SIGNING_PRIVATE_KEY` from `~/.tauri/zoomslidespicker-updater.key`, the password from
the Keychain item `zoomslidespicker-updater-key`), writes `latest.json` (darwin-aarch64:
`.app.tar.gz` and its signature; windows-x86_64: the NSIS installer and its signature), and
uploads the installers, update files, and `latest.json` to a release in the public
repository, and the installers to this repository's release as before.

Bootstrapping: 0.1.6 and earlier have no updater. The first version with it has to be
installed by hand once on each machine; later versions arrive through the dialog.
