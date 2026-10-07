# History window (draft for the round 3 sheet)

The user's request (2026-10-07):
> 常駐のバーのアプリとは別にウィンドウで開くアプリでこれまでの履歴とかをvoice transcriptみたいに検索、フォルダを開くとかのものができるようにしてください。

Status: draft. Only the prototype view (mock data, not wired to Rust) is built, on branch
`feat/library-proto`, to show real screenshots on the sheet. Nothing here is decided until
the round 3 answers are in DECISIONS.md.

## Prototype scope (what the prototype agent builds)

A React view `src/views/Library.tsx` at `index.html#/library`, running on the IPC mock
(`VITE_IPC_MOCK=1`), with two layouts so the sheet can show both:

- Layout A (default): two panes.
- Layout B (`#/library?layout=list` -- the query goes before the hash: `/?layout=list#/library`):
  one list, like voice-transcript's history.

No Rust. New IPC functions exist only in the mock and as typed wrappers (see "Data").

## Window

Normal window (title bar), label `library`, title `ZoomSlidesPicker`, 1100 x 720 by default,
minimum 760 x 520, resizable. Body background `--color-bg`.

### Toolbar (both layouts), full width, one row

- Left: `ZoomSlidesPicker` (font-size-md, weight 600).
- Right: the current session state and its controls, same words as the panel:
  - stopped: `停止中` + primary button `開始`
  - running: `保存中：12 枚` + elapsed `0:18:20` (tabular-nums) + secondary `一時停止` +
    primary `停止`
  - paused: `一時停止中：12 枚` + secondary `再開` + primary `停止`
  - and an IconButton for settings (same as the panel header).
- The mock's preset decides the state (reuse the panel presets: `stopped`, `running`).

### Layout A: list and detail

- Left column, fixed 320 px, its own scroll:
  - Search field at the top (type `search`, placeholder `題名・日付・アプリで検索`), with a
    visually hidden label.
  - The session list, newest first. Each row (a button-like listbox option):
    - thumbnail of the first slide, 64 x 36, radius-sm, border
    - title: the custom name if set, else the source title (window title or video file
      name), one line with ellipsis, font-size-sm weight 600
    - meta line, font-size-xs, text-secondary, tabular-nums:
      `10月7日（水）12:42 · 33 分 · 36 枚`
    - badges after the meta when they apply: `記録中` (accent text on a subtle background,
      for the running session), `PDF なし` (text-secondary)
  - Selected row: `--color-bg-elevated` background and an accent left border 3 px.
  - Keyboard: the list is a listbox; Up/Down move the selection, Home/End jump.
- Right pane, fills the rest, its own scroll, padding space-xl:
  - Title (font-size-xl, weight 600) and an IconButton `名前を変更` next to it.
  - Meta line: `Safari の「第1回授業スライド.mp4」 · 10月7日（水）12:42–13:15 · 36 枚 · PDF あり`
  - Action row (secondary buttons, wrap): `フォルダを開く`, `PDF を開く`,
    `PDF を作り直す`, `同じスライドをまとめる…`, `画像を選ぶ`; then, pushed right,
    a danger-styled secondary button `削除…`.
  - Grid of slide thumbnails: `repeat(auto-fill, minmax(200px, 1fr))`, gap space-md.
    Each tile: the image (16:9, border, radius-sm) and a caption `001 · 0:03:12`
    (font-size-xs, tabular-nums, text-secondary). In "select" mode each tile shows a
    checkbox and the action row turns into `選んだ画像をごみ箱へ（3 枚）` + `やめる`.
  - Empty states: no sessions at all -- `まだ記録はありません。授業の窓を選んで開始すると、ここに並びます。`
    and a primary `開始`; search with no hits -- `見つかりませんでした。`
- Narrow window (below 900 px wide): the left column stays 280 px, the grid columns shrink;
  no horizontal page scroll at 760 px.

### Layout B: one list

- Under the toolbar, the search field (max-width 640 px), then rows of full width
  (max-width 960 px, centred):
  thumbnail 96 x 54, title + meta (as in A), and on the right the actions as text buttons
  `開く`, `フォルダ`, `PDF`, and an IconButton `削除` (trash icon, danger colour).
- `開く` replaces the list with the detail view of A's right pane, plus a `戻る` button at
  the top left.

## Search

Case-insensitive; spaces separate terms, all must match (AND), each term may match any
field: custom name, source title, source app, and the date written as `10/7`, `10月7日`,
`2026-10-07`. Updates as you type. The prototype filters the mock list client-side.

## Data (mock now; Rust after GO)

```ts
interface SessionSummary {
  id: string;              // folder name, e.g. "2026-10-07_1242"
  title: string;           // custom name, else source title
  customTitle: string | null;
  sourceApp: string | null;   // e.g. "Safari"; null for mp4
  sourceTitle: string;     // window title or video file name
  sourceKind: "window" | "mp4";
  startedAt: string;       // RFC 3339
  endedAt: string | null;  // null while recording
  count: number;
  hasPdf: boolean;
  recording: boolean;
  thumb: string | null;    // data URL of the first slide (small), null when count is 0
}
interface SessionDetail extends SessionSummary {
  captures: { index: number; file: string; tContent: number; thumb: string }[];
}
listSessions(): Promise<SessionSummary[]>
getSession(id: string): Promise<SessionDetail>
```

Mock data: six sessions (all invented; no real lecture content):
1. recording now: `第2回授業スライド.mp4`, Safari, today 14:10, 12 slides
2. `第1回授業スライド.mp4`, Safari, today 12:42-13:15, 36 slides, PDF
3. custom name `デジタル人文学 第1回`, source `Zoom ミーティング`, zoom.us, Oct 3 10:30-12:00, 58 slides, PDF
4. `統計学特論`, `Zoom ミーティング`, zoom.us, Oct 2 13:00-14:30, 41 slides, PDF
5. `lecture_2026-10-01.mp4` (mp4 mode), Oct 1, 22 slides, PDF
6. `情報処理演習 - Microsoft Stream`, Microsoft Edge, Sep 30, 0 slides, no PDF

Thumbnails: generated SVG data URLs that look like plain slides (white, a title bar of
text, three to five grey lines, sometimes a box), different per slide. Light content only;
they are images, so they do not follow the theme.

## Wording (draft; Japanese first, English in src/i18n/en.ts)

| Key | ja | en |
|---|---|---|
| library.search | 題名・日付・アプリで検索 | Search by title, date, or app |
| library.meta | {date} · {duration} · {count} 枚 | {date} · {duration} · {count} slides |
| library.recording | 記録中 | Recording |
| library.noPdf | PDF なし | No PDF |
| library.hasPdf | PDF あり | PDF |
| library.source | {app} の「{title}」 | “{title}” in {app} |
| library.rename | 名前を変更 | Rename |
| library.openFolder | フォルダを開く | Open folder |
| library.openPdf | PDF を開く | Open PDF |
| library.rebuildPdf | PDF を作り直す | Rebuild PDF |
| library.dedupe | 同じスライドをまとめる… | Merge duplicate slides… |
| library.select | 画像を選ぶ | Select images |
| library.trashSelected | 選んだ画像をごみ箱へ（{count} 枚） | Move {count} to Trash |
| library.cancelSelect | やめる | Done |
| library.delete | 削除… | Delete… |
| library.open | 開く | Open |
| library.folder | フォルダ | Folder |
| library.empty | まだ記録はありません。授業の窓を選んで開始すると、ここに並びます。 | No sessions yet. Start capturing a lecture window and it will appear here. |
| library.noHits | 見つかりませんでした。 | No matches. |
| library.caption | {index} · {time} | {index} · {time} |

Dates: ja `10月7日（水）12:42`, en `Oct 7 (Tue) 12:42`; durations: ja `33 分` / `1 時間 30 分`,
en `33 min` / `1 h 30 min`.

## Checks for the prototype

`pnpm lint`, `pnpm test` (a few tests: search filters by each field, AND of terms, empty
states, keyboard selection), `sh scripts/ui-check.sh` with a new `tests/ui/library.spec.ts`:
axe has no violations and there is no horizontal overflow, for layout A at 1280 x 800 and
760 x 600 and layout B at 1280 x 800 (list and opened), each light and dark; screenshots go
to `test-results/ui/library-*.png`.

## After GO: implementation outline (draft, the parts no round 3 answer changes)

Rust (new module `src-tauri/src/library.rs`, data helpers in `zsp-output`):
- Session roots: the current save root plus every save root used before (settings gain
  `knownRoots: string[]`, appended when the save root changes). `list_sessions` scans each
  root's direct subfolders for `session.json`, newest first; unreadable folders are skipped.
- `SessionSummary` from session.json: title = `title` (new optional field, written by
  rename) or source title; count = captures whose PNG still exists; hasPdf = slides.pdf
  exists; recording = the folder of the active session.
- Thumbnails: JPEG 320 px wide in the app cache dir (`<cache>/thumbs/<session id>/<file>.jpg`),
  made on first request in a background thread; the webview gets them through the asset
  protocol scoped to the cache dir only (capability change: `core:default` stays; add the
  asset protocol scope in tauri.conf.json, nothing else).
- Actions: open folder / open PDF (opener), rebuild PDF (`zsp_output::rebuild_pdf`, any
  session), rename (rewrite session.json `title` atomically), delete session (trash the
  folder), delete images (trash the PNGs, drop them from session.json, rebuild the PDF),
  merge duplicate slides (run the detector over the session's PNGs as in
  `crates/zsp-detect/examples/dedupe_session.rs`, return the files it would not keep as a
  pre-selection; nothing is deleted until the user confirms).
- Window: label `library`; opening it switches macOS to `ActivationPolicy::Regular` (Dock and
  Cmd-Tab); closing the last library window switches back to `Accessory`. Reopen (Finder,
  Spotlight, Dock) opens it (if R3-8 a). Tray menu gains `履歴…`; the panel header gains a
  history IconButton.
- The toolbar's start: opens the window picker as a dialog inside the library window
  (reusing the panel's picker component), then the region window, as from the panel.
- session.json schema: optional `title` (string) and, if R3-7 includes memo, `memo`.
