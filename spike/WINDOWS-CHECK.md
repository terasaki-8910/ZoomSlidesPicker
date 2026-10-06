# Windows 実機での窓キャプチャ確認（ラウンド 1 の R1-2 で「実機がある」を選んだ後の確認）

目的: xcap 0.9.8 の既定の窓キャプチャが、他の窓に重ねられていても窓の中身を撮れるかを確かめる。Mac で行った確認と同じ台本のページを使う。

結果は Mac と比べられるように、同じ台本（`fixtures/stage.html`）と同じ重なり（`fixtures/occluder.html`）で行う。

## 1. 準備

- Rust（rustup-init を実行。既定の MSVC ツールチェーン）
- Visual Studio Build Tools（「C++ によるデスクトップ開発」）
- Google Chrome

このブランチ（`intake/r1-spike`）は Mac の作業フォルダにだけあり、push していない。次の 2 つを Windows の作業フォルダにコピーする。

- `spike/xcap-capture-spike/`（`target/` は除く）
- `spike/fixtures/`

## 2. ビルドと一覧

```
cd xcap-capture-spike
cargo build --release
target\release\xcap-capture-spike.exe list
```

`list` の `monitor` 行（x, y, w, h, scale）と、`ZSP-` で始まる窓の行を控える。

## 3. 検証 A: 重なりなしで窓の中身を撮る

`fixtures\stage.html` を Chrome の app ウィンドウで開く（パスは `file:///` で始め、区切りは `/`）。

```
"C:\Program Files\Google\Chrome\Application\chrome.exe" --user-data-dir=C:\zsp\profile-stage --app=file:///C:/path/to/fixtures/stage.html --window-size=1000,700 --window-position=100,120
```

ウィンドウのタイトルが `ZSP-STAGE-SPIKE` になってから、すぐに次を実行する（台本はページを開いた時点から始まる）。

```
target\release\xcap-capture-spike.exe window "ZSP-STAGE" 40 500 out-window > window.csv
```

`out-window` の PNG を開き、スライドが切り替わっているか確かめる。台本は 6 秒にスライド 2、12 秒にスライド 3、12.5〜22.5 秒に動画の枠が動く、24 秒にスライド 1、30 秒にスライド 4。

## 4. 検証 B: 重なっている窓の中身を撮る

同じ位置に `fixtures\occluder.html` を重ねる（赤い窓）。

```
"C:\Program Files\Google\Chrome\Application\chrome.exe" --user-data-dir=C:\zsp\profile-occluder --app=file:///C:/path/to/fixtures/occluder.html --window-size=520,360 --window-position=420,300
```

重ねたままで次を実行する。

```
target\release\xcap-capture-spike.exe window "ZSP-STAGE" 10 500 out-window-occluded
target\release\xcap-capture-spike.exe region 100 120 1000 700 2 500 out-region
```

- `out-window-occluded` は、重なりがあってもスライドの内容が写るはず。
- `out-region` は、重なった赤い `OCCLUDER` が写るはず（画面の一部を撮るとこうなる）。

座標（100, 120, 1000, 700）は、拡大率（DPI）が 100% でない場合に合わなくなる。その場合は `list` の値（窓の x, y, w, h）をそのまま使う。

## 5. 報告してほしいこと

- `list` の出力（monitor 行と ZSP 窓の行）
- `window.csv` と各 `*_summary.txt` の `capture_ms` の平均と p95
- `out-window`、`out-window-occluded`、`out-region` の PNG で、次のどちらか
  - スライドの内容が見えるか（検証 A、検証 B）
  - 赤い `OCCLUDER` が写るか（`out-region`）
- Windows のバージョンと拡大率（設定 > システム > ディスプレイ）

## 6. 結果が悪い場合

窓が黒い、またはスライドの内容が写らない場合は、既定の方式では GPU で描いた窓を撮れていない可能性がある。次の段階は、xcap の `wgc` 機能（Windows.Graphics.Capture）との比較。これはまだ実装していない。
