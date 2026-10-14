import { useCallback, useEffect, useMemo, useRef, useState, type ChangeEvent, type ReactNode } from "react";
import {
  FiCheckSquare,
  FiDownload,
  FiEdit2,
  FiFileText,
  FiFolder,
  FiLayers,
  FiMinusSquare,
  FiSettings,
  FiTrash2,
} from "react-icons/fi";
import Button from "../components/Button";
import ContextMenu, { type ContextMenuItem } from "../components/ContextMenu";
import ExportPdfDialog from "../components/ExportPdfDialog";
import UpdateDialog from "../components/UpdateDialog";
import IconButton from "../components/IconButton";
import Modal from "../components/Modal";
import Notice from "../components/Notice";
import WindowPicker from "../components/WindowPicker";
import { useLang } from "../i18n";
import type { TKey } from "../i18n";
import {
  foldSession,
  getAppState,
  getSession,
  listSessions,
  onAppState,
  getUpdateOffer,
  onUpdateOffer,
  onLibraryChanged,
  openCapture,
  openPermissionSettings,
  openSessionFolder,
  openSessionPdf,
  openSettings,
  pauseSession,
  renameSession,
  requestPermission,
  resumeSession,
  setSessionMemo,
  stopSession,
  thumbUrl,
  trashCaptures,
  trashSession,
} from "../ipc";
import type { AppState, Lang, PdfExport, SessionDetail, SessionSummary, UpdateInfo } from "../ipc/types";
import { formatBytes, formatDuration, formatElapsed, formatSessionRange, formatSessionStart, sessionDateSearchForms } from "../lib/format";
import "./Library.css";

type TFunc = (key: TKey, params?: Record<string, string | number>) => string;
type ConfirmKind = "delete" | "trashCaptures" | "trashOthers";
type ActionNotice = { tone: "warning" | "neutral"; text: string; showFolder?: boolean };
/** What the export dialog is open for: a whole session, or only some of its captures. */
type ExportTarget = { dir: string; date: string; indices: number[] | null };
type MenuState = { dir: string; x: number; y: number };

// Thumbnail widths passed to thumbUrl() (docs/impl/library.md > Types): generous enough
// for a retina display at each spot's rendered CSS size (64x36 and the grid tiles).
const LIST_THUMB_WIDTH = 128;
const TILE_THUMB_WIDTH = 360;

function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** Case-insensitive, space-separated AND; each term may match any of these fields
 * (docs/impl/library.md > Layout: title, source title, app, memo, date forms). */
function matchesQuery(session: SessionSummary, query: string): boolean {
  const terms = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return true;
  const haystacks = [session.customTitle, session.sourceTitle, session.sourceApp, session.memo, ...sessionDateSearchForms(session.startedAt)]
    .filter((v): v is string => v != null)
    .map((v) => v.toLowerCase());
  return terms.every((term) => haystacks.some((h) => h.includes(term)));
}

function sessionDurationSeconds(s: SessionSummary): number {
  const start = new Date(s.startedAt).getTime();
  // A still-recording session (endedAt null) has no fixed end yet -- its "so far"
  // duration legitimately grows in real time, the same as the toolbar's own elapsed
  // counter, so this is the one value in the list that is not frozen mock data.
  const end = s.endedAt ? new Date(s.endedAt).getTime() : Date.now();
  return Math.max(0, Math.round((end - start) / 1000));
}

function sessionMetaLine(s: SessionSummary, lang: Lang, t: TFunc): string {
  return t("library.meta", {
    date: formatSessionStart(s.startedAt, lang),
    duration: formatDuration(sessionDurationSeconds(s), lang),
    count: s.count,
  });
}

/** `記録中` while recording, else `PDF なし` when there is no PDF yet -- never both: a
 * still-recording session has no PDF either, but showing that as a second badge next to
 * 記録中 would be noise about a question (is there a PDF) that is not yet meaningful. */
function SessionBadge({ session, t }: { session: SessionSummary; t: TFunc }) {
  if (session.recording) return <span className="library-badge library-badge-accent">{t("library.recording")}</span>;
  if (!session.hasPdf) return <span className="library-badge">{t("library.noPdf")}</span>;
  return null;
}

type MenuAction = "folder" | "pdf" | "export" | "rename" | "trash";

/** The session-row context menu: recording sessions only get folder and rename. */
function SessionMenu({
  session,
  x,
  y,
  onAction,
  onClose,
}: {
  session: SessionSummary;
  x: number;
  y: number;
  onAction: (id: MenuAction) => void;
  onClose: () => void;
}) {
  const { t } = useLang();
  const item = (id: MenuAction, label: string, icon: ReactNode): ContextMenuItem => ({ id, label, icon, onSelect: () => onAction(id) });
  const items = [item("folder", t("library.openFolder"), <FiFolder />)];
  if (!session.recording && session.hasPdf) items.push(item("pdf", t("library.openPdf"), <FiFileText />));
  if (!session.recording && session.count > 0) items.push(item("export", t("library.exportPdf"), <FiDownload />));
  items.push(item("rename", t("library.rename"), <FiEdit2 />));
  if (!session.recording) items.push(item("trash", t("library.menuTrash"), <FiTrash2 />));
  return <ContextMenu x={x} y={y} label={t("library.menuLabel")} items={items} onClose={onClose} />;
}

export default function Library() {
  const { lang, t } = useLang();

  const [appState, setAppState] = useState<AppState | null>(null);
  const [sessions, setSessions] = useState<SessionSummary[] | null>(null);
  const [query, setQuery] = useState("");
  const [selectedDir, setSelectedDir] = useState<string | null>(null);
  const [detail, setDetail] = useState<SessionDetail | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [selectMode, setSelectMode] = useState(false);
  const [selectedCaptures, setSelectedCaptures] = useState<Set<number>>(new Set());
  const [renaming, setRenaming] = useState(false);
  const [renameValue, setRenameValue] = useState("");
  const [memoValue, setMemoValue] = useState("");
  const [memoSaved, setMemoSaved] = useState(false);
  const [confirmKind, setConfirmKind] = useState<ConfirmKind | null>(null);
  const [notice, setNotice] = useState<ActionNotice | null>(null);
  const [exportTarget, setExportTarget] = useState<ExportTarget | null>(null);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [updateOffer, setUpdateOffer] = useState<UpdateInfo | null>(null);

  const selectedDirRef = useRef<string | null>(null);
  const listRef = useRef<HTMLUListElement>(null);
  const lastClickedRef = useRef<number | null>(null);
  const renameRequestRef = useRef<string | null>(null);
  const renameInputRef = useRef<HTMLInputElement>(null);
  const suppressNextBlurRef = useRef(false);
  const memoSavedForRef = useRef<string | null>(null);
  const memoDebounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const memoFlashRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  // ---- app state (toolbar) -------------------------------------------------
  useEffect(() => {
    let active = true;
    getAppState().then((s) => {
      if (active) setAppState(s);
    });
    let unlisten: (() => void) | undefined;
    onAppState((s) => {
      if (active) setAppState(s);
    }).then((u) => {
      unlisten = u;
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  // ---- update offer (docs/impl/updates.md): the launch check's find, asked for on mount
  // and pushed by `update-offer` if it arrives later.
  useEffect(() => {
    let active = true;
    getUpdateOffer().then((info) => {
      if (active && info) setUpdateOffer(info);
    });
    let unlisten: (() => void) | undefined;
    onUpdateOffer((info) => {
      if (active) setUpdateOffer(info);
    }).then((u) => {
      unlisten = u;
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  // ---- sessions list + selection -------------------------------------------
  // Changes the selection and resets the other session-scoped UI state together, in
  // one place -- called from event handlers and promise callbacks only (never
  // synchronously inside a useEffect body, which react-hooks/set-state-in-effect
  // rightly flags as a cascading-render anti-pattern). selectedDirRef is updated here
  // too, synchronously, so promise callbacks elsewhere always read the latest value
  // instead of a stale one captured at effect-setup time.
  const selectDir = useCallback((dir: string | null) => {
    selectedDirRef.current = dir;
    setSelectedDir(dir);
    setSelectMode(false);
    setSelectedCaptures(new Set());
    lastClickedRef.current = null;
    setRenaming(false);
    setConfirmKind(null);
    setMenu(null);
    setNotice(null);
    setMemoSaved(false);
  }, []);

  const applySessions = useCallback(
    (list: SessionSummary[]) => {
      setSessions(list);
      const current = selectedDirRef.current;
      const next = current && list.some((s) => s.dir === current) ? current : (list[0]?.dir ?? null);
      if (next !== current) selectDir(next);
    },
    [selectDir]
  );

  useEffect(() => {
    let active = true;
    listSessions()
      .then((list) => {
        if (active) applySessions(list);
      })
      .catch((e) => {
        if (active) setNotice({ tone: "warning", text: errorMessage(e) });
      });
    return () => {
      active = false;
    };
  }, [applySessions]);

  // Refetches the list and (if the same session is still selected) its detail;
  // falls back to the first remaining session when the selection disappeared.
  const doRefetch = useCallback(async () => {
    let list: SessionSummary[];
    try {
      list = await listSessions();
    } catch (e) {
      setNotice({ tone: "warning", text: errorMessage(e) });
      return;
    }
    setSessions(list);
    const currentDir = selectedDirRef.current;
    if (currentDir == null || !list.some((s) => s.dir === currentDir)) {
      selectDir(list[0]?.dir ?? null);
      return;
    }
    try {
      setDetail(await getSession(currentDir));
    } catch {
      selectDir(null);
    }
  }, [selectDir]);

  // library-changed: refetch, coalesced to at most once per 300 ms (docs/impl/
  // library.md > Commands > Events).
  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout> | null = null;
    let pending = false;

    function run() {
      void doRefetch();
      timer = setTimeout(() => {
        timer = null;
        if (pending) {
          pending = false;
          run();
        }
      }, 300);
    }

    function onChanged() {
      if (!active) return;
      if (timer != null) {
        pending = true;
        return;
      }
      run();
    }

    let unlisten: (() => void) | undefined;
    onLibraryChanged(onChanged).then((u) => {
      unlisten = u;
    });
    return () => {
      active = false;
      if (timer) clearTimeout(timer);
      unlisten?.();
    };
  }, [doRefetch]);

  // Selection changed (via selectDir, which already reset the other session-scoped UI
  // state): load its detail. A stale `detail` from the previous selection is never
  // shown in the meantime -- renderBody() below only renders it once detail.dir
  // matches selectedDir -- so there is nothing to synchronously clear here.
  useEffect(() => {
    if (!selectedDir) return;
    let active = true;
    getSession(selectedDir)
      .then((d) => {
        if (!active) return;
        setDetail(d);
        memoSavedForRef.current = d.memo;
        setMemoValue(d.memo);
        if (renameRequestRef.current === d.dir) {
          // 名前を変更 chosen from the context menu of a row that was not selected yet.
          renameRequestRef.current = null;
          suppressNextBlurRef.current = false;
          setRenameValue(d.title);
          setRenaming(true);
        }
      })
      .catch((e) => {
        if (active) setNotice({ tone: "warning", text: errorMessage(e) });
      });
    return () => {
      active = false;
    };
  }, [selectedDir]);

  const filtered = useMemo(() => (sessions ?? []).filter((s) => matchesQuery(s, query)), [sessions, query]);

  // Listbox keyboard navigation (Up/Down/Home/End), as in the panel's picker -- but
  // never while typing in a text field, or while a modal (picker/confirm) is open.
  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (pickerOpen || confirmKind != null || exportTarget != null || menu != null) return;
      if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
      if (filtered.length === 0) return;
      const idx = filtered.findIndex((s) => s.dir === selectedDir);
      let next: number | null = null;
      if (e.key === "ArrowDown") next = idx === -1 ? 0 : Math.min(idx + 1, filtered.length - 1);
      else if (e.key === "ArrowUp") next = idx === -1 ? 0 : Math.max(idx - 1, 0);
      else if (e.key === "Home") next = 0;
      else if (e.key === "End") next = filtered.length - 1;
      if (next === null) return;
      e.preventDefault();
      selectDir(filtered[next].dir);
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [pickerOpen, confirmKind, exportTarget, menu, filtered, selectedDir, selectDir]);

  // ---- session row context menu ------------------------------------------
  const closeMenu = useCallback(() => {
    setMenu(null);
    listRef.current?.focus();
  }, []);

  function openRowMenu(dir: string, x: number, y: number) {
    if (dir !== selectedDirRef.current) selectDir(dir);
    setMenu({ dir, x, y });
  }

  function runMenuAction(id: MenuAction, target: SessionSummary) {
    const dir = target.dir;
    const fail = (e: unknown) => setNotice({ tone: "warning", text: errorMessage(e) });
    if (id === "folder") void openSessionFolder(dir).catch(fail);
    else if (id === "pdf") void openSessionPdf(dir).catch(fail);
    else if (id === "export") openExport(dir, target.startedAt, null);
    else if (id === "trash") setConfirmKind("delete");
    else if (detail && detail.dir === dir) startRename();
    else renameRequestRef.current = dir; // started once the row's detail has loaded
  }

  // ---- rename in place ------------------------------------------------------
  useEffect(() => {
    if (renaming) {
      renameInputRef.current?.focus();
      renameInputRef.current?.select();
    }
  }, [renaming]);

  function startRename() {
    if (!detail) return;
    // Starts each rename session with a clean slate: without this, a leftover `true`
    // from a previous Enter/Escape (if that unmount never actually fired a blur event
    // to consume it) would wrongly suppress a *later*, genuinely blur-triggered save.
    suppressNextBlurRef.current = false;
    setRenameValue(detail.title);
    setRenaming(true);
  }

  function exitRenaming() {
    suppressNextBlurRef.current = true;
    setRenaming(false);
  }

  function commitRename() {
    if (!detail) return;
    const trimmed = renameValue.trim();
    exitRenaming();
    void renameSession(detail.dir, trimmed === "" ? null : trimmed);
  }

  function cancelRename() {
    exitRenaming();
  }

  // ---- memo: 600ms debounce + save on blur, never the same content twice ---
  async function commitMemo(dir: string, value: string) {
    if (memoSavedForRef.current === value) return;
    memoSavedForRef.current = value;
    try {
      await setSessionMemo(dir, value);
      setMemoSaved(true);
      if (memoFlashRef.current) clearTimeout(memoFlashRef.current);
      memoFlashRef.current = setTimeout(() => setMemoSaved(false), 2000);
    } catch (e) {
      setNotice({ tone: "warning", text: errorMessage(e) });
    }
  }

  function handleMemoChange(e: ChangeEvent<HTMLTextAreaElement>) {
    const value = e.target.value;
    setMemoValue(value);
    if (!detail) return;
    const dir = detail.dir;
    if (memoDebounceRef.current) clearTimeout(memoDebounceRef.current);
    memoDebounceRef.current = setTimeout(() => void commitMemo(dir, value), 600);
  }

  function handleMemoBlur() {
    if (memoDebounceRef.current) {
      clearTimeout(memoDebounceRef.current);
      memoDebounceRef.current = null;
    }
    if (detail) void commitMemo(detail.dir, memoValue);
  }

  // ---- action row ------------------------------------------------------------
  async function handleOpenFolder() {
    if (!detail) return;
    try {
      await openSessionFolder(detail.dir);
    } catch (e) {
      setNotice({ tone: "warning", text: errorMessage(e) });
    }
  }

  async function handleOpenPdf() {
    if (!detail) return;
    try {
      await openSessionPdf(detail.dir);
    } catch (e) {
      setNotice({ tone: "warning", text: errorMessage(e) });
    }
  }

  function openExport(dir: string, startedAt: string, indices: number[] | null) {
    setExportTarget({ dir, date: startedAt.slice(0, 10), indices });
  }

  function handleExportDone(result: PdfExport) {
    setExportTarget(null);
    setNotice({
      tone: "neutral",
      text: t("export.done", { pages: result.pages, size: formatBytes(result.bytes, 1) }),
      showFolder: true,
    });
  }

  async function handleDedupe() {
    if (!detail) return;
    try {
      const indices = await foldSession(detail.dir);
      if (indices.length > 0) {
        setSelectMode(true);
        setSelectedCaptures(new Set(indices));
        setNotice({ tone: "neutral", text: t("library.dedupeSelected", { count: indices.length }) });
      } else {
        setNotice({ tone: "neutral", text: t("library.dedupeNone") });
      }
    } catch (e) {
      setNotice({ tone: "warning", text: errorMessage(e) });
    }
  }

  function handleEnterSelect() {
    setSelectMode(true);
    setSelectedCaptures(new Set());
    lastClickedRef.current = null;
    setNotice(null);
  }

  function handleExitSelect() {
    setSelectMode(false);
    setSelectedCaptures(new Set());
    lastClickedRef.current = null;
  }

  // Click toggles (Cmd/Ctrl+click too); Shift+click adds the range from the last clicked
  // tile to this one, both ends included.
  function handleTileClick(captures: SessionDetail["captures"], index: number, shift: boolean) {
    const last = lastClickedRef.current;
    lastClickedRef.current = index;
    setSelectedCaptures((prev) => {
      const next = new Set(prev);
      if (shift && last != null) {
        const from = captures.findIndex((c) => c.index === last);
        const to = captures.findIndex((c) => c.index === index);
        if (from !== -1 && to !== -1) {
          const [lo, hi] = from < to ? [from, to] : [to, from];
          for (let i = lo; i <= hi; i++) next.add(captures[i].index);
          return next;
        }
      }
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  }

  function handleToggleAll(captures: SessionDetail["captures"]) {
    lastClickedRef.current = null;
    setSelectedCaptures((prev) => (prev.size === captures.length ? new Set() : new Set(captures.map((c) => c.index))));
  }

  async function handleConfirmDeleteSession() {
    if (!detail || detail.dir !== selectedDir) return;
    setConfirmKind(null);
    try {
      await trashSession(detail.dir);
    } catch (e) {
      setNotice({ tone: "warning", text: errorMessage(e) });
    }
  }

  async function handleConfirmTrashCaptures(others: boolean) {
    if (!detail) return;
    const indices = others
      ? detail.captures.map((c) => c.index).filter((i) => !selectedCaptures.has(i))
      : [...selectedCaptures];
    setConfirmKind(null);
    try {
      await trashCaptures(detail.dir, indices);
      handleExitSelect();
    } catch (e) {
      setNotice({ tone: "warning", text: errorMessage(e) });
    }
  }

  async function handleRequestPermission() {
    await requestPermission();
    setAppState(await getAppState());
  }

  // ---- rendering --------------------------------------------------------------
  function renderToolbarStatus() {
    if (!appState) return null;
    const session = appState.session;
    if (!session) {
      return (
        <div className="library-toolbar-status">
          <span className="library-toolbar-state">{t("panel.statusStopped")}</span>
          <Button kind="primary" disabled={!appState.permission.granted} onClick={() => setPickerOpen(true)}>
            {t("action.start")}
          </Button>
        </div>
      );
    }
    if (session.stopping) {
      return (
        <div className="library-toolbar-status">
          <span className="library-toolbar-state">{t("panel.stopping")}</span>
        </div>
      );
    }
    const isPaused = session.state === "paused";
    return (
      <div className="library-toolbar-status">
        <span className="library-toolbar-state tabular-nums">
          {isPaused ? t("panel.statusPaused", { count: session.count }) : t("panel.statusRunning", { count: session.count })}
        </span>
        <span className="library-toolbar-elapsed tabular-nums">{formatElapsed(session.elapsedS)}</span>
        <Button kind="secondary" onClick={() => void (isPaused ? resumeSession() : pauseSession())}>
          {isPaused ? t("action.resume") : t("action.pause")}
        </Button>
        <Button kind="primary" onClick={() => void stopSession()}>
          {t("action.stop")}
        </Button>
      </div>
    );
  }

  function renderMenu() {
    if (!menu) return null;
    const target = sessions?.find((x) => x.dir === menu.dir);
    if (!target) return null;
    return <SessionMenu session={target} x={menu.x} y={menu.y} onAction={(id) => runMenuAction(id, target)} onClose={closeMenu} />;
  }

  function renderDetail(session: SessionDetail) {
    const allSelected = session.captures.length > 0 && selectedCaptures.size === session.captures.length;
    const othersCount = session.captures.filter((c) => !selectedCaptures.has(c.index)).length;
    const metaParts = [
      // No "{app} の「{title}」" segment for an mp4-mode session: there is no capturing
      // app to name (sourceApp is null), and the title above already shows the file name.
      session.sourceApp ? t("library.source", { app: session.sourceApp, title: session.sourceTitle }) : null,
      formatSessionRange(session.startedAt, session.endedAt, lang),
      t("library.slideCount", { count: session.count }),
      session.hasPdf ? t("library.hasPdf") : t("library.noPdf"),
    ].filter((v): v is string => v != null);

    return (
      <div className="library-detail-content">
        <div className="library-detail-head">
          {renaming ? (
            <input
              ref={renameInputRef}
              type="text"
              className="library-rename-input"
              value={renameValue}
              onChange={(e) => setRenameValue(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") {
                  e.preventDefault();
                  commitRename();
                } else if (e.key === "Escape") {
                  e.preventDefault();
                  cancelRename();
                }
              }}
              onBlur={() => {
                if (suppressNextBlurRef.current) {
                  suppressNextBlurRef.current = false;
                  return;
                }
                commitRename();
              }}
              aria-label={t("library.rename")}
            />
          ) : (
            <>
              <h1 className="library-detail-title">{session.title}</h1>
              <IconButton icon={<FiEdit2 aria-hidden="true" />} label={t("library.rename")} onClick={startRename} />
            </>
          )}
        </div>
        <p className="library-detail-meta tabular-nums">{metaParts.join(" · ")}</p>

        <div className="library-memo">
          <label htmlFor="library-memo-field" className="library-memo-label">
            {t("library.memo")}
          </label>
          <textarea
            id="library-memo-field"
            className="library-memo-field"
            placeholder={t("library.memoPlaceholder")}
            value={memoValue}
            onChange={handleMemoChange}
            onBlur={handleMemoBlur}
          />
          {memoSaved && (
            <span className="library-memo-saved" role="status">
              {t("library.saved")}
            </span>
          )}
        </div>

        {session.recording ? (
          <div className="library-detail-actions">
            <Button kind="secondary" onClick={() => void handleOpenFolder()}>
              <FiFolder aria-hidden="true" />
              {t("library.openFolder")}
            </Button>
          </div>
        ) : selectMode ? (
          <div className="library-detail-actions">
            <Button
              kind="primary"
              disabled={selectedCaptures.size === 0}
              onClick={() => openExport(session.dir, session.startedAt, [...selectedCaptures].sort((x, y) => x - y))}
            >
              <FiDownload aria-hidden="true" />
              {t("library.exportSelected")}
            </Button>
            <Button kind="secondary" disabled={selectedCaptures.size === 0} onClick={() => setConfirmKind("trashCaptures")}>
              <FiTrash2 aria-hidden="true" />
              {t("library.trashSelected", { count: selectedCaptures.size })}
            </Button>
            <Button
              kind="secondary"
              disabled={selectedCaptures.size === 0 || othersCount === 0}
              onClick={() => setConfirmKind("trashOthers")}
            >
              <FiMinusSquare aria-hidden="true" />
              {t("library.trashOthers", { count: othersCount })}
            </Button>
            <Button kind="secondary" onClick={() => handleToggleAll(session.captures)}>
              {allSelected ? t("library.clearSelection") : t("library.selectAll")}
            </Button>
            <Button kind="secondary" onClick={handleExitSelect}>
              {t("library.cancelSelect")}
            </Button>
          </div>
        ) : (
          <div className="library-detail-actions">
            <Button kind="secondary" onClick={() => void handleOpenFolder()}>
              <FiFolder aria-hidden="true" />
              {t("library.openFolder")}
            </Button>
            {session.hasPdf && (
              <Button kind="secondary" onClick={() => void handleOpenPdf()}>
                <FiFileText aria-hidden="true" />
                {t("library.openPdf")}
              </Button>
            )}
            <Button
              kind="secondary"
              disabled={session.captures.length === 0}
              onClick={() => openExport(session.dir, session.startedAt, null)}
            >
              <FiDownload aria-hidden="true" />
              {t("library.exportPdf")}
            </Button>
            <Button kind="secondary" onClick={() => void handleDedupe()}>
              <FiLayers aria-hidden="true" />
              {t("library.dedupe")}
            </Button>
            <Button kind="secondary" onClick={handleEnterSelect}>
              <FiCheckSquare aria-hidden="true" />
              {t("library.select")}
            </Button>
            <Button kind="secondary" className="library-btn-danger" onClick={() => setConfirmKind("delete")}>
              <FiTrash2 aria-hidden="true" />
              {t("library.delete")}
            </Button>
          </div>
        )}

        {session.recording && <p className="library-recording-note">{t("library.recordingLocked")}</p>}

        {notice && (
          <Notice tone={notice.tone}>
            {notice.text}
            {notice.showFolder && (
              <>
                {" "}
                <button type="button" className="library-notice-link" onClick={() => void handleOpenFolder()}>
                  {t("export.showInFolder")}
                </button>
              </>
            )}
          </Notice>
        )}

        <div className="library-grid">
          {session.captures.map((c) => {
            const caption = t("library.caption", { index: String(c.index).padStart(3, "0"), time: formatElapsed(c.tContent) });
            const img = <img src={thumbUrl(c.path, TILE_THUMB_WIDTH)} alt="" loading="lazy" className="library-tile-img" />;
            const captionEl = (
              <span className="library-tile-caption tabular-nums">{caption}</span>
            );
            return selectMode ? (
              // The checkbox sits in the caption row, never over the image: a capture's
              // top-left corner is usually its slide title.
              <label key={c.index} className="library-tile library-tile-selectable">
                {img}
                <span className="library-tile-caption-row">
                  {/* onClick, not onChange: only the click event carries Shift / Cmd / Ctrl. */}
                  <input
                    type="checkbox"
                    className="library-tile-check"
                    checked={selectedCaptures.has(c.index)}
                    onChange={() => {}}
                    onClick={(e) => handleTileClick(session.captures, c.index, e.shiftKey)}
                  />
                  {captionEl}
                </span>
              </label>
            ) : (
              <button key={c.index} type="button" className="library-tile library-tile-button" onClick={() => void openCapture(c.path)}>
                {img}
                {captionEl}
              </button>
            );
          })}
        </div>
      </div>
    );
  }

  function renderBody() {
    if (sessions === null) return <div className="library-body" />;
    if (sessions.length === 0) {
      return (
        <div className="library-empty-state">
          <p>{t("library.empty")}</p>
          <Button kind="primary" disabled={!appState?.permission.granted} onClick={() => setPickerOpen(true)}>
            {t("action.start")}
          </Button>
        </div>
      );
    }
    return (
      <div className="library-body">
        <div className="library-list-col">
          <div className="library-search-wrap">
            <label htmlFor="library-search" id="library-search-label" className="visually-hidden">
              {t("library.search")}
            </label>
            <input
              id="library-search"
              type="search"
              className="library-search"
              placeholder={t("library.search")}
              value={query}
              onChange={(e) => setQuery(e.target.value)}
            />
          </div>
          {filtered.length === 0 ? (
            <p className="library-empty-hint">{t("library.noHits")}</p>
          ) : (
            <ul
              ref={listRef}
              role="listbox"
              aria-labelledby="library-search-label"
              aria-activedescendant={selectedDir ? `library-option-${selectedDir}` : undefined}
              tabIndex={0}
              className="library-list"
              onKeyDown={(e) => {
                // The context-menu key / Shift+F10 opens the menu for the active row.
                if (e.key !== "ContextMenu" && !(e.key === "F10" && e.shiftKey)) return;
                const row = selectedDir ? document.getElementById(`library-option-${selectedDir}`) : null;
                if (!selectedDir || !row) return;
                e.preventDefault();
                const r = row.getBoundingClientRect();
                openRowMenu(selectedDir, r.left + 48, r.bottom - 4);
              }}
            >
              {filtered.map((s) => (
                // Keyboard interaction is handled globally above (Arrow/Home/End) via
                // aria-activedescendant, the same listbox pattern as the window picker;
                // the click handler is the mouse equivalent, so no key handler is needed.
                // eslint-disable-next-line jsx-a11y/click-events-have-key-events
                <li
                  key={s.dir}
                  id={`library-option-${s.dir}`}
                  role="option"
                  aria-selected={s.dir === selectedDir}
                  className={s.dir === selectedDir ? "library-row is-selected" : "library-row"}
                  onClick={() => selectDir(s.dir)}
                  onContextMenu={(e) => {
                    e.preventDefault();
                    openRowMenu(s.dir, e.clientX, e.clientY);
                  }}
                >
                  {s.thumbPath ? (
                    <img src={thumbUrl(s.thumbPath, LIST_THUMB_WIDTH)} alt="" loading="lazy" className="library-row-thumb" />
                  ) : (
                    <span className="library-row-thumb" />
                  )}
                  <span className="library-row-main">
                    <span className="library-row-title">{s.title}</span>
                    <span className="library-row-meta tabular-nums">
                      {sessionMetaLine(s, lang, t)}
                      <SessionBadge session={s} t={t} />
                    </span>
                  </span>
                </li>
              ))}
            </ul>
          )}
        </div>
        <div className="library-detail-pane">{detail && detail.dir === selectedDir && renderDetail(detail)}</div>
      </div>
    );
  }

  return (
    <div className="library-root" data-testid="library-root">
      <header className="library-toolbar">
        <span className="library-toolbar-title">{t("panel.title")}</span>
        <div className="library-toolbar-right">
          {renderToolbarStatus()}
          <IconButton icon={<FiSettings aria-hidden="true" />} label={t("action.openSettings")} onClick={() => void openSettings()} />
        </div>
      </header>

      {appState && !appState.permission.granted && (
        <div className="library-permission-notice">
          <Notice>{t("permission.notice")}</Notice>
          <div className="library-permission-actions">
            <Button kind="secondary" onClick={() => void openPermissionSettings()}>
              {t("permission.openSettings")}
            </Button>
            <Button kind="secondary" onClick={() => void handleRequestPermission()}>
              {t("permission.check")}
            </Button>
          </div>
        </div>
      )}

      {renderBody()}

      {pickerOpen && (
        <Modal titleId="window-picker-heading" onClose={() => setPickerOpen(false)}>
          <WindowPicker onPicked={() => setPickerOpen(false)} onCancel={() => setPickerOpen(false)} />
        </Modal>
      )}

      {confirmKind === "delete" && detail && detail.dir === selectedDir && (
        <Modal titleId="library-confirm-delete-text" onClose={() => setConfirmKind(null)}>
          <p id="library-confirm-delete-text" className="library-confirm-message">
            {t("library.confirmDelete", { title: detail.title })}
          </p>
          <div className="library-confirm-actions">
            <Button kind="primary" className="library-btn-confirm-danger" onClick={() => void handleConfirmDeleteSession()}>
              {t("library.confirmDeleteButton")}
            </Button>
            <Button kind="secondary" onClick={() => setConfirmKind(null)}>
              {t("library.cancel")}
            </Button>
          </div>
        </Modal>
      )}

      {confirmKind === "trashCaptures" && (
        <Modal titleId="library-confirm-trash-text" onClose={() => setConfirmKind(null)}>
          <p id="library-confirm-trash-text" className="library-confirm-message">
            {t("library.confirmTrashImages", { count: selectedCaptures.size })}
          </p>
          <div className="library-confirm-actions">
            <Button kind="primary" className="library-btn-confirm-danger" onClick={() => void handleConfirmTrashCaptures(false)}>
              {t("library.confirmDeleteButton")}
            </Button>
            <Button kind="secondary" onClick={() => setConfirmKind(null)}>
              {t("library.cancel")}
            </Button>
          </div>
        </Modal>
      )}

      {confirmKind === "trashOthers" && detail && (
        <Modal titleId="library-confirm-others-text" onClose={() => setConfirmKind(null)}>
          <p id="library-confirm-others-text" className="library-confirm-message">
            {t("library.confirmTrashOthers", { count: detail.captures.filter((c) => !selectedCaptures.has(c.index)).length })}
          </p>
          <div className="library-confirm-actions">
            <Button kind="primary" className="library-btn-confirm-danger" onClick={() => void handleConfirmTrashCaptures(true)}>
              {t("library.confirmDeleteButton")}
            </Button>
            <Button kind="secondary" onClick={() => setConfirmKind(null)}>
              {t("library.cancel")}
            </Button>
          </div>
        </Modal>
      )}

      {exportTarget && (
        <ExportPdfDialog
          dir={exportTarget.dir}
          date={exportTarget.date}
          indices={exportTarget.indices}
          onClose={() => setExportTarget(null)}
          onDone={handleExportDone}
        />
      )}

      {updateOffer && <UpdateDialog info={updateOffer} answerLater onClose={() => setUpdateOffer(null)} />}

      {renderMenu()}
    </div>
  );
}
