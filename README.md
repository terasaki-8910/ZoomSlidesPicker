# ZoomSlidesPicker

授業の画面（Zoom のライブ、SharePoint の動画の再生など）から、先生がスライドを切り替えるたびにその画像を自動で保存し、授業の後にスライド一式（PNG と PDF）を手元に残す。Mac と Windows で動く常駐アプリ。

仕様は [SPEC.md](SPEC.md)、合格の条件は [ACCEPTANCE.md](ACCEPTANCE.md)、判断の記録は [DECISIONS.md](DECISIONS.md)。

## できること

- 授業の窓を選び、スライドの部分を囲むと、スライドが変わるたびに保存する。他の窓が重なっても、選んだ窓の中身を撮る（macOS で確認済み。ただし、完全に隠れた窓をブラウザが描き直し続けるかは未確認）。
- スライドの中で動画や Web カメラの映像が動いている間は保存しない。動画が止まったら、その見た目を 1 枚保存する（止まってから 15 秒後か、次のスライドに替わったとき）。
- 録画の講義に講師のカメラが小さく重なっていても、カメラの変化だけでは保存しない。
- 再生ボタンを押したときやマウスを動かしたときに一瞬出るプレーヤーの操作ボタンは、5 秒以内に消えれば保存しない。
- 箇条書きが順に出るスライドは、最後の状態を 1 枚にまとめる（暗い背景のスライドでも、横に黒い帯がある画面でも同じ）。先生が話してから次の行を出すときも、保存した 1 枚を新しい版に置き換える（前の版はごみ箱へ）。ペンの書き込みも同じ。
- 前のスライドに戻っても、同じ画像は保存しない（戻った時刻は記録する）。
- 遅れて鮮明になる画面共有では、鮮明になった版を保存する。
- 黒や灰色の画面は保存しない。10 秒続いたら通知する（共有の停止、キャプチャ防止、最小化の可能性）。
- 停止すると、保存した画像を 1 つの PDF（`slides.pdf`）にまとめる。画像を消した後に PDF を作り直すこともできる。
- 一時停止、今すぐ保存、直前の保存の取り消し。どれも全体のショートカットでも操作できる。
- 動画ファイル（mp4、mov、m4v）から同じ判定で抽出する（PC に ffmpeg が必要）。

## 使い方

1. 起動すると、メニューバー（Windows は通知領域）にアイコンだけが出る。Mac では Dock に出ない。メニューバーが混んでいてアイコンが見えないとき（ノッチの裏に隠れるなど）は、アプリをもう一度起動するとパネルが画面の中央に開く。
2. アイコンを押してパネルを開き、「開始」を押す。授業の窓を選ぶ。
3. 選んだ窓の画像の上で、スライドの部分をドラッグで囲み、Enter（または「開始」）。囲まずに Enter を押すと窓全体。
4. 授業の間は放っておいてよい。保存した枚数はアイコンの横（Windows はツールチップ）に出る。
5. 終わったら「停止」。PDF ができると通知が出る。

保存先は `~/Pictures/ZoomSlidesPicker/<開始日時>/`（Windows は `%USERPROFILE%\Pictures\ZoomSlidesPicker\`）。設定（Mac は ⌘,、Windows は Ctrl+,）で変えられる。

フォルダの中身:

- `001_00-03-12.png` など: 連番と、開始からの経過時間（そのスライドが落ち着いた時刻）
- `session.json`: 各画像の時刻、戻った記録、一時停止などの出来事（形は [docs/session.schema.json](docs/session.schema.json)）
- `slides.pdf`: 保存した画像を順に並べた PDF（画像は劣化させずに入れる）

### ショートカット（既定。設定で変えられる）

| 操作 | Mac | Windows |
|---|---|---|
| 今すぐ保存 | ⌃⌥⌘S | Ctrl+Alt+Shift+S |
| 直前の保存を取り消す | ⌃⌥⌘Z | Ctrl+Alt+Shift+Z |
| 一時停止と再開 | ⌃⌥⌘P | Ctrl+Alt+Shift+P |

### macOS の画面収録の許可

初めて撮るときに「画面収録」の許可が要る。パネルの案内から「システム設定」を開いて ZoomSlidesPicker を許可し、アプリを再起動する。macOS 15 以降は、許可の確認が月に一度ほど出ることがある。

## 既知の制約

- 主催者がキャプチャ防止を有効にした授業（Zoom の「画面キャプチャの防止」など）は撮れない。黒や灰色の画面として通知される。
- Mac で、全画面表示の Zoom（別の操作スペース）を撮れるかは未確認。
- マウスを動画の上に置いたままにして再生バーが出続けると、バーが写った版が保存される。録画の再生中はマウスを動画の外に置く。範囲は Zoom の操作バーを避けて選ぶ。
- 動画の中に写っている操作ボタン（講師の画面の PowerPoint の操作ボタンなど）は、スライドの一部として保存される。
- 講師のカメラが映り始めてすぐ（数十秒）に大きく動くと、同じスライドを 1 枚余分に保存することがある。
- 画面がロックされている間やディスプレイが眠っている間は、ブラウザが画面を描かないので、新しいスライドは保存できない（2026-10-07 に確認）。
- 同じ窓の中でブラウザのタブを切り替えると、そのタブの内容が保存される。録画は専用の窓で開く。
- 明るさが同じで色だけが変わる変化は検出しない。
- 判定の数値は合成の画面で調整した。実際の授業での精度は、使いながら確かめる段階。
- 署名と公証をしていない。macOS で「壊れているため開けません」と出たら、`xattr -cr /Applications/ZoomSlidesPicker.app` を実行してから開く。署名がないので、作り直した版に入れ替えると、画面収録の許可をもう一度求められることがある（システム設定の一覧に古い項目が残っていたら、消してから許可し直す）。

## 開発

構成:

| 場所 | 中身 |
|---|---|
| `crates/zsp-detect` | 判定（解析画像と、保存するかどうかの判断）。参照実装は `samples/proto/detect.py` |
| `crates/zsp-output` | PNG、`session.json`、PDF、ごみ箱 |
| `crates/zsp-capture` | 窓の一覧とキャプチャ（xcap）、範囲、mp4（ffmpeg）、macOS の権限 |
| `crates/zsp-session` | 上の 3 つをつなぐエンジン（スレッド、操作、窓を見失ったとき） |
| `src-tauri` | 常駐アイコン、窓、ショートカット、通知、設定、コマンド |
| `src` | 画面（React）。Rust とのやり取りは [docs/impl/ipc.md](docs/impl/ipc.md) |
| `fixtures` | テスト用の合成の授業画面（`samples/fixture/` で生成） |
| `spike` | キャプチャの検証用（ラウンド 1）。Windows での確認手順は `docs/WINDOWS-CHECK.md` |

コマンド:

```sh
pnpm install
sh scripts/check.sh          # すべての検査（Rust のテスト・fmt・clippy、画面のテスト・lint・UI チェック）
pnpm tauri dev               # 開発用に起動
pnpm tauri build             # 配布物（macOS は .app と .dmg）
```

Windows 用のインストーラーは Mac でも作れる（手順は `docs/WINDOWS-CHECK.md` の最後）。

テスト用の環境変数（受け入れの確認用。普段は使わない）:

| 変数 | 働き |
|---|---|
| `ZSP_AUTOSTART_TITLE` | 起動時に、題名にこの文字列を含む窓で記録を始める（窓全体） |
| `ZSP_AUTOSTART_SECONDS` | その秒数で停止して終了する |
| `ZSP_SAVE_ROOT` | 保存先を差し替える（設定には書かない）。他の変数がなくても効く |
| `ZSP_OPEN_WINDOW` | 起動時に `panel`、`settings`、`region`、`library` の窓を開く。`region` は `ZSP_AUTOSTART_TITLE` の窓を使い、記録は始めない。どれかの変数があると、起動時に履歴の窓は開かない（`library` を指定したときを除く） |

実際の画面で判定を試す（記録した窓の連番と `index.tsv` を判定に通し、保存する所を表示する。記録には授業の内容が写るので、リポジトリには入れない）:

```sh
cargo run --release -p zsp-detect --example replay -- <記録のフォルダ> <x> <y> <w> <h>
cargo run -p zsp-detect --example trace_fixture -- F7   # fixture での判定の出来事
```

記録中のセッションの窓を 1 秒ごとに縮小画像で残し、保存と突き合わせる（画像そのものは残さない）。縮小画像は fixture と同じ形に直して、判定に通し直せる（`ZSP_CFG=live_min_cells=6` のように数値を差し替えられる）:

```sh
.venv/bin/python samples/verify/record_grids.py <窓の ID> <セッションのフォルダ> grids.npz
.venv/bin/python samples/verify/verify_session.py <セッションのフォルダ> grids.npz
.venv/bin/python samples/verify/grids_to_fixture.py grids.npz <作業用のフォルダ>
cargo run --release -p zsp-detect --example trace_fixture -- <作業用のフォルダ>
```

fixture の作り直し（`samples/fixture/fixture.py` を変えたときだけ。`F7` のように番号を付けると、その fixture だけ）:

```sh
python3 -m venv .venv && .venv/bin/pip install numpy pillow
.venv/bin/python samples/fixture/export_fixtures.py
```
