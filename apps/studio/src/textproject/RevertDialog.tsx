import { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText, revertTextProject } from "../api";
import type { RevertReport } from "../api";
import { templatesNote } from "../git/gitWords";
import { t } from "../i18n";
import { Dialog, Notice } from "../ui/primitives";
import styles from "./AdoptDialog.module.css";

/** The message after taking the text project back: where its files went, then what became of the Git templates. */
export function revertedText(report: RevertReport): string {
  const note = templatesNote(report);
  return note === "" ? t("textproject:revertDialog.reverted", { backup: report.backup }) : t("textproject:revertDialog.revertedWithNote", { backup: report.backup, note });
}

export type RevertDialogProps = {
  /** Unsaved edits of the store and the open scripts: taking back needs everything saved. */
  unsaved: number;
  /** „Abbrechen“, Escape, and after taking it back. */
  onClose: () => void;
  /** It went in: the backend opened the session anew (the app reads it again and says so). */
  onReverted: (report: RevertReport) => void | Promise<void>;
  /** Runs it while nothing else may write (the app's session is busy meanwhile); by default it just runs. */
  exclusive?: <T>(work: () => Promise<T>) => Promise<T>;
};

/**
 * „Textprojekt zurücknehmen?“: the studio works with the DSPRE files again, the text project's files move to `.g4/backup/`. Locked while
 * something is unsaved; the kernel's refusal (something waits for the sync) shows its sentence.
 */
export function RevertDialog({ unsaved, onClose, onReverted, exclusive = (work) => work() }: RevertDialogProps) {
  const { t } = useTranslation("textproject");
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const busy = useRef(false);

  const revert = async () => {
    if (unsaved > 0 || busy.current) return;
    busy.current = true;
    setRunning(true);
    setError(null);
    try {
      const report = await exclusive(() => revertTextProject());
      await onReverted(report);
      onClose();
    } catch (e) {
      setError(errorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy.current) onClose();
      }}
      dismissible={!running}
      title={t("revertDialog.title")}
      description={t("revertDialog.description")}
      data={{ "data-revert-dialog": "" }}
      actions={[
        { label: t("revertDialog.cancel"), onClick: onClose, disabled: running },
        { label: t("revertDialog.revert"), kind: "primary", onClick: () => void revert(), disabled: unsaved > 0 || running },
      ]}
    >
      {(unsaved > 0 || error !== null) && (
        <div className={styles.content}>
          {unsaved > 0 && <Notice kind="warning">{t("revertDialog.saveFirst")}</Notice>}
          {error !== null && <Notice kind="error">{error}</Notice>}
        </div>
      )}
    </Dialog>
  );
}
