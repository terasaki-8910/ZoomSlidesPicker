# README の図

承認された B 案（アイコン単体＋番号付き経路）の定義。画像生成 AI は使わず、Simple Icons / Tabler Icons の SVG と、ノード・境界枠・接続線を組み合わせる。

| ファイル | 内容 |
|---|---|
| `architecture.json` | 入力・IPC・Rust の処理・保存先・更新通信 |
| `stack.json` | 技術の層、利用関係、テスト対象 |
| `detection-flow.json` | 自動抽出の判断と分岐を簡略化したフロー |
| `tokens.json` | 図専用のライト／ダーク配色。本体のトークンとは別 |

```sh
node scripts/readme-diagrams.mjs
# または
node scripts/readme-assets.mjs diagrams
```

ルートから実行する。既存依存の Playwright Chromium が必要。出力は `docs/images/diagram-<名前>-<テーマ>.svg` と互換用の PNG。README は SVG を参照する。

`nodes` の `x` は中央、`y` はアイコン領域の上端。`kind: "decision"` は同じ座標規則のひし形。`groups` は境界枠、`edges` は接続元・先の ID、SVG の経路、ラベル位置を持つ。`dashed` は破線、`both` は双方向を示す。

ノードのロゴは `si:SiReact`、機能アイコンは `tb:TbCapture` のように指定する。接続の番号はラベルから自動で付き、実行順序を意味しない。フォントの実測値でラベルの背景幅を調整する。

描画時に ID の重複・未定義の接続先・未定義のアイコンを検査し、ブラウザで文字のはみ出し、ラベル同士とアイコンの重なり、アイコンと接続の数を検査する。出力画像は別途目視する。判定の詳細は `SPEC.md`「判定」、構成は各 crate と `src/ipc` に照合する。
