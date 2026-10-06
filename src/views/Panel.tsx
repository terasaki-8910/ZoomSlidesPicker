import { useEffect, useRef, useState } from "react";
import { FiSettings } from "react-icons/fi";
import Button from "../components/Button";
import IconButton from "../components/IconButton";
import Notice from "../components/Notice";
import { useLang } from "../i18n";
import {
  beginRegion,
  closeWindow,
  fitPanel,
  getAppState,
  listWindows,
  onAppState,
  openFolder,
  openPermissionSettings,
  openSettings,
  pauseSession,
  rebuildPdf,
  requestPermission,
  resumeSession,
  saveNow,
  stopSession,
  undoLast,
} from "../ipc";
import type { AppState, WindowEntry } from "../ipc/types";
import { formatElapsed } from "../lib/format";
import { isMac } from "../lib/platform";
import "./Panel.css";

type RebuildStatus = { kind: "result"; count: number } | { kind: "error"; message: string };

function matchesSettingsShortcut(e: KeyboardEvent): boolean {
  if (e.key !== "," || e.shiftKey) return false;
  return isMac() ? e.metaKey && !e.ctrlKey && !e.altKey : e.ctrlKey && !e.metaKey && !e.altKey;
}

export default function Panel() {
  const { t } = useLang();
  const [appState, setAppState] = useState<AppState | null>(null);
  const [pickerOpen, setPickerOpen] = useState(false);
  const [windows, setWindows] = useState<WindowEntry[] | null>(null);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [rebuilding, setRebuilding] = useState(false);
  const [rebuildStatus, setRebuildStatus] = useState<RebuildStatus | null>(null);
  const listboxRef = useRef<HTMLUListElement>(null);
  const panelRef = useRef<HTMLDivElement>(null);

  // Reports content height so Rust can size the (otherwise fixed-width, 480 px tall)
  // panel window to fit it -- mostly empty when stopped, taller once a session is
  // running (docs/impl/ipc.md > fit_panel; docs/impl/shell.md).
  useEffect(() => {
    const el = panelRef.current;
    if (!el) return;
    function report() {
      if (el) void fitPanel(el.getBoundingClientRect().height);
    }
    report();
    if (typeof ResizeObserver === "undefined") {
      window.addEventListener("resize", report);
      return () => window.removeEventListener("resize", report);
    }
    const ro = new ResizeObserver(report);
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

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
    if (pickerOpen && windows && windows.length > 0) listboxRef.current?.focus();
  }, [pickerOpen, windows]);

  useEffect(() => {
    function onKeyDown(e: KeyboardEvent) {
      if (pickerOpen) {
        if (e.key === "Escape") {
          e.preventDefault();
          setPickerOpen(false);
          return;
        }
        if (!windows || windows.length === 0) return;
        if (e.key === "ArrowDown") {
          e.preventDefault();
          setSelectedIndex((i) => Math.min(i + 1, windows.length - 1));
          return;
        }
        if (e.key === "ArrowUp") {
          e.preventDefault();
          setSelectedIndex((i) => Math.max(i - 1, 0));
          return;
        }
        if (e.key === "Enter") {
          e.preventDefault();
          const w = windows[selectedIndex];
          if (w) {
            setPickerOpen(false);
            void beginRegion(w.id);
          }
        }
        return;
      }
      if (e.key === "Escape") {
        e.preventDefault();
        void closeWindow();
        return;
      }
      if (matchesSettingsShortcut(e)) {
        e.preventDefault();
        void openSettings();
      }
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [pickerOpen, windows, selectedIndex]);

  function handleOpenPicker() {
    setPickerOpen(true);
    setWindows(null);
    setSelectedIndex(0);
    listWindows().then(setWindows);
  }

  async function handleRequestPermission() {
    await requestPermission();
    setAppState(await getAppState());
  }

  async function handleRebuildPdf() {
    setRebuilding(true);
    setRebuildStatus(null);
    try {
      const count = await rebuildPdf();
      setRebuildStatus({ kind: "result", count });
    } catch (err) {
      setRebuildStatus({ kind: "error", message: err instanceof Error ? err.message : String(err) });
    } finally {
      setRebuilding(false);
    }
  }

  function renderBody() {
    if (!appState) return null;

    if (!appState.permission.granted) {
      return (
        <div className="panel-section">
          <Notice>{t("permission.notice")}</Notice>
          <div className="panel-actions-secondary">
            <Button kind="secondary" onClick={() => void openPermissionSettings()}>
              {t("permission.openSettings")}
            </Button>
            <Button kind="secondary" onClick={() => void handleRequestPermission()}>
              {t("permission.check")}
            </Button>
          </div>
          <Button kind="primary" disabled>
            {t("action.start")}
          </Button>
        </div>
      );
    }

    if (appState.videoProgress) {
      const { done, total } = appState.videoProgress;
      return (
        <div className="panel-section">
          <p className="panel-status">{t("video.processing")}</p>
          <p className="tabular-nums">
            {total === null ? t("video.progressUnknown", { done }) : t("video.progressKnown", { done, total })}
          </p>
        </div>
      );
    }

    if (appState.session) {
      const session = appState.session;
      const isPaused = session.state === "paused";
      return (
        <div className="panel-section">
          <p className="panel-status tabular-nums">
            {isPaused ? t("panel.statusPaused", { count: session.count }) : t("panel.statusRunning", { count: session.count })}
          </p>
          <p className="panel-elapsed tabular-nums">{formatElapsed(session.elapsedS)}</p>
          {session.lastThumb && (
            <button type="button" className="panel-thumb" onClick={() => void openFolder()} aria-label={t("action.openFolder")}>
              <img src={session.lastThumb} alt="" />
            </button>
          )}
          {session.windowLost && <Notice>{t("notice.windowLost")}</Notice>}
          {session.blank && <Notice>{t("notice.blank")}</Notice>}
          <div className="panel-actions">
            <Button kind="primary" onClick={() => void stopSession()}>
              {t("action.stop")}
            </Button>
            <div className="panel-actions-secondary">
              <Button kind="secondary" onClick={() => void (isPaused ? resumeSession() : pauseSession())}>
                {isPaused ? t("action.resume") : t("action.pause")}
              </Button>
              <Button kind="secondary" onClick={() => void saveNow()}>
                {t("action.saveNow")}
              </Button>
              <Button kind="secondary" onClick={() => void undoLast()}>
                {t("action.undoLast")}
              </Button>
            </div>
          </div>
        </div>
      );
    }

    if (pickerOpen) {
      return (
        <div className="panel-section">
          <h2 id="picker-heading" className="panel-picker-heading">
            {t("picker.heading")}
          </h2>
          {windows === null ? (
            <p className="panel-picker-status">{t("picker.loading")}</p>
          ) : windows.length === 0 ? (
            <p className="panel-picker-status">{t("picker.empty")}</p>
          ) : (
            <ul
              ref={listboxRef}
              role="listbox"
              aria-labelledby="picker-heading"
              aria-activedescendant={`picker-option-${selectedIndex}`}
              tabIndex={0}
              className="panel-picker-list"
            >
              {windows.map((w, i) => (
                // Keyboard interaction for this listbox is handled once, globally, by the
                // keydown handler above (Arrow keys + Enter) using aria-activedescendant --
                // the standard ARIA listbox pattern, where individual options are not
                // themselves tab-stops. The click handler below is the mouse-only
                // equivalent of that same Enter action, so it needs no keyboard listener
                // of its own.
                // eslint-disable-next-line jsx-a11y/click-events-have-key-events
                <li
                  key={w.id}
                  id={`picker-option-${i}`}
                  role="option"
                  aria-selected={i === selectedIndex}
                  className={i === selectedIndex ? "panel-picker-option is-selected" : "panel-picker-option"}
                  onPointerEnter={() => setSelectedIndex(i)}
                  onClick={() => {
                    setPickerOpen(false);
                    void beginRegion(w.id);
                  }}
                >
                  <img src={w.thumb} alt="" className="panel-picker-thumb" />
                  <span className="panel-picker-meta">
                    <span className="panel-picker-app">{w.app}</span>
                    <span className="panel-picker-window-title">{w.title}</span>
                  </span>
                </li>
              ))}
            </ul>
          )}
          <Button kind="secondary" onClick={() => setPickerOpen(false)}>
            {t("action.back")}
          </Button>
        </div>
      );
    }

    return (
      <div className="panel-section">
        <p className="panel-status">{t("panel.statusStopped")}</p>
        <div className="panel-actions">
          <Button kind="primary" onClick={handleOpenPicker}>
            {t("action.start")}
          </Button>
          {appState.lastSession && (
            <div className="panel-actions-secondary">
              <Button kind="secondary" onClick={() => void openFolder()}>
                {t("action.openFolder")}
              </Button>
              <Button kind="secondary" onClick={() => void handleRebuildPdf()} disabled={rebuilding}>
                {t("action.rebuildPdf")}
              </Button>
            </div>
          )}
          {rebuildStatus && (
            <Notice tone={rebuildStatus.kind === "error" ? "warning" : "neutral"}>
              {rebuildStatus.kind === "error" ? rebuildStatus.message : t("rebuild.result", { count: rebuildStatus.count })}
            </Notice>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="panel" data-testid="panel-root" ref={panelRef}>
      <header className="panel-header">
        <span className="panel-header-title">{t("panel.title")}</span>
        <IconButton icon={<FiSettings aria-hidden="true" />} label={t("action.openSettings")} onClick={() => void openSettings()} />
      </header>
      <div className="panel-body">{renderBody()}</div>
    </div>
  );
}
