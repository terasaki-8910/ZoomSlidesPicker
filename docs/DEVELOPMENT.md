# 開発と検証

[README に戻る](../README.md)。アプリの概要と技術構成は README、判定の数値は [SPEC](../SPEC.md)、合格条件は [ACCEPTANCE](../ACCEPTANCE.md) を参照。


構成:

| 場所 | 中身 |
|---|---|
| `crates/zsp-detect` | 判定（解析画像と、保存するかどうかの判断）。参照実装は `samples/proto/detect.py` |
| `crates/zsp-output` | PNG、`session.json`、PDF、ごみ箱 |
| `crates/zsp-capture` | 窓の一覧とキャプチャ（xcap）、範囲、mp4（ffmpeg）、macOS の権限 |
| `crates/zsp-session` | 上の 3 つをつなぐエンジン（スレッド、操作、窓を見失ったとき） |
| `src-tauri` | 常駐アイコン、窓、ショートカット、通知、設定、コマンド |
| `src` | 画面（React）。Rust とのやり取りは [docs/impl/ipc.md](impl/ipc.md) |
| `fixtures` | テスト用の合成の授業画面（`samples/fixture/` で生成） |
| `spike` | キャプチャの検証用（ラウンド 1）。Windows での確認手順は [Windows の確認手順](WINDOWS-CHECK.md) |

コマンド:

```sh
pnpm install
sh scripts/check.sh          # すべての検査（Rust のテスト・fmt・clippy、画面のテスト・lint・UI チェック）
pnpm tauri dev               # 開発用に起動
pnpm tauri build --config src-tauri/tauri.local.conf.json \
  --config '{"bundle":{"macOS":{"signingIdentity":"-"}}}' # 個人の鍵を使わない確認用
```

Windows 用のインストーラーは Mac でも作れる（手順は [Windows の確認手順](WINDOWS-CHECK.md) の最後）。

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


## README の画像を作り直す

実際の React 画面と既存 IPC モックを Playwright で動かす。実際の授業・アカウント・保存ファイルには触れない。図は `docs/diagrams/` の HTML/CSS と、依存関係に含まれる React Icons から生成する。色は本体の `src/styles/tokens.css` を参照する。

```sh
pnpm exec playwright install chromium
node scripts/readme-assets.mjs               # スクショ・操作 GIF・図のすべて
node scripts/readme-assets.mjs screenshots   # スクショと GIF（ffmpeg が必要）
node scripts/readme-assets.mjs diagrams      # アイコン付きの図
```

Chromium が別の場所にある場合は `CHROMIUM_PATH` に実行ファイルのパスを指定する。スクショは既存 IPC モックの固定サンプルで、検出エンジンや OS 権限の実機検証を置き換えるものではない。出力は `docs/images/`（ライト / ダーク）。
