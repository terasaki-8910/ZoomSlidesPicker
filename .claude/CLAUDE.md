## サブエージェントへの委任

この節はメインの会話のエージェント向け。サブエージェントとして起動している場合は適用しない。

- 設計判断、原因の調査、仕様の解釈など、判断や専門知識が必要な作業は委任せず、自分で行う。
- 委任するのは、指示文だけで完結し、出力が多いか手順が機械的で、結果を差分やコマンドで確かめられる作業に限る。例：テストを実行して失敗だけを要約する、ログを集計する、広い範囲を検索する、仕様が決まっている定型的な書き換えをする。
- ツール操作1〜2回で終わる作業は委任しない。
- 委任するときは、目的、対象の範囲、完了条件、報告に含める内容（変更したファイル、実行したコマンドと結果、確認できていない点）を指示に書く。判断が必要な箇所に当たったら、推測で進めずに止めて報告するよう伝える。
- 報告はそのまま採用しない。ファイルが変更されていれば git diff を自分で読み、必要に応じてテストやビルドを実行する。調査結果は、結論の根拠になる箇所を自分で開いて確かめる。
- 問題があれば自分で修正する。

## Confirm before building
- Choices that are mine -- which reading of an ambiguous request is meant,
  anything I will see (layout, visuals, wording), scope, tradeoffs with no
  clear default, conflicts between my instructions -- are not yours to
  settle. Show me your understanding as something I can look at (a decision
  sheet with real samples, or a local test page) and build only after I say
  GO. A task doc that looks complete is not a GO.
- One sheet per human gate (see Process), with every open question on it:
  results waiting for acceptance, samples for the next task, wording with
  drafts, things found outside the task, and what happens after the build
  ("merge if checks pass?"). Questions must work in any combination of
  answers; save a question for a later round only if it can't be answered
  before something is built.
- Don't ask what a test can decide, what has a conventional default, or what
  is cheap to undo -- decide it and say so in one line.
- A project's decisions record outranks task docs and memory, and what isn't
  in it is undecided. Append my answers to it as I wrote them.
- How to build and read the sheet: the `decision-sheet` skill.
- The decisions record is `DECISIONS.md` at the repository root.
