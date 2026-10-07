# Windows での確認（ACCEPTANCE D2）

目的：Windows でアプリが動き、ほかの窓が重なっていても授業の窓の中身を撮れるかを確かめる。Mac の D1 と同じ台本のページ（`stage.html`）と、重ねる赤い窓（`occluder.html`）を使う。

送ったもの（zip）：

- `ZoomSlidesPicker_0.1.4_x64-setup.exe`：インストーラー（Mac で作った Windows 用。署名なし）
- `stage.html`、`occluder.html`：台本のページと重ねる窓
- この手順書

zip の中の 4 つのファイルを `C:\zsp` フォルダに置く前提で書いています（`C:\zsp\stage.html` になるように。ほかの場所なら、下のパスを読み替えてください）。

## 1. 入れて起動する

1. `ZoomSlidesPicker_0.1.4_x64-setup.exe` を実行する（前の版を入れていれば、そのまま上書きされる）。署名がないので「Windows によって PC が保護されました」と出たら、「詳細情報」を押してから「実行」。管理者の権限は要らない（このユーザーにだけ入る）。
2. スタートメニューから ZoomSlidesPicker を起動する。履歴の窓が開く（0.1.3 から）。
3. 通知領域（時計の近く）にアイコンが出る。青い四角に白いスライドの形。見えなければタスクバーの「^」の中にある。タスクバーにドラッグして出しておくと使いやすい。

確かめること：

- 履歴の窓に、これまでの記録が新しい順に並ぶ（初めてなら「まだ記録はありません。」）。記録を選ぶと、右にスライドの小さな画像が並ぶ（画像が出ず枠だけなら知らせてください）。窓を閉じても、通知領域のアイコンは残る。
- 左クリックでパネル（「停止中」と「開始」）が出る。パネルの外を押すと閉じる。右上に履歴のボタンがある。
- 右クリックでメニュー（履歴…、開始…、停止、保存先を開く、PDF を作り直す、動画ファイルから抽出…、設定…、終了）が出る。
- タスクバーがダーク（設定 > 個人用設定 > 色 > Windows モード）でもライトでも、アイコンが見える。

## 2. 台本のページを撮る（重なりなし）

1. PowerShell で台本のページを開く（Chrome の app ウィンドウ。題名が `ZSP-STAGE-SPIKE` になる）。

   ```powershell
   & "C:\Program Files\Google\Chrome\Application\chrome.exe" --user-data-dir=C:\zsp\profile-stage --no-first-run --no-default-browser-check --app=file:///C:/zsp/stage.html --window-size=1000,700 --window-position=100,120
   ```

2. 履歴の窓の上の「開始」（またはアイコンを左クリックしてパネルの「開始」）→ 一覧から `ZSP-STAGE-SPIKE` を選ぶ → 範囲を囲まずに Enter（窓全体）。
3. 台本はページを開いた時点から動くので、台本の窓をクリックして F5 で読み込み直す（6 秒にスライド 2、12 秒にスライド 3 と動画の枠、24 秒にスライド 1 に戻る、30 秒にスライド 4）。
4. 40 秒ほど待って、パネルの「停止」。停止の通知が出る。
5. パネルの「保存先を開く」で保存先（ピクチャ\ZoomSlidesPicker\日時）を開く。

合格：スライド 1、2、3、4 の画像が 1 枚ずつ（スライド 3 は動画が止まった後の版がもう 1 枚あってもよい）、`slides.pdf` と `session.json` がある。スライド 1 に戻った所は、新しい画像にならない。履歴の窓の一覧にこの記録が出て、選ぶと同じ画像が並ぶ。

## 3. 重なっている窓を撮る

1. 2 と同じく、台本のページを開いて「開始」し、台本の窓で F5 を押す。
2. すぐに PowerShell で赤い窓を重ねる。

   ```powershell
   & "C:\Program Files\Google\Chrome\Application\chrome.exe" --user-data-dir=C:\zsp\profile-occluder --no-first-run --no-default-browser-check --app=file:///C:/zsp/occluder.html --window-size=900,600 --window-position=150,170
   ```

3. 重ねたまま 40 秒ほど待って「停止」。

合格：画像に写っているのは台本のスライドで、赤い `OCCLUDER` の窓は写っていない。

## 4. うまくいかないとき

- 画像が黒い、または灰色だけ、または「画面が 10 秒以上、単色のままです」と通知が出る：Windows の標準の取り込み方式で、この窓を撮れていない。知らせてください。別の方式（Windows.Graphics.Capture）の版を作ります。
- 窓の一覧に `ZSP-STAGE-SPIKE` が出ない、アイコンが出ない、エラーの文が出る：その文と状況を知らせてください。

## 5. 実際の授業で

- Zoom は窓のまま使う（全画面表示にはしない）。録画はブラウザの専用の窓で開き、再生中はマウスを動画の外に置く。
- 範囲は、スライドの部分だけをドラッグで囲む（Zoom の操作バーや参加者の映像は外す）。
- ショートカット：Ctrl+Alt+Shift+S（今すぐ保存）、Ctrl+Alt+Shift+Z（直前の保存の取り消し）、Ctrl+Alt+Shift+P（一時停止と再開）。
- 授業の窓を最小化すると、一時停止して通知が出る。戻したら、パネルの「再開」。
- 終わったら「停止」。PDF ができると通知が出る。保存した枚数は、アイコンにマウスを乗せると出る。

## 6. 知らせてほしいこと

- Windows のバージョン（設定 > システム > バージョン情報）と、表示の拡大率（設定 > システム > ディスプレイ）
- 1〜5 の各項目：できた、できなかった（できなかったときは、出た文や見えたもの）
- 2 と 3 の保存先の画像の枚数と、黒くなかったか（画像そのものを送る必要はない）

## 7. 消すとき

設定 > アプリ > インストールされているアプリ > ZoomSlidesPicker > アンインストール。保存した画像は消えない。

## このインストーラーの作り方（Mac、開発の控え）

```sh
brew install nsis llvm
cargo install --locked cargo-xwin
rustup target add x86_64-pc-windows-msvc
XWIN_CACHE_DIR="$HOME/Library/Caches/cargo-xwin" PATH="/opt/homebrew/opt/llvm/bin:$PATH" \
  pnpm tauri build --runner cargo-xwin --target x86_64-pc-windows-msvc
# -> target/x86_64-pc-windows-msvc/release/bundle/nsis/ZoomSlidesPicker_<版>_x64-setup.exe
```

cargo-xwin は初回に Microsoft の Windows SDK と CRT をダウンロードする（Microsoft のライセンスの下で）。インストーラーは NSIS だけ（MSI は Windows でないと作れない）。署名はしていない。
