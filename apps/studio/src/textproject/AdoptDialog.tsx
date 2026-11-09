import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { adoptTextProject, errorText, isCmdError } from "../api";
import type { AdoptProgress, AdoptReport } from "../api";
import { templatesNote } from "../git/gitWords";
import { t } from "../i18n";
import { CheckList, Dialog, Notice } from "../ui/primitives";
import type { CheckRow, DialogAction } from "../ui/primitives";
import styles from "./AdoptDialog.module.css";

/**
 * The backend's answer when the files were set up but the open session changed meanwhile and was kept (`textsource::REOPEN`): the project
 * must be opened anew, setting it up again would be refused.
 */
export const reopenText = (): string => t("textproject:adoptDialog.reopen");

/** The unsaved count as the kernel says it („Erst speichern: 1 ungespeicherte Änderung.“). */
export function unsavedText(n: number): string {
  return t("textproject:adoptDialog.unsaved", { count: n });
}

/** The progress line of the set-up. */
export function adoptProgressText(p: AdoptProgress): string {
  const counted = p.total > 0;
  switch (p.phase) {
    case "scripts":
      return counted ? t("textproject:adoptDialog.progress.scriptsCount", { done: p.done, total: p.total }) : t("textproject:adoptDialog.progress.scripts");
    case "base_layer":
      return t("textproject:adoptDialog.progress.baseLayer");
    case "entities":
      return counted ? t("textproject:adoptDialog.progress.entitiesCount", { done: p.done, total: p.total }) : t("textproject:adoptDialog.progress.entities");
    case "sync":
      return t("textproject:adoptDialog.progress.sync");
  }
}

/** The message after the set-up: the files of the text project and the seconds it took, then what became of the Git templates. */
export function adoptedText(report: AdoptReport): string {
  return `${t("textproject:adoptDialog.done", { count: report.export.files, seconds: Math.round(report.ms / 1000) })}${templatesNote(report)}`;
}

export type AdoptDialogProps = {
  /** Unsaved edits of the store and the open scripts: the set-up needs everything saved. */
  unsaved: number;
  /** The base ROM's name when it is found; null when it is not (`baseReason` says why) or not known yet. */
  baseLabel: string | null;
  /** Why the base ROM is missing; null while it is looked for (then the set-up waits too). */
  baseReason: string | null;
  /** The scripts are adopted already; otherwise the set-up adopts them. */
  scriptsAdopted: boolean;
  /** „Abbrechen“, Escape, and after the set-up. */
  onClose: () => void;
  /** The set-up went in: the backend opened the session anew (the app reads it again and says so). */
  onAdopted: (report: AdoptReport) => void | Promise<void>;
  /** Runs the set-up while nothing else may write (the app's session is busy meanwhile); by default it just runs. */
  exclusive?: <T>(work: () => Promise<T>) => Promise<T>;
};

/**
 * „Als Textprojekt einrichten?“: what is checked before (everything saved, the base ROM found, the scripts), then on „Einrichten“ the
 * set-up with its progress. While it runs the dialog has no button and stays (it takes 10 to 120 s and the backend refuses writes
 * meanwhile); a refusal shows its sentence and the dialog can try again.
 */
export function AdoptDialog({ unsaved, baseLabel, baseReason, scriptsAdopted, onClose, onAdopted, exclusive = (work) => work() }: AdoptDialogProps) {
  const { t } = useTranslation("textproject");
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<AdoptProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reopen, setReopen] = useState(false);
  // Read by a click that comes before the render of the one before (a double click).
  const busy = useRef(false);

  const rows: CheckRow[] = [
    unsaved === 0 ? { id: "saved", state: "ok", text: t("adoptDialog.allSaved") } : { id: "saved", state: "error", text: unsavedText(unsaved) },
    baseLabel !== null
      ? { id: "base", state: "ok", text: t("adoptDialog.baseFound", { label: baseLabel }) }
      : baseReason !== null
        ? { id: "base", state: "error", text: baseReason }
        : { id: "base", state: "info", text: t("adoptDialog.baseSearching") },
    scriptsAdopted ? { id: "scripts", state: "ok", text: t("adoptDialog.scriptsAdopted") } : { id: "scripts", state: "info", text: t("adoptDialog.scriptsAdoptedAlong") },
  ];
  const ready = unsaved === 0 && baseLabel !== null;

  const adopt = async () => {
    if (!ready || busy.current) return;
    busy.current = true;
    setRunning(true);
    setProgress(null);
    setError(null);
    try {
      const report = await exclusive(() => adoptTextProject(setProgress));
      await onAdopted(report);
      onClose();
    } catch (e) {
      setError(errorText(e));
      setReopen(isCmdError(e) && e.message === reopenText());
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  const actions: DialogAction[] = running
    ? []
    : reopen
      ? [{ label: t("adoptDialog.close"), onClick: onClose }]
      : [
          { label: t("adoptDialog.cancel"), onClick: onClose },
          { label: t("adoptDialog.setUp"), kind: "primary", onClick: () => void adopt(), disabled: !ready },
        ];

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy.current) onClose();
      }}
      dismissible={!running}
      title={t("adoptDialog.title")}
      description={t("adoptDialog.description")}
      data={{ "data-adopt-dialog": "" }}
      actions={actions}
    >
      <div className={styles.content}>
        <CheckList ariaLabel={t("adoptDialog.beforeSetUp")} rows={rows} />
        {running && (
          <p role="status" className={styles.progress} data-adopt-progress="">
            {progress === null ? t("adoptDialog.settingUp") : adoptProgressText(progress)}
          </p>
        )}
        {error !== null && <Notice kind="error">{error}</Notice>}
      </div>
    </Dialog>
  );
}
