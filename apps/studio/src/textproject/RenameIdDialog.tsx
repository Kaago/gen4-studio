import type { KeyboardEvent } from "react";
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText, planRenameId, renameId } from "../api";
import type { RenamePlan, RenameResult } from "../api";
import { t } from "../i18n";
import { Dialog, Notice, TextField } from "../ui/primitives";
import styles from "./RenameIdDialog.module.css";

/** How long the plan waits for the typing to stop. */
export const PLAN_DELAY_MS = 300;

/** How many of the files the plan names; the rest are counted. */
const SHOWN_FILES = 8;

/** The message after a rename. */
export function renamedText(plan: RenamePlan): string {
  return t("textproject:renameIdDialog.renamed", { old: plan.old_id, new: plan.new_id });
}

const filesText = (n: number) => t("textproject:renameIdDialog.filesCount", { count: n });
const refsText = (n: number) => t("textproject:renameIdDialog.refsCount", { count: n });

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
  const { t } = useTranslation("textproject");
  const [typed, setTyped] = useState(id);
  const [plan, setPlan] = useState<RenamePlan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  // The plan for the current text is being asked for (it reads every source file, which a virus scanner can slow to half a minute).
  const [asking, setAsking] = useState(false);
  const busy = useRef(false);
  // Only the answer for the text typed last counts.
  const asked = useRef(0);

  const wanted = typed.trim();
  useEffect(() => {
    const seq = ++asked.current;
    setPlan(null);
    setError(null);
    setAsking(false);
    if (wanted === "" || wanted === id) return;
    const timer = setTimeout(() => {
      setAsking(true);
      planRenameId(id, wanted).then(
        (next) => {
          if (seq !== asked.current) return;
          setPlan(next);
          setAsking(false);
        },
        (e: unknown) => {
          if (seq !== asked.current) return;
          setError(errorText(e));
          setAsking(false);
        },
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

  // Enter in the field renames like „Umbenennen“, when that is ready; otherwise it does nothing (the key is used up either way, so the
  // button that opened the dialog is not pressed again as the focus returns to it).
  const onEnter = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "Enter" || e.nativeEvent.isComposing) return;
    e.preventDefault();
    void rename();
  };

  const more = current === null ? 0 : current.files.length - SHOWN_FILES;
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy.current) onClose();
      }}
      dismissible={!running}
      title={t("renameIdDialog.title")}
      data={{ "data-rename-id-dialog": "" }}
      actions={[
        { label: t("renameIdDialog.cancel"), onClick: onClose, disabled: running },
        { label: t("renameIdDialog.rename"), kind: "primary", onClick: () => void rename(), disabled: !ready },
      ]}
    >
      <div className={styles.content}>
        <div onKeyDown={onEnter}>
          <TextField label={t("renameIdDialog.newId")} showLabel value={typed} onCommit={setTyped} onChange={setTyped} disabled={running} />
        </div>
        {asking && !running && (
          <p role="status" className={styles.wait}>
            {t("renameIdDialog.searching")}
          </p>
        )}
        {current?.refusal != null && <Notice kind="error">{current.refusal}</Notice>}
        {current !== null && current.refusal === null && (
          <Notice kind="info">
            <p className={styles.sentence}>{t("renameIdDialog.sentence", { refs: refsText(current.total), files: filesText(current.files.length) })}</p>
            {current.files.length > 0 && (
              <ul className={styles.files}>
                {current.files.slice(0, SHOWN_FILES).map((f) => (
                  <li key={f.path} className={styles.file} data-rename-file="">
                    {f.path}
                  </li>
                ))}
                {more > 0 && <li className={styles.more}>{t("renameIdDialog.moreCount", { count: more })}</li>}
              </ul>
            )}
          </Notice>
        )}
        {running && (
          <p role="status" className={styles.wait}>
            {t("renameIdDialog.renaming")}
          </p>
        )}
        {error !== null && <Notice kind="error">{error}</Notice>}
      </div>
    </Dialog>
  );
}
