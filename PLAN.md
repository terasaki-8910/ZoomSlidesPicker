# PLAN — 作る順番と状態

仕様は SPEC.md、合格の条件は ACCEPTANCE.md、判断は DECISIONS.md。ここには作る順番と状態だけを書く。

## 進め方

- 設計、API、テスト（ACCEPTANCE をコードにしたもの）は Opus が書く。実装は Sonnet のサブエージェントが、`.worktrees/<名前>` の作業ツリーで行う。
- 取り込む前に Opus が差分を読み、`sh scripts/check.sh` の該当部分を自分で実行してから `git merge --no-ff` する。テストを弱めたり消したりした変更は取り込まない。
- API とテストは先に main に入っている。実装者はそれを変えない（変える必要があれば止めて報告する）。

## 部品と順番

| 順 | 部品 | 中身 | 合格の判定 | 担当 | 状態 |
|---|---|---|---|---|---|
| 1 | `crates/zsp-detect` | 解析画像、判定（参照実装 `samples/proto/detect.py` の移植）、今すぐ保存・取り消し・一時停止 | A1〜A14、B1（`cargo test -p zsp-detect`、A11 は `--release --test timing`） | Sonnet | 完了（13 件と A11 合格、c5f9f33） |
| 1 | `crates/zsp-output` | PNG、session.json、PDF、ごみ箱、PDF の作り直し | B2〜B7、B9（`cargo test -p zsp-output`） | Sonnet | 完了（8 件合格、c05ffb4） |
| 1 | `crates/zsp-capture` | 窓の一覧とキャプチャ（xcap）、範囲、mp4（ffmpeg）、権限 | B8 と範囲の計算（`cargo test -p zsp-capture`） | Sonnet | 取り込み済み（c00af08）。B8 も合格 |
| 1 | フロントエンド | パネル、範囲、設定、言語、トークン、モック | C1〜C9（`pnpm test`、`pnpm lint`、`sh scripts/ui-check.sh`） | Sonnet | 完了（26 件、UI チェック 17 件合格、c910d53） |
| 2 | `crates/zsp-session` | 1〜3 をつなぐ。保留の画像、操作、窓を見失ったとき | `cargo test -p zsp-session` | Sonnet + Opus の修正 | 完了（13 件合格、37c6fa5） |
| 3 | `src-tauri` | 常駐アイコン、窓、ショートカット、通知、言語、設定の保存、`docs/impl/ipc.md` のコマンド | ビルドと起動、D1、D3 | Sonnet + Opus の修正 | 完了（7f2b2b9 ほか）。Opus の修正：トレイ更新のデッドロック、パネルの高さ、終了処理、Windows のアイコン、再起動でパネルを開く |
| 4 | 仕上げ | 実機の確認（D1、D3）、README、配布物のビルド | D、E の準備 | Opus | D1、D3 合格（下）。README 更新済み |

1 の 4 つは、互いのファイルに触らないので並行して作る。

## 実機の確認の結果（Mac、2026-10-07）

- D1：合格。`spike/fixtures/stage.html` を窓指定で 40 秒撮り、スライド 1〜4 を 1 枚ずつ保存。スライド 1 に戻った所は保存せず、戻った時刻だけ記録。別の窓で全体を覆っても、中身だけが写る。
- 撮影中に窓を閉じた場合：xcap はエラーを返す（空の画像ではない）。一時停止し、`window_lost` を記録し、終了時に PDF を書く。
- D3：合格（リリースビルド、窓全体 2000 x 1400 px、周期 0.5 秒、10 分）。メモリは最大 118.6 MiB、1 分後から 10 分後の増加は -18.7 MiB、CPU 平均は 4.56%。余裕は小さい。macOS 26 では窓の取り込みを OS の replayd が代行し、そちらが別に約 1.1〜1.2% 使う（D3 の対象外）。実際の使い方では範囲を絞るので、解析の負荷はこれより小さい。
- Windows：`cargo check` と clippy は Windows 向けに通る（`x86_64-pc-windows-msvc`、リソースコンパイラは代わりのスクリプト）。実機では未確認（D2）。

## 人の確認（最後にまとめて）

- 見た目（デザイントークンと画面）: DECISIONS の「進め方の変更」のとおり、最後に画面を見せて確かめる。
- E1〜E3（実際の授業で使う）、D2（Windows）、D4（Mac の全画面表示）。
