// English strings. Wording for the rows design_brief.md's table covers is copied from
// it verbatim; everything else is invented to match its tone (the brief only gives
// Japanese for those). The `Record<keyof typeof ja, string>` annotation makes a missing
// or extra key a type error, so this file's keys cannot drift from ja.ts.
import type { ja } from "./ja";

export const en: Record<keyof typeof ja, string> = {
  "panel.title": "ZoomSlidesPicker",

  "action.start": "Start",
  "action.stop": "Stop",
  "action.pause": "Pause",
  "action.resume": "Resume",
  "action.saveNow": "Save now",
  "action.undoLast": "Undo last save",
  "action.openFolder": "Open save folder",
  "action.rebuildPdf": "Rebuild PDF",
  "action.openSettings": "Settings…",
  "action.back": "Back",
  "action.change": "Change",
  "action.changeEllipsis": "Change…",
  "action.cancel": "Cancel",

  "panel.statusStopped": "Stopped",
  "panel.statusRunning": "Capturing: {count} slides",
  "panel.statusPaused": "Paused: {count} slides",

  "picker.heading": "Choose the lecture window",
  "picker.loading": "Checking windows…",
  "picker.empty": "No capturable windows found.",

  "region.hint": "Drag around the slide, then press Enter (Esc to cancel)",
  "region.imageAlt": "Still image of {title}",
  "region.size": "{w} × {h} px",

  "permission.notice": "Screen recording permission is needed. Allow it in System Settings, then restart the app.",
  "permission.openSettings": "Open System Settings",
  "permission.check": "Check permission",

  "notice.windowLost": "Paused because the lecture window can't be found.",
  "notice.blank":
    "The screen has been a single colour for over 10 seconds. Sharing may have stopped, capture may be blocked, or the window may be minimized.",

  "video.processing": "Processing video…",
  "video.progressKnown": "{done} / {total}",
  "video.progressUnknown": "{done}",

  "rebuild.result": "Rebuilt with {count} pages.",

  "settings.title": "Settings",
  "settings.general": "General",
  "settings.capture": "Capture",
  "settings.shortcuts": "Shortcuts",
  "settings.language": "Language",
  "settings.langAuto": "Auto",
  "settings.langJa": "日本語",
  "settings.langEn": "English",
  "settings.saveRoot": "Save location",
  "settings.interval": "Check interval (seconds)",
  "settings.intervalError": "The check interval must be between 0.5 and 5 seconds, in steps of 0.5.",
  "settings.recording": "Press the new keys (Esc to cancel)",
  "settings.shortcut.pauseResume": "Pause and Resume",
  "settings.saved": "Saved.",
};
