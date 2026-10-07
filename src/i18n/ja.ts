// Japanese strings. Wording for the rows design_brief.md's table covers is copied from it
// verbatim; everything else (picker/permission/settings helper copy) is written to match
// its tone. Keep this file's key set identical to en.ts (index.ts type-checks that).
export const ja = {
  "panel.title": "ZoomSlidesPicker",

  "action.start": "開始",
  "action.stop": "停止",
  "action.pause": "一時停止",
  "action.resume": "再開",
  "action.saveNow": "今すぐ保存",
  "action.undoLast": "直前の保存を取り消す",
  "action.openFolder": "保存先を開く",
  "action.rebuildPdf": "PDF を作り直す",
  "action.openSettings": "設定…",
  "action.back": "戻る",
  "action.change": "変更",
  "action.changeEllipsis": "変更…",
  "action.cancel": "取り消す",

  "panel.statusStopped": "停止中",
  "panel.statusRunning": "保存中：{count} 枚",
  "panel.statusPaused": "一時停止中：{count} 枚",

  "picker.heading": "授業の窓を選ぶ",
  "picker.loading": "窓を確認中…",
  "picker.empty": "撮れる窓がありません",

  "region.hint": "スライドの部分を囲んで、Enter で決定（Esc で取り消し）",
  "region.imageAlt": "{title} の静止画",
  "region.size": "{w} × {h} px",

  "permission.notice": "画面収録の許可が必要です。システム設定で許可してから、アプリを再起動してください。",
  "permission.openSettings": "システム設定を開く",
  "permission.check": "許可を確かめる",

  "notice.windowLost": "授業の窓が見つからないので、一時停止しました。",
  "notice.blank": "画面が 10 秒以上、単色のままです。共有の停止、キャプチャ防止、最小化の可能性があります。",

  "video.processing": "動画を処理中…",
  "video.progressKnown": "{done} / {total}",
  "video.progressUnknown": "{done}",

  "rebuild.result": "{count} ページで作り直しました。",

  "settings.title": "設定",
  "settings.general": "一般",
  "settings.capture": "キャプチャ",
  "settings.shortcuts": "ショートカット",
  "settings.language": "言語",
  "settings.langAuto": "自動",
  "settings.langJa": "日本語",
  "settings.langEn": "English",
  "settings.saveRoot": "保存先",
  "settings.interval": "比較の周期（秒）",
  "settings.intervalError": "比較の周期は 0.5〜5 秒、0.5 刻みで指定してください。",
  "settings.recording": "キーを押してください（Esc で取り消し）",
  "settings.shortcut.pauseResume": "一時停止と再開",
  "settings.saved": "保存しました",

  // Library (history) window -- docs/impl/library.md > Strings, copied verbatim, plus
  // library.slideCount (see en.ts for why it is not in that table).
  "library.search": "題名・日付・アプリ・メモで検索",
  "library.meta": "{date} · {duration} · {count} 枚",
  "library.recording": "記録中",
  "library.noPdf": "PDF なし",
  "library.hasPdf": "PDF あり",
  "library.source": "{app} の「{title}」",
  "library.rename": "名前を変更",
  "library.openFolder": "フォルダを開く",
  "library.openPdf": "PDF を開く",
  "library.rebuildPdf": "PDF を作り直す",
  "library.dedupe": "同じスライドをまとめる…",
  "library.select": "画像を選ぶ",
  "library.trashSelected": "選んだ画像をごみ箱へ（{count} 枚）",
  "library.cancelSelect": "やめる",
  "library.delete": "削除…",
  "library.open": "履歴…",
  "library.openButton": "履歴",
  "library.memo": "メモ",
  "library.memoPlaceholder": "授業の内容や課題など",
  "library.saved": "保存しました",
  "library.recordingLocked": "記録中のため、整理は停止してからできます。",
  "library.confirmDelete": "「{title}」をごみ箱に移しますか？ フォルダごと移します。ごみ箱から戻せます。",
  "library.confirmDeleteButton": "ごみ箱に移す",
  "library.confirmTrashImages": "{count} 枚をごみ箱に移して、PDF を作り直しますか？",
  "library.dedupeSelected": "同じスライドとみなした {count} 枚を選びました。確かめてから、ごみ箱に移してください。",
  "library.dedupeNone": "同じスライドは見つかりませんでした。",
  "library.pickTitle": "授業の窓を選ぶ",
  "library.empty": "まだ記録はありません。",
  "library.notFound": "記録が見つかりません。",
  "library.noPdfError": "PDF がありません。",
  "library.cancel": "やめる",
  "library.noHits": "見つかりませんでした。",
  "library.caption": "{index} · {time}",
  // SPEC-QUESTION: not in the strings table. The detail pane's meta line needs the
  // "{count} 枚" fragment on its own (it has a 4th segment, PDF status, after it, so it
  // cannot reuse library.meta's fixed template) -- text taken verbatim from that
  // template's own tail, not newly invented wording.
  "library.slideCount": "{count} 枚",
} as const;
