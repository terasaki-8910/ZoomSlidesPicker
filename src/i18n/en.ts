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

  "notice.windowLost": "Paused because the lecture window can't be found. It resumes when the window is back.",
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

  // Library (history) window -- docs/impl/library.md > Strings, copied verbatim, plus
  // library.slideCount (see ja.ts for why it is not in that table).
  "library.search": "Search by title, date, app, or memo",
  "library.meta": "{date} · {duration} · {count} slides",
  "library.recording": "Recording",
  "library.noPdf": "No PDF",
  "library.hasPdf": "PDF",
  "library.source": "“{title}” in {app}",
  "library.rename": "Rename",
  "library.openFolder": "Open folder",
  "library.openPdf": "Open PDF",
  "library.rebuildPdf": "Rebuild PDF",
  "library.dedupe": "Merge duplicate slides…",
  "library.select": "Select images",
  "library.trashSelected": "Move {count} to Trash",
  "library.cancelSelect": "Done",
  "library.delete": "Delete…",
  "library.open": "History…",
  "library.openButton": "History",
  "library.memo": "Memo",
  "library.memoPlaceholder": "Notes about the lecture, assignments, …",
  "library.saved": "Saved",
  "library.recordingLocked": "Stop recording to tidy up this session.",
  "library.confirmDelete": "Move “{title}” to the Trash? The whole folder moves; you can restore it from the Trash.",
  "library.confirmDeleteButton": "Move to Trash",
  "library.confirmTrashImages": "Move {count} images to the Trash and rebuild the PDF?",
  "library.dedupeSelected": "Selected {count} images that repeat a slide. Check them, then move them to the Trash.",
  "library.dedupeNone": "No repeated slides found.",
  "library.pickTitle": "Choose the lecture window",
  "library.empty": "No sessions yet.",
  "library.notFound": "Session not found.",
  "library.noPdfError": "There is no PDF.",
  "library.cancel": "Cancel",
  "library.noHits": "No matches.",
  "library.caption": "{index} · {time}",
  "library.slideCount": "{count} slides",
};
