import { useMemo, useState } from "react";
import { editTextArchive, errorText, splitMessage } from "../../api";
import type { DialogMessage } from "../../api";
import type { ScriptFiles } from "../../data/useDataArea";
import { Badge, Button, DSScreen, Dialog, Notice } from "../../ui/primitives";
import { MessageEditor } from "../text/MessageEditor";
import { pages } from "../text/textModel";
import { dialogSummary, distinctMessages, sharedText } from "./dialogModel";
import type { DialogTarget } from "./dialogModel";
import { useDialog } from "./useDialog";
import styles from "./DialogEditor.module.css";

/** The store label of a text written here (one undo step per commit, „Dialog geändert“ in the history). */
const EDIT_LABEL = "Dialog geändert";

const keyOf = (m: Pick<DialogMessage, "archive" | "index">) => `${m.archive}:${m.index}`;

/** „wenn …“ (or „sonst“) for a message in a branch of the script. */
const branchText = (branch: string) => (branch === "else" ? "sonst" : `wenn ${branch}`);

/**
 * „Nachricht 4 · Archiv 554“, with „wenn …“ (or „sonst“) for a message in a branch of the script and „auch Zeile 9 (wenn …)“ for the
 * other places the script shows the same message.
 */
function messageHead(m: DialogMessage, others: readonly DialogMessage[]): string {
  let head = `Nachricht ${m.index} · Archiv ${m.archive}`;
  if (m.branch !== null) head += ` · ${branchText(m.branch)}`;
  if (others.length === 0) return head;
  const uses = others.map((o) => (o.branch === null ? `Zeile ${o.line}` : `Zeile ${o.line} (${branchText(o.branch)})`));
  return `${head} · auch ${uses.join(", ")}`;
}

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
  const { dialog, error, reload } = useDialog(target, revision);
  const [chosen, setChosen] = useState<string | null>(null);
  const [page, setPage] = useState(0);
  // What is typed into a field before it is committed: the screen follows it.
  const [draft, setDraft] = useState<{ key: string; text: string } | null>(null);
  const [notice, setNotice] = useState<{ kind: "error" | "success"; text: string } | null>(null);
  const [splitting, setSplitting] = useState(false);

  const source = dialog?.source ?? null;
  // Each message once (its first use): a message the script shows twice is one field.
  const distinct = useMemo(() => (source?.kind === "local" ? distinctMessages(dialog!.messages) : []), [dialog, source?.kind]);
  const messages = distinct.map((d) => d.message);
  const current = messages.find((m) => keyOf(m) === chosen) ?? messages[0] ?? null;
  const currentKey = current === null ? null : keyOf(current);
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

  const commit = async (m: DialogMessage, written: string): Promise<boolean> => {
    try {
      await editTextArchive(m.archive, { Set: { index: m.index, text: written } }, EDIT_LABEL);
    } catch (e) {
      setNotice({ kind: "error", text: errorText(e) });
      return false;
    }
    setNotice(null);
    onChanged();
    reload();
    return true;
  };

  const split = async (m: DialogMessage) => {
    if (source?.kind !== "local") return;
    const file = source.file;
    setSplitting(true);
    try {
      try {
        await scriptFiles?.saveIfDirty(file);
      } catch (e) {
        onError(e);
        return;
      }
      const report = await splitMessage(target.header, target.eventFile, target.index, m.archive, m.index);
      onChanged();
      void scriptFiles?.reload(file);
      setChosen(keyOf({ archive: report.archive, index: report.new_index }));
      setPage(0);
      setNotice({ kind: "success", text: `Nachricht ${report.new_index} ist jetzt nur diese Figur.` });
      reload();
    } catch (e) {
      setNotice({ kind: "error", text: errorText(e) });
    } finally {
      setSplitting(false);
    }
  };

  let body;
  if (error !== null) {
    body = <Notice kind="error">{error}</Notice>;
  } else if (dialog === null) {
    body = <p className={styles.note}>Lese …</p>;
  } else if (source?.kind !== "local") {
    body = <Notice kind="info">{dialogSummary(dialog)}</Notice>;
  } else if (messages.length === 0) {
    body = <p className={styles.note}>Das Script dieser Figur zeigt keine Nachricht.</p>;
  } else {
    body = (
      <div className={styles.columns}>
        <ol className={styles.messages} aria-label="Nachrichten">
          {distinct.map(({ message: m, others }) => {
            const key = keyOf(m);
            const selected = key === currentKey;
            return (
              <li key={key} className={styles.message} data-dialog-message={key} data-selected={selected ? "" : undefined} onFocus={() => choose(key)}>
                <div className={styles.head}>
                  <Button kind="quiet" size="s" aria-pressed={selected} onClick={() => choose(key)}>
                    {messageHead(m, others)}
                  </Button>
                  {m.shared_with > 0 && (
                    <Badge kind="warning" icon="shared">
                      {sharedText(m.shared_with)}
                    </Badge>
                  )}
                </div>
                {m.text === null ? (
                  <p className={styles.note}>{`Archiv ${m.archive} hat keine Nachricht ${m.index}.`}</p>
                ) : (
                  <MessageEditor key={key} value={m.text} onCommit={(written) => commit(m, written)} onDraft={(typed) => setDraft({ key, text: typed })} />
                )}
                {/* Only an object's script can be given its own copy (a sign shares its text where it is written), and only of a message the archive has.
                    A message the kernel would refuse (`split_refusal`: another figure runs the script, a shared jump target) keeps the key
                    disabled with the reason under it. */}
                {m.shared_with > 0 && target.kind === "object" && m.text !== null && (
                  <div className={styles.split} data-split-refusal={m.split_refusal === null ? undefined : ""}>
                    <Button kind="secondary" size="s" disabled={splitting || m.split_refusal !== null} aria-describedby={m.split_refusal === null ? undefined : `${key}-split`} onClick={() => void split(m)}>
                      Für diese Figur abspalten
                    </Button>
                    {m.split_refusal !== null && (
                      <p id={`${key}-split`} className={styles.note}>
                        {m.split_refusal}
                      </p>
                    )}
                  </div>
                )}
              </li>
            );
          })}
        </ol>
        <div className={styles.preview}>
          <span className={styles.caption}>{current === null ? "" : `Nachricht ${current.index}`}</span>
          <DSScreen variant={variant} pages={shown} page={pageIndex} scale={1} />
          <div className={styles.pager}>
            <Button kind="quiet" size="s" aria-label="Vorige Seite" disabled={pageIndex === 0} onClick={() => setPage(pageIndex - 1)}>
              ‹
            </Button>
            <span className={styles.page}>{`Seite ${pageIndex + 1} von ${total}`}</span>
            <Button kind="quiet" size="s" aria-label="Nächste Seite" disabled={pageIndex >= total - 1} onClick={() => setPage(pageIndex + 1)}>
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
      title={`Dialog von ${target.title}`}
      data={{ "data-dialog-editor": "" }}
      actions={[{ label: "Schließen", kind: "primary", onClick: onClose }]}
    >
      <div className={styles.layout}>
        {notice !== null && <Notice kind={notice.kind}>{notice.text}</Notice>}
        {body}
      </div>
    </Dialog>
  );
}
