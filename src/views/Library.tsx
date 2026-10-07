import { useEffect, useMemo, useState } from "react";
import { FiEdit2, FiSettings, FiTrash2 } from "react-icons/fi";
import Button from "../components/Button";
import IconButton from "../components/IconButton";
import { useLang } from "../i18n";
import type { TKey } from "../i18n";
import { getAppState, getSession, listSessions, onAppState, openSettings } from "../ipc";
import type { AppState, Lang, SessionDetail, SessionSummary } from "../ipc/types";
import { formatDuration, formatElapsed, formatSessionRange, formatSessionStart, sessionDateSearchForms } from "../lib/format";
import "./Library.css";

type TFunc = (key: TKey, params?: Record<string, string | number>) => string;

// Prototype scope (docs/impl/library.md): running on the IPC mock only, not wired to
// Rust. Every action button below other than the ones that only change local UI state
// (search, list/listbox selection, select-mode entry/exit, layout B's open/back) is
// visual only and calls nothing -- same treatment as the select-mode actions the spec
// itself calls out as "no-op mock functions", extended consistently to the rest of the
// action row and the toolbar's start/pause/resume/stop so the prototype does not mix
// some wired controls with others that only look wired.

function layoutFromLocation(): "split" | "list" {
  if (typeof location === "undefined") return "split";
  return new URLSearchParams(location.search).get("layout") === "list" ? "list" : "split";
}

/** Case-insensitive, space-separated AND; each term may match any of these fields. */
function matchesQuery(session: SessionSummary, query: string): boolean {
  const terms = query.trim().toLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return true;
  const haystacks = [session.customTitle, session.sourceTitle, session.sourceApp, ...sessionDateSearchForms(session.startedAt)]
    .filter((v): v is string => v != null)
    .map((v) => v.toLowerCase());
  return terms.every((term) => haystacks.some((h) => h.includes(term)));
}

function sessionDurationSeconds(s: SessionSummary): number {
  const start = new Date(s.startedAt).getTime();
  // A still-recording session (endedAt null) has no fixed end yet -- its "so far"
  // duration legitimately grows in real time, the same as the toolbar's own elapsed
  // counter, so this is the one value in the list that is not frozen mock data and can
  // read a little differently between runs. Harmless: these screenshots are for human
  // review on a decision sheet, not pixel-diffed.
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

interface DetailPaneProps {
  t: TFunc;
  lang: Lang;
  session: SessionDetail;
  selectMode: boolean;
  selectedCaptures: Set<number>;
  onEnterSelect: () => void;
  onExitSelect: () => void;
  onToggleCapture: (index: number) => void;
  onTrashSelected: () => void;
}

function SessionDetailPane({
  t,
  lang,
  session,
  selectMode,
  selectedCaptures,
  onEnterSelect,
  onExitSelect,
  onToggleCapture,
  onTrashSelected,
}: DetailPaneProps) {
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
        <h1 className="library-detail-title">{session.title}</h1>
        <IconButton icon={<FiEdit2 aria-hidden="true" />} label={t("library.rename")} />
      </div>
      <p className="library-detail-meta tabular-nums">{metaParts.join(" · ")}</p>

      {selectMode ? (
        <div className="library-detail-actions">
          <Button kind="primary" disabled={selectedCaptures.size === 0} onClick={onTrashSelected}>
            {t("library.trashSelected", { count: selectedCaptures.size })}
          </Button>
          <Button kind="secondary" onClick={onExitSelect}>
            {t("library.cancelSelect")}
          </Button>
        </div>
      ) : (
        <div className="library-detail-actions">
          <Button kind="secondary">{t("library.openFolder")}</Button>
          <Button kind="secondary">{t("library.openPdf")}</Button>
          <Button kind="secondary">{t("library.rebuildPdf")}</Button>
          <Button kind="secondary">{t("library.dedupe")}</Button>
          <Button kind="secondary" onClick={onEnterSelect}>
            {t("library.select")}
          </Button>
          <Button kind="secondary" className="library-btn-danger">
            {t("library.delete")}
          </Button>
        </div>
      )}

      <div className="library-grid">
        {session.captures.map((c) => {
          const caption = t("library.caption", { index: String(c.index).padStart(3, "0"), time: formatElapsed(c.tContent) });
          const tile = (
            <>
              <img src={c.thumb} alt="" className="library-tile-img" />
              <span className="library-tile-caption tabular-nums">{caption}</span>
            </>
          );
          // The checkbox's accessible name comes from the caption text inside the same
          // label, rather than inventing a new "select image N" string the spec does not
          // give (SPEC-QUESTION, see the report).
          return selectMode ? (
            <label key={c.index} className="library-tile">
              <input
                type="checkbox"
                className="library-tile-check"
                checked={selectedCaptures.has(c.index)}
                onChange={() => onToggleCapture(c.index)}
              />
              {tile}
            </label>
          ) : (
            <div key={c.index} className="library-tile">
              {tile}
            </div>
          );
        })}
      </div>
    </div>
  );
}

/**
 * Loads one session's detail on demand and owns its select-mode UI state. A fresh
 * instance (keyed by `id` at both call sites below) is the reset mechanism when the
 * shown session changes -- remounting naturally returns detail/selectMode/
 * selectedCaptures to their initial values, rather than an effect calling setState
 * synchronously in its body to reset them (react-hooks/set-state-in-effect).
 */
function SessionDetailLoader({ id, t, lang }: { id: string; t: TFunc; lang: Lang }) {
  const [detail, setDetail] = useState<SessionDetail | null>(null);
  const [selectMode, setSelectMode] = useState(false);
  const [selectedCaptures, setSelectedCaptures] = useState<Set<number>>(new Set());

  useEffect(() => {
    let active = true;
    getSession(id).then((d) => {
      if (active) setDetail(d);
    });
    return () => {
      active = false;
    };
  }, [id]);

  function toggleCapture(index: number) {
    setSelectedCaptures((prev) => {
      const next = new Set(prev);
      if (next.has(index)) next.delete(index);
      else next.add(index);
      return next;
    });
  }

  function exitSelect() {
    setSelectMode(false);
    setSelectedCaptures(new Set());
  }

  if (!detail) return null;

  return (
    <SessionDetailPane
      t={t}
      lang={lang}
      session={detail}
      selectMode={selectMode}
      selectedCaptures={selectedCaptures}
      onEnterSelect={() => setSelectMode(true)}
      onExitSelect={exitSelect}
      onToggleCapture={toggleCapture}
      // No-op mock function (docs/impl/library.md > What to build, item 5): nothing is
      // actually deleted in this prototype.
      onTrashSelected={exitSelect}
    />
  );
}

export default function Library() {
  const { lang, t } = useLang();
  const layout = layoutFromLocation();

  const [appState, setAppState] = useState<AppState | null>(null);
  const [sessions, setSessions] = useState<SessionSummary[] | null>(null);
  const [query, setQuery] = useState("");
  const [selectedId, setSelectedId] = useState<string | null>(null); // layout A
  const [openedId, setOpenedId] = useState<string | null>(null); // layout B

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

  useEffect(() => {
    let active = true;
    listSessions().then((list) => {
      if (!active) return;
      setSessions(list);
      setSelectedId((current) => current ?? list[0]?.id ?? null);
    });
    return () => {
      active = false;
    };
  }, []);

  const filtered = useMemo(() => (sessions ?? []).filter((s) => matchesQuery(s, query)), [sessions, query]);

  useEffect(() => {
    if (layout !== "split") return;
    function onKeyDown(e: KeyboardEvent) {
      // Let arrow keys behave normally while typing in the search field (or any other
      // input) instead of hijacking them for list navigation.
      if (e.target instanceof HTMLInputElement) return;
      if (filtered.length === 0) return;
      const idx = filtered.findIndex((s) => s.id === selectedId);
      let next: number | null = null;
      if (e.key === "ArrowDown") next = idx === -1 ? 0 : Math.min(idx + 1, filtered.length - 1);
      else if (e.key === "ArrowUp") next = idx === -1 ? 0 : Math.max(idx - 1, 0);
      else if (e.key === "Home") next = 0;
      else if (e.key === "End") next = filtered.length - 1;
      if (next === null) return;
      e.preventDefault();
      setSelectedId(filtered[next].id);
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [layout, filtered, selectedId]);

  function renderToolbarStatus() {
    const session = appState?.session;
    if (!session) {
      return (
        <div className="library-toolbar-status">
          <span className="library-toolbar-state">{t("panel.statusStopped")}</span>
          <Button kind="primary">{t("action.start")}</Button>
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
        <Button kind="secondary">{isPaused ? t("action.resume") : t("action.pause")}</Button>
        <Button kind="primary">{t("action.stop")}</Button>
      </div>
    );
  }

  function renderSplitBody() {
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
              role="listbox"
              aria-labelledby="library-search-label"
              aria-activedescendant={selectedId ? `library-option-${selectedId}` : undefined}
              tabIndex={0}
              className="library-list"
            >
              {filtered.map((s) => (
                // Keyboard interaction is handled globally above (Arrow/Home/End) via
                // aria-activedescendant, the same listbox pattern as Panel's picker; the
                // click handler is the mouse equivalent, so no key handler is needed here.
                // Click rather than Panel's hover-to-preselect: this is a persistent
                // master-detail list someone may leave open, not a short-lived picker, so
                // changing the detail pane just by passing the mouse over it would surprise.
                // eslint-disable-next-line jsx-a11y/click-events-have-key-events
                <li
                  key={s.id}
                  id={`library-option-${s.id}`}
                  role="option"
                  aria-selected={s.id === selectedId}
                  className={s.id === selectedId ? "library-row is-selected" : "library-row"}
                  onClick={() => setSelectedId(s.id)}
                >
                  {s.thumb ? <img src={s.thumb} alt="" className="library-row-thumb" /> : <span className="library-row-thumb" />}
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
        <div className="library-detail-pane">{selectedId && <SessionDetailLoader key={selectedId} id={selectedId} t={t} lang={lang} />}</div>
      </div>
    );
  }

  function renderListBody() {
    if (openedId) {
      return (
        <div className="library-detail-pane">
          <Button kind="secondary" className="library-back-btn" onClick={() => setOpenedId(null)}>
            {t("action.back")}
          </Button>
          <SessionDetailLoader key={openedId} id={openedId} t={t} lang={lang} />
        </div>
      );
    }
    return (
      <div className="library-body-list">
        <div className="library-search-wrap library-search-wrap-wide">
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
          filtered.map((s) => (
            <div key={s.id} className="library-row-wide">
              {s.thumb ? <img src={s.thumb} alt="" className="library-row-wide-thumb" /> : <span className="library-row-wide-thumb" />}
              <div className="library-row-wide-main">
                <p className="library-row-wide-title">{s.title}</p>
                <p className="library-row-wide-meta tabular-nums">
                  {sessionMetaLine(s, lang, t)}
                  <SessionBadge session={s} t={t} />
                </p>
              </div>
              <div className="library-row-wide-actions">
                <button type="button" className="library-text-btn" onClick={() => setOpenedId(s.id)}>
                  {t("library.open")}
                </button>
                <button type="button" className="library-text-btn">
                  {t("library.folder")}
                </button>
                <button type="button" className="library-text-btn">
                  {t("library.pdf")}
                </button>
                <IconButton icon={<FiTrash2 aria-hidden="true" />} label={t("library.delete")} className="library-icon-btn-danger" />
              </div>
            </div>
          ))
        )}
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
      {sessions && sessions.length === 0 ? (
        <div className="library-empty-state">
          <p>{t("library.empty")}</p>
          <Button kind="primary">{t("action.start")}</Button>
        </div>
      ) : sessions ? (
        layout === "list" ? (
          renderListBody()
        ) : (
          renderSplitBody()
        )
      ) : (
        <div className="library-body" />
      )}
    </div>
  );
}
