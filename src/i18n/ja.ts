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
} as const;
