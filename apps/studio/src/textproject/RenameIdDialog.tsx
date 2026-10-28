import { useEffect, useRef, useState } from "react";
import { errorText, planRenameId, renameId } from "../api";
import type { RenamePlan, RenameResult } from "../api";
import { count } from "../technik/syncWords";
import { Dialog, Notice, TextField } from "../ui/primitives";
import styles from "./RenameIdDialog.module.css";

/** How long the plan waits for the typing to stop. */
export const PLAN_DELAY_MS = 300;

/** How many of the files the plan names; the rest are counted. */
const SHOWN_FILES = 8;

/** The message after a rename. */
export function renamedText(plan: RenamePlan): string {
  return `ID umbenannt: ${plan.old_id} → ${plan.new_id}. ⌘Z nimmt es zurück.`;
}

const filesText = (n: number) => count(n, "Datei", "Dateien");
const refsText = (n: number) => count(n, "Verweis", "Verweise");

export type RenameIdDialogProps = {
  /** The register id to rename (`TRAINER_…`, `MAP_…`). */
  id: string;
  onClose: () => void;
  /** The rename went in (one undo step); the dialog closes. */
  onRenamed: (result: RenameResult) => void;
};

/**
 * „ID umbenennen“: a field filled with the old id; a pause after typing asks for the plan, which names the files whose references follow
 * (or why the new id is refused, which locks „Umbenennen“). „Umbenennen“ renames as one undo step.
 */
export function RenameIdDialog({ id, onClose, onRenamed }: RenameIdDialogProps) {
  const [typed, setTyped] = useState(id);
  const [plan, setPlan] = useState<RenamePlan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const busy = useRef(false);
  // Only the answer for the text typed last counts.
  const asked = useRef(0);

  const wanted = typed.trim();
  useEffect(() => {
    const seq = ++asked.current;
    setPlan(null);
    setError(null);
    if (wanted === "" || wanted === id) return;
    const timer = setTimeout(() => {
      planRenameId(id, wanted).then(
        (next) => seq === asked.current && setPlan(next),
        (e: unknown) => seq === asked.current && setError(errorText(e)),
      );
    }, PLAN_DELAY_MS);
    return () => clearTimeout(timer);
  }, [id, wanted]);

  const current = plan !== null && plan.new_id === wanted ? plan : null;
  const ready = current !== null && current.refusal === null && !running;

  const rename = async () => {
    if (!ready || current === null || busy.current) return;
    busy.current = true;
    setRunning(true);
    setError(null);
    try {
      const result = await renameId(id, current.new_id);
      onRenamed(result);
      onClose();
    } catch (e) {
      setError(errorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  const more = current === null ? 0 : current.files.length - SHOWN_FILES;
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy.current) onClose();
      }}
      dismissible={!running}
      title="ID umbenennen"
      data={{ "data-rename-id-dialog": "" }}
      actions={[
        { label: "Abbrechen", onClick: onClose, disabled: running },
        { label: "Umbenennen", kind: "primary", onClick: () => void rename(), disabled: !ready },
      ]}
    >
      <div className={styles.content}>
        <TextField label="Neue ID" showLabel value={id} onCommit={setTyped} onChange={setTyped} disabled={running} />
        {current?.refusal != null && <Notice kind="error">{current.refusal}</Notice>}
        {current !== null && current.refusal === null && (
          <Notice kind="info">
            <p className={styles.sentence}>{`${refsText(current.total)} in ${filesText(current.files.length)} werden nachgezogen.`}</p>
            {current.files.length > 0 && (
              <ul className={styles.files}>
                {current.files.slice(0, SHOWN_FILES).map((f) => (
                  <li key={f.path} className={styles.file} data-rename-file="">
                    {f.path}
                  </li>
                ))}
                {more > 0 && <li className={styles.more}>{`und ${count(more, "weitere", "weitere")}`}</li>}
              </ul>
            )}
          </Notice>
        )}
        {error !== null && <Notice kind="error">{error}</Notice>}
      </div>
    </Dialog>
  );
}
