# History window ("library") implementation spec

The user's requests (2026-10-07):
> 常駐のバーのアプリとは別にウィンドウで開くアプリでこれまでの履歴とかをvoice transcriptみたいに検索、フォルダを開くとかのものができるようにしてください。

> アイデアとして常駐のバーからの開始だけでなくウィンドウからの開始もできるようにしてください。というのも、Macで上のバーが埋まっていることが多く、わざわざ探すのが大変だからです。

Decided in round 3 (DECISIONS.md, 2026-10-07 ラウンド 3): R3-4 a (a "履歴" window in the same
app; in the Dock and Cmd-Tab while open), R3-5 a (list and search on the left, the chosen
session's slides on the right), R3-6 (search title, date, app, memo; not slide text), R3-7
(open folder, open PDF, rebuild PDF, rename, session to the trash, chosen images to the
trash, merge duplicate slides, memo; start and stop in the window's top bar), R3-8 a (opening
the app opens this window), R3-9 (merge when checks pass, build the DMG).

The prototype on branch `feat/library-proto` (layout A and B, mock data) is the starting
point for the UI. Layout B is dropped (R3-5 a).

This file is the contract for both sides. docs/impl/ipc.md stays the contract for everything
else; the types and commands below are added to it when this lands.

## Window

- Label `library`, URL `index.html#/library`, title `ZoomSlidesPicker`, 1100 x 720, minimum
  760 x 520, resizable, centered, normal decorations. One instance: opening it again focuses
  it.
- Opened by: the tray menu item 「履歴…」 / "History…" (new, first item), a history icon
  button in the panel header (next to the gear), and the app itself:
  - At launch, the library window opens (R3-8 a), unless a test hook variable is set
    (`ZSP_AUTOSTART_TITLE`, `ZSP_OPEN_WINDOW`; README > development notes).
  - macOS `RunEvent::Reopen` (Finder, Spotlight, Launchpad, Dock while running) opens or
    focuses the library window (it used to open the panel centered).
- macOS: while the library window exists, `ActivationPolicy::Regular` (Dock icon, Cmd-Tab);
  when it is destroyed, back to `Accessory`. Set the policy before showing the window, then
  focus it. Windows: a normal taskbar window.
- Closing destroys it (like the other windows). Quitting the app is still only from the tray
  menu (closing this window does not quit).

## Layout (layout A of the prototype)

Top bar, full width, one row:
- Left: `ZoomSlidesPicker` (font-size-md, weight 600).
- Right: the session state and controls, the same words as the panel:
  - stopped: `停止中` + primary `開始`
  - running: `保存中：12 枚` + elapsed `0:18:20` (tabular-nums) + secondary `一時停止` +
    primary `停止`
  - paused: `一時停止中：12 枚` + elapsed + secondary `再開` + primary `停止`
  - then an IconButton `設定` (gear) -> `open_settings`.
  - macOS without Screen Recording permission: `開始` is disabled and a short notice with
    「システム設定を開く」 shows under the top bar (same strings as the panel).
- `開始` opens the window picker as a modal dialog inside this window (the panel's picker,
  moved to a shared component `src/components/WindowPicker.tsx` and used by both). Choosing a
  window calls `begin_region` as from the panel; the region window opens, this window stays
  open. The dialog closes with Escape, a backdrop click, or its close button (ui.md: light
  dismiss).

Left column (320 px, its own scroll): search field and the session list, as in the
prototype. Search matches custom title, source title, source app, memo, and the date forms
(`10/7`, `10月7日`, `2026-10-07`); space-separated terms, all must match, case-insensitive.
Row: thumbnail of the first slide (64 x 36), title, meta line, badge (`記録中` or
`PDF なし`). Keyboard as in the prototype (listbox; Up/Down, Home/End).

Right pane (its own scroll):
- Title (font-size-xl) + IconButton `名前を変更`. Rename edits in place: a text field with the
  current title selected; Enter or blur saves, Escape cancels; an empty value goes back to the
  source title.
- Meta line as in the prototype.
- Memo: a labelled multi-line text field `メモ` under the meta line (placeholder
  `授業の内容や課題など`), saved 600 ms after typing stops and on blur. Shows `保存しました`
  briefly (text-secondary) after a save.
- Action row (secondary buttons, wrap): `フォルダを開く`, `PDF を開く` (only when there is a
  PDF), `PDF を作り直す`, `同じスライドをまとめる…`, `画像を選ぶ`; then, pushed right,
  `削除…` (danger-styled secondary).
- Grid of slide thumbnails as in the prototype. A tile is a button: click or Enter opens the
  image with the OS default viewer (`open_capture`). In select mode each tile is a checkbox
  instead, and the action row turns into `選んだ画像をごみ箱へ（3 枚）` (primary, disabled at
  0) + `やめる`.
- While the chosen session is recording: rename, memo, and open folder work; `PDF を開く`,
  `PDF を作り直す`, `同じスライドをまとめる…`, `画像を選ぶ`, and `削除…` are not shown
  (design_brief: no unusable controls), and a line `記録中のため、整理は停止してからできます。`
  explains why.

Confirmations (in-page dialogs, never `window.confirm`; light dismiss + explicit buttons):
- `削除…`: `「{title}」をごみ箱に移しますか？ フォルダごと移します。ごみ箱から戻せます。`
  with `ごみ箱に移す` (danger primary) and `やめる`.
- `選んだ画像をごみ箱へ（N 枚）`: `N 枚をごみ箱に移して、PDF を作り直しますか？` with
  `ごみ箱に移す` and `やめる`.
- `同じスライドをまとめる…`: calls `fold_session`; when it returns indices, enters select
  mode with exactly those selected and shows
  `同じスライドとみなした N 枚を選びました。確かめてから、ごみ箱に移してください。`; when it
  returns none, an inline note `同じスライドは見つかりませんでした。` Nothing is moved
  until the user confirms the selection above.

Empty states: no sessions at all: `まだ記録はありません。` + `開始` (primary, opens the
picker); search with no hits: `見つかりませんでした。`

Errors from commands show inline under the action row (Notice component), in the effective
language, as returned.

Narrow window (760 px): the left column 280 px, grid columns shrink, no horizontal scroll.

## Types (add to src/ipc/types.ts; serde camelCase on the Rust side)

```ts
interface SessionSummary {
  dir: string;                 // absolute session folder: the identifier
  folder: string;              // folder name, e.g. "2026-10-07_1242"
  title: string;               // customTitle, else sourceTitle
  customTitle: string | null;
  memo: string;                // "" when none
  sourceApp: string | null;    // e.g. "Safari"; null for mp4
  sourceTitle: string;         // window title or video file name
  sourceKind: "window" | "mp4";
  startedAt: string;           // RFC 3339 from session.json
  endedAt: string | null;      // null while recording (or after a crash)
  count: number;               // captures whose PNG exists
  hasPdf: boolean;             // slides.pdf exists
  recording: boolean;          // the folder of the running or paused session
  thumbPath: string | null;    // the first existing capture PNG, null when none
}

interface CaptureView {
  index: number;
  file: string;                // e.g. "003_00-04-12.png"
  path: string;                // absolute path of the PNG
  tContent: number;
}

interface SessionDetail extends SessionSummary {
  captures: CaptureView[];     // in index order, only those whose PNG exists
}
```

Thumbnails: the UI never gets image bytes through commands. `thumbUrl(path)` in
`src/ipc/index.ts` returns `convertFileSrc(path, "zspthumb") + "?w=" + width` (Tauri builds
`zspthumb://localhost/...` on macOS and `http://zspthumb.localhost/...` on Windows); `<img
loading="lazy">` loads only what is on screen. The mock returns generated SVG data URLs.

## Commands (add to docs/impl/ipc.md)

All library commands are `async` (they read folders, decode images, or open dialogs) and run
off the main thread. `dir` must be a folder directly inside a known root (see Roots) with a
session.json; anything else is an error `記録が見つかりません。` / `Session not found.`

| Command | Args | Returns | Behaviour |
|---|---|---|---|
| `open_library` | | | Opens or focuses the library window (macOS: Regular policy). |
| `list_sessions` | | `SessionSummary[]` | Every session folder in every known root, newest `startedAt` first. Unreadable folders and folders without a parsable session.json are skipped. |
| `get_session` | `dir: string` | `SessionDetail` | |
| `open_session_folder` | `dir` | | Opens the folder with the OS (opener). |
| `open_session_pdf` | `dir` | | Opens slides.pdf; error `PDF がありません。` / `There is no PDF.` when missing. |
| `rebuild_session_pdf` | `dir` | `number` | zsp-output `rebuild_pdf`; returns the page count. Error while recording. |
| `rename_session` | `dir, title: string \| null` | | Writes `title` to meta.json (trimmed; empty or null clears it). Allowed while recording. |
| `set_session_memo` | `dir, memo: string` | | Writes `memo` to meta.json. Allowed while recording. |
| `trash_session` | `dir` | | Moves the folder to the OS trash. Error while recording. Clears `lastSession` if it pointed there. |
| `trash_captures` | `dir, indices: number[]` | `number` | Moves those PNGs to the trash, removes their records from session.json, rebuilds slides.pdf when it existed (or deletes it when no image is left); returns the number moved. Error while recording. |
| `fold_session` | `dir` | `number[]` | Dry run of `zsp_detect::fold_captures` over the session's existing PNGs (in index order); returns the indices it would not keep. Error while recording. |
| `open_capture` | `path: string` | | Opens one capture PNG with the OS default app. The path must be a capture of a known session. |

Events (Rust -> all windows): `library-changed` (no payload) after anything that changes what
`list_sessions` or `get_session` returns: a session starts, a capture is saved, replaced, or
undone, a session finishes, a library command changes something, the save root changes. The
library refetches the list, and the open detail if any, on it (coalesce bursts: at most one
refetch per 300 ms).

`begin_region` gains: when called from the library window, it does not close it (it still
closes the panel when called from there).

## Rust side

- `crates/zsp-output`:
  - `read_session(dir) -> io::Result<SessionFile>`.
  - Session metadata in a separate file `meta.json` in the session folder:
    `{ "title": string | null, "memo": string }`, read with defaults when missing, written
    atomically. A separate file so a rename or memo never races the session writer, which
    rewrites session.json after every capture.
  - `trash_captures(dir, indices, trash) -> io::Result<usize>`: as the command above (records
    removed, PDF rebuilt or removed, `pdf` updated: pages and `rebuilt_at`, or null).
  - Tests (ACCEPTANCE B11, B12).
- `src-tauri/src/library.rs`: the commands, the scan, the `zspthumb` protocol, the window.
- Roots: the current save root (the `ZSP_SAVE_ROOT` override when set) plus the roots used
  before. Those are kept by Rust only, in the settings file under `knownRoots` (`string[]`,
  default empty): not part of the `Settings` type the UI sends, and kept as they are by
  `set_settings`; when `set_settings` changes `saveRoot`, the old root is appended if
  missing. Missing folders are skipped.
- `zspthumb` protocol (`register_asynchronous_uri_scheme_protocol`): the request path is a
  capture PNG path (URL-decoded); serve only files that end in `.png` and lie directly in a
  session folder of a known root (canonicalize both, compare), else 404. Query `w` (default
  360, clamp 64-800): JPEG (quality 80) at most that wide, cached in
  `<app cache dir>/thumbs/<hash of path, size, and mtime>.jpg`; serve from the cache when
  present. Content-Type `image/jpeg`. Decode and resize off the main thread.
- CSP `img-src 'self' data: zspthumb: http://zspthumb.localhost`. Capability windows gain
  `library`; still only `core:default`.
- Tray menu: `履歴…` / `History…` as the first item. Panel header: a history IconButton
  (FiClock or similar from react-icons, label `履歴` / `History`) -> `open_library`.
- `library-changed`: emitted from `ui::sync` callers where a notice changes the count
  (Saved, Replaced, Undone, Finished), on start, and from the library commands.

## Strings (add to src/i18n; ja and en)

| Key | ja | en |
|---|---|---|
| library.open | 履歴… | History… |
| library.openButton | 履歴 | History |
| library.memo | メモ | Memo |
| library.memoPlaceholder | 授業の内容や課題など | Notes about the lecture, assignments, … |
| library.saved | 保存しました | Saved |
| library.recordingLocked | 記録中のため、整理は停止してからできます。 | Stop recording to tidy up this session. |
| library.confirmDelete | 「{title}」をごみ箱に移しますか？ フォルダごと移します。ごみ箱から戻せます。 | Move “{title}” to the Trash? The whole folder moves; you can restore it from the Trash. |
| library.confirmDeleteButton | ごみ箱に移す | Move to Trash |
| library.confirmTrashImages | {count} 枚をごみ箱に移して、PDF を作り直しますか？ | Move {count} images to the Trash and rebuild the PDF? |
| library.dedupeSelected | 同じスライドとみなした {count} 枚を選びました。確かめてから、ごみ箱に移してください。 | Selected {count} images that repeat a slide. Check them, then move them to the Trash. |
| library.dedupeNone | 同じスライドは見つかりませんでした。 | No repeated slides found. |
| library.pickTitle | 授業の窓を選ぶ | (same as the panel's picker title) |
| library.empty | まだ記録はありません。 | No sessions yet. |
| library.notFound | 記録が見つかりません。 | Session not found. |
| library.noPdfError | PDF がありません。 | There is no PDF. |
| library.cancel | やめる | Cancel |

Keep the prototype's other keys (search, meta, recording, noPdf, hasPdf, source, rename,
openFolder, openPdf, rebuildPdf, dedupe, select, trashSelected, cancelSelect, delete,
noHits, caption). Drop the layout-B-only keys (open, folder). Rust-side strings (tray item,
errors) go in src-tauri/src/i18n.rs.

## Checks

ACCEPTANCE C10-C14 (UI, on the mock) and B11-B12 (zsp-output); `sh scripts/check.sh` passes.
