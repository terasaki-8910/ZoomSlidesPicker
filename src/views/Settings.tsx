import { useCallback, useEffect, useState } from "react";
import Button from "../components/Button";
import Notice from "../components/Notice";
import { useLang } from "../i18n";
import type { TKey } from "../i18n";
import { chooseSaveRoot, getSettings, onSettings, setSettings as setSettingsIpc } from "../ipc";
import type { LangSetting, Settings as SettingsModel, SettingsView, Shortcuts } from "../ipc/types";
import { isMac } from "../lib/platform";
import { acceleratorFromKeyDown, formatAccelerator } from "../lib/shortcuts";
import "./Settings.css";

type ShortcutKey = keyof Shortcuts;

const SHORTCUT_ROWS: { key: ShortcutKey; labelKey: TKey }[] = [
  { key: "saveNow", labelKey: "action.saveNow" },
  { key: "undo", labelKey: "action.undoLast" },
  { key: "pauseResume", labelKey: "settings.shortcut.pauseResume" },
];

type Status = { kind: "saved" } | { kind: "error"; message: string };

export default function Settings() {
  const { t } = useLang();
  const [settings, setSettingsState] = useState<SettingsView | null>(null);
  const [intervalInput, setIntervalInput] = useState("");
  const [intervalError, setIntervalError] = useState<string | null>(null);
  const [status, setStatus] = useState<Status | null>(null);
  const [recording, setRecording] = useState<ShortcutKey | null>(null);
  const mac = isMac();

  useEffect(() => {
    let active = true;
    function apply(s: SettingsView) {
      if (!active) return;
      setSettingsState(s);
      setIntervalInput(String(s.intervalS));
    }
    getSettings().then(apply);
    let unlisten: (() => void) | undefined;
    onSettings(apply).then((u) => {
      unlisten = u;
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  const persist = useCallback(
    async (partial: Partial<SettingsModel>) => {
      if (!settings) return;
      const next: SettingsModel = {
        language: settings.language,
        saveRoot: settings.saveRoot,
        intervalS: settings.intervalS,
        shortcuts: settings.shortcuts,
        ...partial,
      };
      try {
        const updated = await setSettingsIpc(next);
        setSettingsState(updated);
        setIntervalInput(String(updated.intervalS));
        setStatus({ kind: "saved" });
      } catch (err) {
        setStatus({ kind: "error", message: err instanceof Error ? err.message : String(err) });
      }
    },
    [settings]
  );

  async function handleChooseSaveRoot() {
    const picked = await chooseSaveRoot();
    if (picked) void persist({ saveRoot: picked });
  }

  function handleIntervalBlur() {
    const value = Number(intervalInput);
    const steps = value * 2;
    if (!Number.isFinite(value) || value < 0.5 || value > 5 || Math.round(steps) !== steps) {
      setIntervalError(t("settings.intervalError"));
      return;
    }
    setIntervalError(null);
    void persist({ intervalS: value });
  }

  useEffect(() => {
    if (!recording) return;
    function onKeyDown(e: KeyboardEvent) {
      // Re-checked here, not just above: TS does not carry a narrowing from the effect
      // body into a callback that can run later (addEventListener), even for a `const`.
      if (!recording || !settings) return;
      e.preventDefault();
      if (e.key === "Escape") {
        setRecording(null);
        return;
      }
      const accel = acceleratorFromKeyDown(e);
      if (!accel) return; // only modifier keys pressed so far -- keep waiting
      setRecording(null);
      void persist({ shortcuts: { ...settings.shortcuts, [recording]: accel } });
    }
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [recording, settings, persist]);

  if (!settings) return null;

  return (
    <div className="settings" data-testid="settings-root">
      <h1 className="settings-title">{t("settings.title")}</h1>

      <section className="settings-section">
        <h2>{t("settings.general")}</h2>
        <div className="settings-row">
          <label className="settings-row-label" htmlFor="settings-language">
            {t("settings.language")}
          </label>
          <select
            id="settings-language"
            value={settings.language}
            onChange={(e) => void persist({ language: e.target.value as LangSetting })}
          >
            <option value="auto">{t("settings.langAuto")}</option>
            <option value="ja">{t("settings.langJa")}</option>
            <option value="en">{t("settings.langEn")}</option>
          </select>
        </div>
        <div className="settings-row">
          <span className="settings-row-label">{t("settings.saveRoot")}</span>
          <span className="settings-save-path" title={settings.saveRoot}>
            {settings.saveRoot}
          </span>
          <Button kind="secondary" onClick={() => void handleChooseSaveRoot()}>
            {t("action.changeEllipsis")}
          </Button>
        </div>
      </section>

      <section className="settings-section">
        <h2>{t("settings.capture")}</h2>
        <div className="settings-row">
          <label className="settings-row-label" htmlFor="settings-interval">
            {t("settings.interval")}
          </label>
          <input
            id="settings-interval"
            type="number"
            min={0.5}
            max={5}
            step={0.5}
            inputMode="decimal"
            className="tabular-nums"
            value={intervalInput}
            onChange={(e) => setIntervalInput(e.target.value)}
            onBlur={handleIntervalBlur}
            aria-invalid={intervalError != null}
            aria-describedby={intervalError ? "settings-interval-error" : undefined}
          />
        </div>
        {intervalError && <Notice id="settings-interval-error">{intervalError}</Notice>}
      </section>

      <section className="settings-section">
        <h2>{t("settings.shortcuts")}</h2>
        {SHORTCUT_ROWS.map(({ key, labelKey }) => (
          <div className="settings-row" key={key}>
            <span className="settings-row-label">{t(labelKey)}</span>
            <span className="settings-shortcut-value tabular-nums">
              {recording === key ? (
                t("settings.recording")
              ) : (
                <span className="settings-shortcut-accel">{formatAccelerator(settings.shortcuts[key], mac)}</span>
              )}
            </span>
            <Button kind="secondary" onClick={() => setRecording(key)} disabled={recording != null}>
              {t("action.change")}
            </Button>
          </div>
        ))}
      </section>

      <p className={status?.kind === "error" ? "settings-status is-error" : "settings-status"} role="status">
        {status?.kind === "saved" ? t("settings.saved") : status?.kind === "error" ? status.message : ""}
      </p>
    </div>
  );
}
