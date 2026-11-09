import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText, isCmdError, restoreTextProject } from "../api";
import type { RestoreProgress, RestoreReport } from "../api";
import { t } from "../i18n";
import { openPath } from "../platform/dialogs";
import { toast } from "../ui/frame/toasts";
import { Dialog, Notice } from "../ui/primitives";
import type { DialogAction } from "../ui/primitives";
import styles from "./GitInitDialog.module.css";

/** The file dialog of „Basis-ROM wählen…“. */
const pickOptions = () => ({ directory: false, title: t("git:restoreDialog.pickTitle"), filters: [{ name: t("git:restoreDialog.pickFilter"), extensions: ["nds"] }] });

/** The progress line of a phase; the import's count only when it counts more than one step. */
export function restoreProgressText(p: RestoreProgress): string {
  switch (p.phase) {
    case "extract":
      return t("git:restoreDialog.progress.extract");
    case "merge":
      return t("git:restoreDialog.progress.merge");
    case "import":
      // Without fine progress (only 0 and 1 of 1) the count says nothing.
      return p.total > 1 ? t("git:restoreDialog.progress.importCount", { done: p.done, total: p.total }) : t("git:restoreDialog.progress.import");
    case "sync":
      return t("git:restoreDialog.progress.sync");
  }
}

/** The message after completing: the seconds it took. */
export function restoredText(report: RestoreReport): string {
  return t("git:restoreDialog.restored", { seconds: Math.round(report.ms / 1000) });
}

export type RestoreDialogProps = {
  /** The folder of the clone (as it was asked to open). */
  root: string;
  /** The kernel's sentence of `restore_needed`, with the base ROM's name. */
  message: string;
  /** „Abbrechen“ and Escape: nothing is done, the clone stays as it is. */
  onClose: () => void;
  /** Completed: the backend opened the project; the app reads it as after opening. */
  onOpened: () => void | Promise<void>;
};

/**
 * „Textprojekt vervollständigen?“ when a folder opens that is a text project without its game files (a fresh Git clone): on
 * „Vervollständigen“ the game files come from the base ROM (the user's files stay as they are), then the project opens. Without a found
 * base ROM the reason shows with „Basis-ROM wählen…“, which completes from the chosen file (a foreign ROM is refused with the kernel's
 * sentence). While it runs the dialog has no button and stays.
 */
export function RestoreDialog({ root, message, onClose, onOpened }: RestoreDialogProps) {
  const { t } = useTranslation("git");
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<RestoreProgress | null>(null);
  // Why no base ROM was found (`base_missing`): „Basis-ROM wählen…“ shows from then on.
  const [missing, setMissing] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  // Read by a click that comes before the render of the one before (a double click).
  const busy = useRef(false);

  const restore = async (nds: string | null) => {
    if (busy.current) return;
    busy.current = true;
    setRunning(true);
    setProgress(null);
    setError(null);
    try {
      const report = await restoreTextProject(root, nds, setProgress);
      toast.info(restoredText(report));
      await onOpened();
    } catch (e) {
      if (isCmdError(e) && e.code === "base_missing") setMissing(e.message);
      else setError(errorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };
  const pick = async () => {
    if (busy.current) return;
    const nds = await openPath(pickOptions());
    if (nds !== null) await restore(nds);
  };

  const actions: DialogAction[] = running
    ? []
    : [
        { label: t("shared.cancel"), onClick: onClose },
        { label: t("restoreDialog.complete"), kind: "primary", onClick: () => void restore(null) },
      ];

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy.current) onClose();
      }}
      dismissible={!running}
      title={t("restoreDialog.title")}
      description={t("restoreDialog.description", { message })}
      data={{ "data-restore-dialog": "" }}
      actions={actions}
    >
      {(running || missing !== null || error !== null) && (
        <div className={styles.content}>
          {missing !== null && (
            <Notice kind="warning" actions={running ? undefined : [{ label: t("restoreDialog.chooseBase"), onClick: () => void pick() }]}>
              {missing}
            </Notice>
          )}
          {running && (
            <p role="status" className={styles.progress} data-restore-progress="">
              {restoreProgressText(progress ?? { phase: "extract", done: 0, total: 1 })}
            </p>
          )}
          {error !== null && <Notice kind="error">{error}</Notice>}
        </div>
      )}
    </Dialog>
  );
}
