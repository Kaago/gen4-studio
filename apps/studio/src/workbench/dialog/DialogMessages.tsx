import { useId, useState } from "react";
import { useTranslation } from "react-i18next";
import { editTextArchive, errorText, splitMessage } from "../../api";
import type { DialogMessage, DialogSource } from "../../api";
import type { ScriptFiles } from "../../data/useDataArea";
import { t } from "../../i18n";
import { Badge, Button } from "../../ui/primitives";
import { MessageEditor } from "../text/MessageEditor";
import { sharedText } from "./dialogModel";
import type { DialogTarget, DistinctMessage } from "./dialogModel";
import styles from "./DialogMessages.module.css";

/** The store label of a text written here (one undo step per commit, „Dialog geändert“ in the history), in the language that is shown. */
const editLabel = () => t("workbench:dialogMessages.editLabel");

/** A message's key: „archive:index“. */
export const messageKey = (m: Pick<DialogMessage, "archive" | "index">) => `${m.archive}:${m.index}`;

/** „wenn …“ (or „sonst“) for a message in a branch of the script. */
const branchText = (branch: string) => (branch === "else" ? t("workbench:dialogMessages.else") : t("workbench:dialogMessages.if", { branch }));

/**
 * „Nachricht 4 · Archiv 554“, with „wenn …“ (or „sonst“) for a message in a branch of the script and „auch Zeile 9 (wenn …)“ for the
 * other places the script shows the same message.
 */
function messageHead(m: DialogMessage, others: readonly DialogMessage[]): string {
  let head = t("workbench:dialogMessages.head", { index: m.index, archive: m.archive });
  if (m.branch !== null) head += ` · ${branchText(m.branch)}`;
  if (others.length === 0) return head;
  const uses = others.map((o) =>
    o.branch === null ? t("workbench:dialogMessages.line", { line: o.line }) : t("workbench:dialogMessages.lineInBranch", { line: o.line, branch: branchText(o.branch) }),
  );
  return t("workbench:dialogMessages.also", { head, uses: uses.join(", ") });
}

/** What the writes of a dialog need from their owner (the dialog editor, the hand of the texts). */
export interface DialogWriteHooks {
  /** Reads the dialog again (after a commit or a split). */
  reload: () => void;
  /** A text or a split was written: the app counts the edit and tells the text panel. */
  onChanged: () => void;
  /** A failure that is not a refusal of the edit (saving the open script document before a split). */
  onError: (e: unknown) => void;
  /** The script panel's documents: a split writes the figure's script file, so typing there is saved first and the document reloaded after. */
  scriptFiles?: ScriptFiles;
  /** A split made message `key` the figure's own copy: the owner chooses it. */
  onSplit: (key: string) => void;
  /** Erzählen: what the writes say names no message number. */
  narrating?: boolean;
}

/** The writes of a figure's dialog: a text, a split, and what the last one said (a refusal, the split done). */
export interface DialogWrites {
  /** Writes `text` into message `m` (one undo step „Dialog geändert“); resolves to whether the store took it. */
  commit: (m: DialogMessage, text: string) => Promise<boolean>;
  /** Gives the figure its own copy of message `m`. */
  split: (m: DialogMessage) => Promise<void>;
  /** A split is running (the keys wait). */
  splitting: boolean;
  notice: { kind: "error" | "success"; text: string } | null;
}

/**
 * The writes of the dialog of `target` from `source`, shared by the dialog editor and the hand of the texts: neither keeps a text of its
 * own, every commit goes to the store (`editTextArchive`) and the dialog is read again.
 */
export function useDialogWrites(target: DialogTarget, source: DialogSource | null, hooks: DialogWriteHooks): DialogWrites {
  const [notice, setNotice] = useState<DialogWrites["notice"]>(null);
  const [splitting, setSplitting] = useState(false);
  const { reload, onChanged, onError, scriptFiles, onSplit, narrating = false } = hooks;

  const commit = async (m: DialogMessage, written: string): Promise<boolean> => {
    try {
      await editTextArchive(m.archive, { Set: { index: m.index, text: written } }, editLabel());
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
      onSplit(messageKey({ archive: report.archive, index: report.new_index }));
      setNotice({ kind: "success", text: narrating ? t("workbench:dialogMessages.splitDoneNarrating") : t("workbench:dialogMessages.splitDone", { index: report.new_index }) });
      reload();
    } catch (e) {
      setNotice({ kind: "error", text: errorText(e) });
    } finally {
      setSplitting(false);
    }
  };

  return { commit, split, splitting, notice };
}

export interface DialogMessagesProps {
  /** The figure's class: only an object's script can be given its own copy of a message. */
  kind: DialogTarget["kind"];
  /** The dialog's messages, each once (`distinctMessages`). */
  distinct: readonly DistinctMessage[];
  /** The chosen message (`messageKey`); its block is marked. */
  currentKey: string | null;
  onChoose: (key: string) => void;
  /** Hears the typing in a message's field before it is committed (a preview that follows it). */
  onDraft?: (key: string, text: string) => void;
  writes: DialogWrites;
  /** Erzählen: the heads count the messages („Nachricht 1 von 2“) instead of naming archive, number, condition and lines. */
  narrating?: boolean;
}

/**
 * The messages of a figure's dialog in reading order (the left column of the dialog editor, and the hand of the texts in Erzählen): each in
 * the field of the texts with its head, the badge „Geteilt mit N Figuren“ and „Für diese Figur abspalten“ when other figures show it too.
 * In Erzählen (`narrating`) the head only counts („Nachricht 1 von 2“): archives, numbers and script conditions are Bauen's.
 */
export function DialogMessages({ kind, distinct, currentKey, onChoose, onDraft, writes, narrating = false }: DialogMessagesProps) {
  // Renders the list again when the language changes; the heads are read at render time.
  useTranslation("workbench");
  const { commit, split, splitting } = writes;
  // The editor and the hand may show the same figure at once: the ids of the refusals are this list's own.
  const id = useId();
  return (
    <ol className={styles.messages} aria-label={t("workbench:dialogMessages.messages")}>
      {distinct.map(({ message: m, others }, position) => {
        const key = messageKey(m);
        const selected = key === currentKey;
        const refusalId = `${id}${key}-split`;
        return (
          <li key={key} className={styles.message} data-dialog-message={key} data-selected={selected ? "" : undefined} onFocus={() => onChoose(key)}>
            <div className={styles.head}>
              <Button kind="quiet" size="s" aria-pressed={selected} onClick={() => onChoose(key)}>
                <span className={styles.headText}>{narrating ? t("workbench:dialogMessages.position", { n: position + 1, total: distinct.length }) : messageHead(m, others)}</span>
              </Button>
              {m.shared_with > 0 && (
                <Badge kind="warning" icon="shared">
                  {sharedText(m.shared_with)}
                </Badge>
              )}
            </div>
            {m.text === null ? (
              <p className={styles.note}>{narrating ? t("workbench:dialogMessages.missingNarrating") : t("workbench:dialogMessages.missing", { archive: m.archive, index: m.index })}</p>
            ) : (
              <MessageEditor key={key} value={m.text} onCommit={(written) => commit(m, written)} onDraft={onDraft && ((typed) => onDraft(key, typed))} />
            )}
            {/* Only an object's script can be given its own copy (a sign shares its text where it is written), and only of a message the archive has.
                A message the kernel would refuse (`split_refusal`: another figure runs the script, a shared jump target) keeps the key
                disabled with the reason under it. */}
            {m.shared_with > 0 && kind === "object" && m.text !== null && (
              <div className={styles.split} data-split-refusal={m.split_refusal === null ? undefined : ""}>
                <Button kind="secondary" size="s" disabled={splitting || m.split_refusal !== null} aria-describedby={m.split_refusal === null ? undefined : refusalId} onClick={() => void split(m)}>
                  {t("workbench:dialogMessages.split")}
                </Button>
                {m.split_refusal !== null && (
                  <p id={refusalId} className={styles.note}>
                    {m.split_refusal}
                  </p>
                )}
              </div>
            )}
          </li>
        );
      })}
    </ol>
  );
}
