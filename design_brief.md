# design_brief — ZoomSlidesPicker

全体の方針は `~/.claude/rules/ui.md`（`.claude/rules/ui.md` から参照）。ここには、このアプリに固有の決まりだけを書く。色、字の大きさ、余白の値は design gate でトークンとして決める。

## このアプリの性格

授業の間、裏で黙って動く道具。見るのは、始めるときと、ときどき枚数を確かめるときだけ。目立たないことを優先する。

## すること

- 主な操作は、いつでも 1 つだけを目立たせる。停止中は「開始」、実行中は「停止」。それ以外は二次的な見た目にする。
- 状態（停止中、実行中、一時停止中）は、形と文字の両方で示す。色だけに頼らない。
- 停止中は、使えない操作（一時停止、今すぐ保存、直前の保存の取り消し）を出さない。
- 実行中は、直前に保存した 1 枚を小さく見せる。押すと保存先のフォルダを開く。
- 数字（枚数、時刻）は等幅の数字（tabular-nums）にする。
- 範囲を選ぶ画面は、範囲の外を暗くし、範囲の中は元の明るさのままにする。四隅につまみを付け、範囲の大きさ（px）を出す。Enter と Esc の案内は 1 行だけ。
- 設定は ⌘,（Mac）と Ctrl+,（Windows）で開く。パネルにも歯車のアイコンを置く。
- 中立色は純粋なグレー。強調の色は 1 つだけ。
- 赤は、失敗と警告にだけ使う。実行中を赤い点（録画の印）で示さない。実行中はアイコンの形（塗り）で示す。
- 常駐アイコンは単色の画像にする（Mac はテンプレート画像。メニューバーの明暗に合わせて色が変わる）。
- UI のアイコンは react-icons（`ui.md` の既定）。
- 字は OS の書体（日本語はヒラギノ角ゴシック、Windows は Yu Gothic UI）。
- ライトとダークは OS の設定に従う。

## しないこと

- 絵文字、装飾だけの影やぼかし、意味のないグラデーション。
- 同じ重さの操作を並べること（主な操作が分からなくなる）。
- 説明の文やツールチップの付けすぎ。
- 色を直接書くこと（トークンだけを使う）。

## 主な文言（案）

ボタンは、押すと何が起きるかを動詞で書く。

| 場所 | 日本語 | English |
|---|---|---|
| 開始のボタン | 開始 | Start |
| 停止のボタン | 停止 | Stop |
| 一時停止のボタン | 一時停止 | Pause |
| 再開のボタン | 再開 | Resume |
| 今すぐ保存のボタン | 今すぐ保存 | Save now |
| 取り消しのボタン | 直前の保存を取り消す | Undo last save |
| 状態（停止中） | 停止中 | Stopped |
| 状態（実行中） | 保存中：12 枚 | Capturing: 12 slides |
| 状態（一時停止中） | 一時停止中：12 枚 | Paused: 12 slides |
| 窓を選ぶ画面の見出し | 授業の窓を選ぶ | Choose the lecture window |
| 範囲を選ぶ画面の案内 | スライドの部分を囲んで、Enter で決定（Esc で取り消し） | Drag around the slide, then press Enter (Esc to cancel) |
| mp4 のメニュー | 動画ファイルから抽出… | Extract from video file… |
| 通知（停止） | 12 枚を保存し、PDF を作りました。 | Saved 12 slides and created a PDF. |
| 通知（単色） | 画面が 10 秒以上、単色のままです。共有の停止、キャプチャ防止、最小化の可能性があります。 | The screen has been a single colour for over 10 seconds. Sharing may have stopped, capture may be blocked, or the window may be minimized. |
| 通知（窓を見失った） | 授業の窓が見つからないので、一時停止しました。 | Paused because the lecture window can't be found. |
| 権限の案内（Mac） | 画面収録の許可が必要です。システム設定で許可してから、アプリを再起動してください。 | Screen recording permission is needed. Allow it in System Settings, then restart the app. |
| 設定の項目 | 言語（自動／日本語／English）、保存先、比較の周期（秒）、ショートカット | Language (Auto / 日本語 / English), Save location, Check interval (seconds), Shortcuts |
