import { useEffect, useState } from "react";
import { useLang } from "../i18n";
import type { TKey } from "../i18n";
import { estimatePdf, exportPdf } from "../ipc";
import type { PdfExport, PdfQuality } from "../ipc/types";
import { formatBytes } from "../lib/format";
import Button from "./Button";
import Modal from "./Modal";
import Notice from "./Notice";
import "./ExportPdfDialog.css";

const QUALITIES: { quality: PdfQuality; label: TKey }[] = [
  { quality: "lossless", label: "export.lossless" },
  { quality: "standard", label: "export.standard" },
  { quality: "light", label: "export.light" },
];

interface ExportPdfDialogProps {
  dir: string;
  /** "YYYY-MM-DD" of the session's start, used in the selected-images file name. */
  date: string;
  /** null: the whole session (rewrites slides.pdf); else only these capture indices. */
  indices: number[] | null;
  onClose: () => void;
  onDone: (result: PdfExport) => void;
}

function errorMessage(e: unknown): string {
  return e instanceof Error ? e.message : String(e);
}

/** docs/impl/library.md > Round 5 > Library window > Export dialog. */
export default function ExportPdfDialog({ dir, date, indices, onClose, onDone }: ExportPdfDialogProps) {
  const { t } = useLang();
  const [quality, setQuality] = useState<PdfQuality>("standard");
  const [estimates, setEstimates] = useState<Partial<Record<PdfQuality, number>> | null>(null);
  const [exporting, setExporting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    estimatePdf(dir, indices)
      .then((list) => {
        if (active) setEstimates(Object.fromEntries(list.map((e) => [e.quality, e.bytes])));
      })
      .catch((e) => {
        if (active) setError(errorMessage(e));
      });
    return () => {
      active = false;
    };
    // indices is a fresh array per render in the caller only when the selection changed,
    // which cannot happen while this modal is open.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [dir]);

  async function run() {
    setExporting(true);
    setError(null);
    try {
      onDone(await exportPdf(dir, quality, indices));
    } catch (e) {
      setError(errorMessage(e));
      setExporting(false);
    }
  }

  const destination =
    indices === null ? t("export.destWhole") : t("export.destSelected", { name: t("export.selectedName", { date }) });

  return (
    <Modal titleId="export-dialog-title" onClose={onClose}>
      <h2 id="export-dialog-title" className="export-title">
        {t("export.title")}
      </h2>
      <fieldset className="export-options" disabled={exporting}>
        <legend className="visually-hidden">{t("export.quality")}</legend>
        {QUALITIES.map(({ quality: q, label }) => (
          <label key={q} className={quality === q ? "export-option is-checked" : "export-option"}>
            <input type="radio" name="pdf-quality" value={q} checked={quality === q} onChange={() => setQuality(q)} />
            <span className="export-option-label">{t(label)}</span>
            <span className="export-option-size tabular-nums">
              {estimates?.[q] != null
                ? t("export.estimate", { size: formatBytes(estimates[q]) })
                : error
                  ? ""
                  : t("export.estimating")}
            </span>
          </label>
        ))}
      </fieldset>
      <p className="export-destination">{destination}</p>
      {error && <Notice>{error}</Notice>}
      <div className="export-actions">
        <Button kind="secondary" onClick={onClose}>
          {t("library.cancel")}
        </Button>
        <Button kind="primary" disabled={exporting} onClick={() => void run()}>
          {exporting ? t("export.running") : t("export.run")}
        </Button>
      </div>
    </Modal>
  );
}
