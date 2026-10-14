import { useEffect, useState } from "react";
import { useLang } from "../i18n";
import { answerUpdate, getAppState, installUpdate, onAppState, onUpdateProgress } from "../ipc";
import type { UpdateInfo, UpdateProgress } from "../ipc/types";
import Button from "./Button";
import Modal from "./Modal";
import Notice from "./Notice";
import "./UpdateDialog.css";

interface UpdateDialogProps {
  info: UpdateInfo;
  /** The dialog is done (Later, Skip, or Close after a failure, which counts as Later). Never called while installing. */
  onClose: () => void;
  /**
   * Library window (launch offer): Later and light dismiss also tell Rust (`answer_update("later")`).
   * Settings window (manual check): Later just closes. Skip always calls `answer_update("skip")`.
   */
  answerLater: boolean;
}

type Stage = "offer" | "installing" | "failed";

function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** docs/impl/updates.md > UI > Update dialog. */
export default function UpdateDialog({ info, onClose, answerLater }: UpdateDialogProps) {
  const { t } = useLang();
  const [stage, setStage] = useState<Stage>("offer");
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [error, setError] = useState("");

  // Busy is live: a session starting or ending while the dialog is open changes the button.
  useEffect(() => {
    let active = true;
    const apply = (s: { session: unknown; videoProgress: unknown }) => {
      if (active) setBusy(s.session !== null || s.videoProgress !== null);
    };
    getAppState().then(apply);
    let unlisten: (() => void) | undefined;
    onAppState(apply).then((u) => {
      if (active) unlisten = u;
      else u();
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  useEffect(() => {
    let active = true;
    let unlisten: (() => void) | undefined;
    onUpdateProgress((p) => {
      if (active) setProgress(p);
    }).then((u) => {
      if (active) unlisten = u;
      else u();
    });
    return () => {
      active = false;
      unlisten?.();
    };
  }, []);

  function later() {
    if (answerLater) void answerUpdate("later");
    onClose();
  }

  function skip() {
    void answerUpdate("skip");
    onClose();
  }

  async function install() {
    setStage("installing");
    setProgress(null);
    try {
      await installUpdate();
    } catch (e) {
      setError(errorMessage(e));
      setStage("failed");
    }
  }

  if (stage === "installing") {
    // Deliberately not dismissible: the app restarts as soon as the install finishes.
    const percent =
      progress?.phase === "downloading" && progress.total ? Math.min(100, Math.floor((progress.downloaded / progress.total) * 100)) : null;
    const line =
      progress?.phase === "installing"
        ? t("update.installing")
        : percent !== null
          ? t("update.downloading", { percent })
          : t("update.downloadingNoTotal");
    return (
      <Modal titleId="update-dialog-title" onClose={() => {}}>
        <h2 id="update-dialog-title" className="update-title">
          {t("update.updatingTitle")}
        </h2>
        <p className="update-line tabular-nums" role="status">
          {line}
        </p>
        {percent !== null && progress?.phase === "downloading" && (
          <div className="update-bar" aria-hidden="true">
            <div className="update-bar-fill" style={{ width: `${percent}%` }} />
          </div>
        )}
        <p className="update-note">{t("update.restartNote")}</p>
      </Modal>
    );
  }

  if (stage === "failed") {
    // Closing after a failure counts as Later: the launch offer is not shown again this run
    // (the manual check in Settings can try again).
    return (
      <Modal titleId="update-dialog-title" onClose={later}>
        <h2 id="update-dialog-title" className="update-title">
          {t("update.title")}
        </h2>
        <Notice>{t("update.failed", { error })}</Notice>
        <div className="update-actions">
          <Button kind="primary" onClick={later}>
            {t("update.close")}
          </Button>
        </div>
      </Modal>
    );
  }

  return (
    <Modal titleId="update-dialog-title" onClose={later}>
      <h2 id="update-dialog-title" className="update-title">
        {t("update.title")}
      </h2>
      <div className="update-text">
        <p>{t("update.body", { version: info.version })}</p>
        <p className="update-current">{t("update.current", { current: info.current })}</p>
      </div>
      {busy && <Notice tone="neutral">{t("update.busy")}</Notice>}
      <div className="update-actions">
        <Button kind="secondary" className="update-skip" onClick={skip}>
          {t("update.skip")}
        </Button>
        <Button kind="secondary" onClick={later}>
          {t("update.later")}
        </Button>
        <Button kind="primary" disabled={busy} onClick={() => void install()}>
          {t("update.now")}
        </Button>
      </div>
    </Modal>
  );
}
