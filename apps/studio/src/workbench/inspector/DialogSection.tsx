import { useEffect, useId, useMemo, useRef } from "react";
import { Button, DSScreen, Tooltip } from "../../ui/primitives";
import { pages } from "../text/textModel";
import { dialogSummary } from "../dialog/dialogModel";
import type { DialogTarget } from "../dialog/dialogModel";
import { useDialog } from "../dialog/useDialog";
import styles from "./DialogSection.module.css";

/** Why a figure without a script cannot get a dialog here yet. */
const ANLEGEN_LATER = "Kommt mit 3d.7";

/**
 * The field „Dialog“ of a figure in Erzählen (a sign's is „Text“): the first page of its first message on the DS screen, a line with
 * what it says, and the way into the dialog editor. A common script, a trainer's battle and a script that cannot be read say where they
 * are edited instead; a figure without a script cannot get a dialog before 3d.7.
 */
export function DialogSection({ target, revision, onEdit, onError }: { target: DialogTarget; revision: number; onEdit: (t: DialogTarget) => void; onError: (e: unknown) => void }) {
  const { dialog, error } = useDialog(target, revision);
  const reasonId = useId();
  const variant = target.kind === "bg" ? "sign" : "textbox";
  const first = dialog?.messages[0]?.text ?? null;
  const shown = useMemo(() => (first === null ? [] : pages(first)), [first]);
  // A read that failed is said once per failure, like the editor's own read.
  const report = useRef(onError);
  report.current = onError;
  useEffect(() => {
    if (error !== null) report.current(error);
  }, [error]);

  const source = dialog?.source.kind ?? null;
  return (
    <section className={styles.section} data-dialog-section="">
      <h3 className={styles.title}>{target.kind === "bg" ? "Text" : "Dialog"}</h3>
      <div className={styles.preview} data-dialog-preview={variant}>
        <DSScreen variant={first === null ? "empty" : variant} pages={shown} page={0} />
      </div>
      <p className={styles.summary}>{error !== null ? "Dialog nicht lesbar." : dialog === null ? "Lese …" : dialogSummary(dialog)}</p>
      {source === "local" && (
        <div className={styles.actions}>
          <Button kind="quiet" size="s" onClick={() => onEdit(target)}>
            Dialog bearbeiten…
          </Button>
        </div>
      )}
      {source === "none" && (
        <div className={styles.actions}>
          <Tooltip content={ANLEGEN_LATER}>
            <span className={styles.lockedWrap}>
              <Button kind="quiet" size="s" disabled aria-describedby={reasonId}>
                Dialog anlegen…
              </Button>
            </span>
          </Tooltip>
          <span id={reasonId} hidden>
            {ANLEGEN_LATER}
          </span>
        </div>
      )}
    </section>
  );
}
