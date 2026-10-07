import { useEffect, useRef, useState } from "react";
import { FiClock, FiSettings } from "react-icons/fi";
import Button from "../components/Button";
import IconButton from "../components/IconButton";
import Notice from "../components/Notice";
import WindowPicker from "../components/WindowPicker";
import { useLang } from "../i18n";
import {
  closeWindow,
  fitPanel,
  getAppState,
  onAppState,
  openFolder,
  openLibrary,
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
import type { AppState } from "../ipc/types";
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
  const [rebuilding, setRebuilding] = useState(false);
  const [rebuildStatus, setRebuildStatus] = useState<RebuildStatus | null>(null);
  const headerRef = useRef<HTMLElement>(null);
  const contentRef = useRef<HTMLDivElement>(null);

  // Reports the content's natural height (header plus body content, not the body box,
  // which fills the window) so Rust can size the panel window to fit it: short when
  // stopped, taller with the window list or a running session (docs/impl/ipc.md >
  // fit_panel).
  useEffect(() => {
    const header = headerRef.current;
    const content = contentRef.current;
    if (!header || !content) return;
    function report() {
      // Nothing to fit before the app state has arrived (the body renders nothing).
      if (!header || !content || content.childElementCount === 0) return;
      void fitPanel(Math.ceil(header.getBoundingClientRect().height + content.getBoundingClientRect().height));
    }
    report();
    if (typeof ResizeObserver === "undefined") return;
    const ro = new ResizeObserver(report);
    ro.observe(header);
    ro.observe(content);
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
    function onKeyDown(e: KeyboardEvent) {
      // While the picker is open, WindowPicker owns Escape/Arrow/Enter itself (it calls
      // onCancel/onPicked, which this component wires back to setPickerOpen(false)).
      if (pickerOpen) return;
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
  }, [pickerOpen]);

  function handleOpenPicker() {
    setPickerOpen(true);
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
          <WindowPicker onPicked={() => setPickerOpen(false)} onCancel={() => setPickerOpen(false)} />
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
    <div className="panel" data-testid="panel-root">
      <header className="panel-header" ref={headerRef}>
        <span className="panel-header-title">{t("panel.title")}</span>
        <div className="panel-header-actions">
          <IconButton icon={<FiClock aria-hidden="true" />} label={t("library.openButton")} onClick={() => void openLibrary()} />
          <IconButton icon={<FiSettings aria-hidden="true" />} label={t("action.openSettings")} onClick={() => void openSettings()} />
        </div>
      </header>
      <div className="panel-body">
        <div className="panel-content" ref={contentRef}>
          {renderBody()}
        </div>
      </div>
    </div>
  );
}
