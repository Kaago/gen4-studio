import { useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { errorText, nameFlag } from "../api";
import type { FlagKind, NameReport } from "../api";
import { Dialog, Notice, TextField } from "../ui/primitives";
import styles from "./FlagsSurface.module.css";

/** The message after the dialog: a new own name is one step in the history; a description is saved. */
export function namedText(report: NameReport): string {
  return report.created ? `${report.id} benannt (ein Schritt im Verlauf).` : "Beschreibung gespeichert.";
}

export type NameFlagDialogProps = {
  kind: FlagKind;
  number: number;
  /** The pret name, which stays valid beside the own one; null when the game has none. */
  name: string | null;
  /** The own name and description; with an own name the dialog changes only the description. */
  current: { id: string | null; description: string | null };
  onClose: () => void;
  /** The name or description went in (one undo step); the dialog closes. */
  onDone: (report: NameReport) => void;
};

/**
 * „Als eigenes Flag benennen“ / „Als eigene Variable benennen“: the own name (filled with the kind's prefix) and an optional description;
 * with an own name „Beschreibung“, the description alone. „Benennen“ or „Speichern“ writes through `name_flag`; a refusal of the kernel
 * (the id's form, an id taken, the DSPRE layout) stands under the field and the dialog stays.
 */
export function NameFlagDialog({ kind, number, name, current, onClose, onDone }: NameFlagDialogProps) {
  const naming = current.id === null;
  const [id, setId] = useState(kind === "flag" ? "FLAG_" : "VAR_");
  const [description, setDescription] = useState(current.description ?? "");
  const [refusal, setRefusal] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const busy = useRef(false);

  const submit = async () => {
    if (busy.current) return;
    busy.current = true;
    setRunning(true);
    setRefusal(null);
    const text = description.trim();
    try {
      const report = naming
        ? await nameFlag({ kind, number, id: id.trim(), ...(text === "" ? {} : { description: text }) })
        : await nameFlag({ kind, number, description: text });
      onDone(report);
      onClose();
    } catch (e) {
      setRefusal(errorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  // Enter in the id field names like „Benennen“ (the key is used up, so the button that opened the dialog is not pressed again).
  const onEnter = (e: KeyboardEvent<HTMLDivElement>) => {
    if (e.key !== "Enter" || e.nativeEvent.isComposing) return;
    e.preventDefault();
    void submit();
  };

  const title = !naming ? "Beschreibung" : kind === "flag" ? "Als eigenes Flag benennen" : "Als eigene Variable benennen";
  const hint = name === null ? "A–Z, 0–9 und _." : `A–Z, 0–9 und _; der pret-Name ${name} bleibt gültig.`;
  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy.current) onClose();
      }}
      dismissible={!running}
      title={title}
      data={{ "data-name-flag-dialog": "" }}
      actions={[
        { label: "Abbrechen", onClick: onClose, disabled: running },
        { label: naming ? "Benennen" : "Speichern", kind: "primary", onClick: () => void submit(), disabled: running || (naming && id.trim() === "") },
      ]}
    >
      <div className={styles.dialog}>
        {naming && (
          <div onKeyDown={onEnter}>
            <TextField label="Eigener Name" showLabel value={id} onCommit={setId} onChange={setId} hint={hint} disabled={running} />
          </div>
        )}
        {naming && refusal !== null && <Notice kind="error">{refusal}</Notice>}
        <TextField label="Beschreibung" showLabel multiline value={description} onCommit={setDescription} onChange={setDescription} disabled={running} />
        {!naming && refusal !== null && <Notice kind="error">{refusal}</Notice>}
      </div>
    </Dialog>
  );
}
