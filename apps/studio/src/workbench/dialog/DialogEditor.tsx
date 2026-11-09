import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { ScriptFiles } from "../../data/useDataArea";
import { Button, DSScreen, Dialog, Notice } from "../../ui/primitives";
import { pages } from "../text/textModel";
import { dialogSummary, distinctMessages } from "./dialogModel";
import type { DialogTarget } from "./dialogModel";
import { DialogMessages, messageKey, useDialogWrites } from "./DialogMessages";
import { useDialog } from "./useDialog";
import styles from "./DialogEditor.module.css";

export interface DialogEditorProps {
  target: DialogTarget;
  /** Changes when the texts or scripts may have moved behind the editor (undo, redo, a write elsewhere): it reads the dialog again. */
  revision: number;
  onClose: () => void;
  /** A text or a split was written: the app counts the edit and tells the text panel. */
  onChanged: () => void;
  /** A failure that is not a refusal of the edit (saving the open script document before a split). */
  onError: (e: unknown) => void;
  /** The script panel's documents: a split writes the figure's script file, so typing there is saved first and the document reloaded after. */
  scriptFiles?: ScriptFiles;
}

/**
 * The dialog editor of a figure (Modal `[data-dialog-editor]`): left its messages in reading order, each in the field of the texts with the
 * badge „Geteilt mit N Figuren“ and „Für diese Figur abspalten“ when other figures show it too; right the DS screen of the chosen message,
 * page by page. It keeps no text of its own: every commit goes to the store (`editTextArchive`, one undo step „Dialog geändert“) and the
 * dialog is read again; ⌘Z in the app changes `revision` and the editor reads it once more. Esc and „Schließen“ close it.
 */
export function DialogEditor({ target, revision, onClose, onChanged, onError, scriptFiles }: DialogEditorProps) {
  const { t } = useTranslation("workbench");
  const { dialog, error, reload } = useDialog(target, revision);
  const [chosen, setChosen] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  // What is typed into a field before it is committed: the screen follows it.
  const [draft, setDraft] = useState<{ key: string; text: string } | null>(null);

  const source = dialog?.source ?? null;
  // Each message once (its first use): a message the script shows twice is one field.
  const distinct = useMemo(() => (source?.kind === "local" ? distinctMessages(dialog!.messages) : []), [dialog, source?.kind]);
  const messages = distinct.map((d) => d.message);
  const current = messages.find((m) => messageKey(m) === chosen) ?? messages[0] ?? null;
  const currentKey = current === null ? null : messageKey(current);
  const text = current === null ? null : draft !== null && draft.key === currentKey ? draft.text : current.text;
  const shown = useMemo(() => (text === null ? [] : pages(text)), [text]);
  const total = Math.max(1, shown.length);
  const pageIndex = Math.min(page, total - 1);
  const variant = target.kind === "bg" ? "sign" : "textbox";

  const choose = (key: string) => {
    if (key === currentKey) return;
    setChosen(key);
    setPage(0);
  };

  const writes = useDialogWrites(target, source, {
    reload,
    onChanged,
    onError,
    scriptFiles,
    onSplit: (key) => {
      setChosen(key);
      setPage(0);
    },
  });
  const notice = writes.notice;

  let body;
  if (error !== null) {
    body = <Notice kind="error">{error}</Notice>;
  } else if (dialog === null) {
    body = <p className={styles.note}>{t("dialogEditor.reading")}</p>;
  } else if (source?.kind !== "local") {
    body = <Notice kind="info">{dialogSummary(dialog)}</Notice>;
  } else if (messages.length === 0) {
    body = <p className={styles.note}>{t("dialogEditor.noMessage")}</p>;
  } else {
    body = (
      <div className={styles.columns}>
        <DialogMessages kind={target.kind} distinct={distinct} currentKey={currentKey} onChoose={choose} onDraft={(key, text) => setDraft({ key, text })} writes={writes} />
        <div className={styles.preview}>
          <span className={styles.caption}>{current === null ? "" : t("dialogEditor.message", { index: current.index })}</span>
          <DSScreen variant={variant} pages={shown} page={pageIndex} scale={1} />
          <div className={styles.pager}>
            <Button kind="quiet" size="s" aria-label={t("dialogEditor.previousPage")} disabled={pageIndex === 0} onClick={() => setPage(pageIndex - 1)}>
              ‹
            </Button>
            <span className={styles.page}>{t("dialogEditor.page", { page: pageIndex + 1, total })}</span>
            <Button kind="quiet" size="s" aria-label={t("dialogEditor.nextPage")} disabled={pageIndex >= total - 1} onClick={() => setPage(pageIndex + 1)}>
              ›
            </Button>
          </div>
        </div>
      </div>
    );
  }

  return (
    <Dialog
      open
      size="wide"
      onOpenChange={(open) => !open && onClose()}
      title={t("dialogEditor.title", { title: target.title })}
      data={{ "data-dialog-editor": "" }}
      actions={[{ label: t("dialogEditor.close"), kind: "primary", onClick: onClose }]}
    >
      <div className={styles.layout}>
        {notice !== null && <Notice kind={notice.kind}>{notice.text}</Notice>}
        {body}
      </div>
    </Dialog>
  );
}
