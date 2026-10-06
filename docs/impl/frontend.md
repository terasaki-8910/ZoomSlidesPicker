# Frontend implementation spec

React + TypeScript + Vite, pnpm. Read with: SPEC.md (画面と操作), design_brief.md (rules and
the strings table), docs/impl/ipc.md (the only way to talk to Rust), ACCEPTANCE.md (C1-C9),
and `~/.claude/rules/ui.md` (personal UI direction; loaded from `.claude/rules/ui.md`).

## Layout

```
package.json  pnpm-lock.yaml  index.html  vite.config.ts  tsconfig.json
eslint.config.js  playwright.config.ts
src/
  main.tsx            mounts <App/>; picks the view from location.hash: #/panel, #/region, #/settings
  App.tsx
  views/Panel.tsx  views/Region.tsx  views/Settings.tsx
  components/         small shared pieces (Button, Notice, ...)
  i18n/ja.ts  i18n/en.ts  i18n/index.ts   (t(key, params), useLang())
  ipc/types.ts        types from docs/impl/ipc.md, exactly
  ipc/index.ts        typed wrappers over @tauri-apps/api invoke/listen
  ipc/mock.ts         in-memory implementation with sample data (no real user data)
  styles/tokens.css   design tokens (the only place with colour values)
  styles/*.css        component styles, tokens only
src/**/*.test.tsx     vitest + Testing Library (jsdom)
tests/ui/*.spec.ts    Playwright + @axe-core/playwright (scripts/ui-check.sh)
scripts/lint-text.mjs scripts/ui-check.sh
```

`package.json` scripts: `dev`, `build` (`tsc -b && vite build`), `build:mock`
(`VITE_IPC_MOCK=1` build into `dist-mock/`), `preview:mock`, `test` (`vitest run`), `lint`
(`eslint . && node scripts/lint-text.mjs`), `tauri` (`tauri`).
Dependencies include `@tauri-apps/api` 2 and, as a dev dependency, `@tauri-apps/cli` 2 (the
Rust side uses it later). Icons: `react-icons` (ui.md default).

`src/ipc/index.ts` uses the mock when `import.meta.env.VITE_IPC_MOCK === "1"`; tests mock the
module with `vi.mock`. The mock keeps state and emits `app-state`/`settings` to listeners so
flows work end to end in the browser (start a session, pause, stop, and so on).

## Views

### Panel (`#/panel`, 320 px wide)

Header: the app name, small, and a gear icon button (label 設定 / Settings) -> `open_settings`.

States, in this order of precedence:

1. macOS without permission (`permission.granted === false`): the permission notice (strings
   table), a button 「システム設定を開く」 -> `open_permission_settings`, a button
   「許可を確かめる」 -> `request_permission` then refresh. 「開始」 is shown disabled.
2. A video is being processed (`videoProgress`): 「動画を処理中…」 with done / total.
3. Running or paused (`session !== null`):
   - Status line 「保存中：12 枚」 or 「一時停止中：12 枚」 and the elapsed time `h:mm:ss`
     (tabular numerals).
   - The latest capture thumbnail (`lastThumb`), as a button that opens the folder.
   - Banners when `windowLost` (window lost notice) or `blank` (single-colour notice).
   - Primary: 「停止」. Secondary: 「一時停止」 or 「再開」, 「今すぐ保存」, 「直前の保存を取り消す」.
4. Stopped: status 「停止中」 and the primary 「開始」, which switches to the picker. If
   `lastSession`: secondary 「保存先を開く」 and 「PDF を作り直す」 (show the result inline:
   「N ページで作り直しました」 or the error). Nothing else (design_brief: no unusable controls).

Picker (inside the panel): heading 「授業の窓を選ぶ」, a list of `WindowEntry` (thumbnail, app,
title) as a listbox: Arrow keys move, Enter picks (`begin_region`), Esc goes back; a 「戻る」
button; a loading state while `list_windows` runs; an empty state 「撮れる窓がありません」.

Keyboard anywhere in the panel: Esc closes it (`close_window`) unless the picker is open
(then Esc goes back); Cmd+, (macOS) / Ctrl+, (others) -> `open_settings`.

### Region (`#/region`)

Loads `get_region_snapshot`. Shows the image fitted to the window. Drag to draw a
rectangle; four corner handles resize it; outside the rectangle is darkened, inside keeps its
brightness; a label shows the size in the captured window's pixels (`W × H px`). Minimum
16 x 16 pixels. One hint line (strings table). Buttons 「開始」 (primary) and 「取り消す」.
Enter starts (`start_session` with the rectangle as fractions, or the whole window when
nothing is drawn); Esc cancels (`cancel_region`).

### Settings (`#/settings`)

Sections: 一般 (language select 自動 / 日本語 / English; save folder path + 「変更…」 ->
`choose_save_root` then `set_settings`), キャプチャ (comparison period in seconds, 0.5-5,
step 0.5, with validation), ショートカット (three rows; 「変更」 records the next key
combination, Esc cancels recording; shown in the platform's notation). Changes save
immediately through `set_settings`; a polite status line confirms 「保存しました」 or shows the
error.

## Language

All strings in `src/i18n/ja.ts` and `src/i18n/en.ts` (same keys; the strings table in
design_brief.md is the source for the ones it lists). The language is
`SettingsView.effectiveLanguage`; update on the `settings` event. Call `apply_language` on
mount and whenever `effectiveLanguage` changes.

## Visual rules

- Colours only from `src/styles/tokens.css` (light values on `:root`, dark values under
  `@media (prefers-color-scheme: dark)`). Neutrals are pure grey; one accent colour; a danger
  colour used only for errors and warnings (design_brief: no red "recording" dot).
- System fonts: `-apple-system, "Hiragino Sans", "Hiragino Kaku Gothic ProN", "Yu Gothic UI",
  "Segoe UI", sans-serif`. A small type scale (12/13/15/17/20), spacing scale (4/8/12/16/24),
  tabular numerals for counts and times.
- One primary action at a time; other actions secondary. Every interactive element is
  keyboard reachable with a visible focus ring. Respect `prefers-reduced-motion`.

## Checks

- `pnpm test` (vitest): ACCEPTANCE C1 (language from `effectiveLanguage`; switching calls
  `set_settings` and `apply_language`), C2 (picker -> region -> start using only the keyboard;
  Esc cancels), C3 (stopped shows only 開始 [plus last-session actions]; running shows count,
  thumbnail, pause, save now, undo, stop; paused shows 一時停止中 and 再開), C4 (Cmd+, and Ctrl+,
  call `open_settings`), C5 (axe on each view, jsdom), C7 (no permission: notice shown, 開始
  disabled), C8 (last-session actions only with `lastSession`; they call `open_folder` /
  `rebuild_pdf`), C9 (Esc in the panel calls `close_window`).
- `pnpm lint`: eslint (typescript-eslint, react-hooks, jsx-a11y recommended) and
  `scripts/lint-text.mjs`, which fails on (a) any emoji (`\p{Extended_Pictographic}`) in
  `src/`, `crates/*/src/`, `src-tauri/src/`, `tests/`, and (b) colour literals (`#rgb`,
  `#rrggbb`, `#rrggbbaa`, `rgb(`, `rgba(`, `hsl(`, `hsla(`) in `src/` outside
  `src/styles/tokens.css`.
- `sh scripts/ui-check.sh`: builds with the mock, serves it, and runs Playwright on Chromium:
  each view (panel stopped / picker / running / no permission; region; settings) at its window
  size, in light and dark; axe must report no violations (including colour contrast); no
  horizontal overflow. Saves screenshots to `test-results/ui/` (gitignored) for review.
