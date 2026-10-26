import { useEffect, useRef, useState } from "react";
import { placeSlots, renamePlace } from "../../api";
import type { HeaderSummary } from "../../api";
import { Dialog, Notice, TextField } from "../../ui/primitives";
import { nameWarning } from "./placeChoices";
import styles from "./PlacePicker.module.css";

export interface RenamePlaceDialogProps {
  open: boolean;
  /** The place's index in the location bank (never 0: no place cannot be renamed). */
  index: number;
  /** The header list: the place's present name and how many headers carry it. */
  headers: readonly HeaderSummary[];
  onClose: () => void;
  /** The place was renamed (one undo step); `bank` is the location bank that changed (null while it was not known yet). */
  onRenamed: (bank: number | null) => void | Promise<void>;
  /** A refusal of the kernel (an empty name, an unknown character); the dialog stays open. */
  onError: (e: unknown) => void;
}

/** How far a rename reaches: every header with the place, and the place of capture of Pokémon caught there. */
function reachText(count: number): string {
  const what = count === 1 ? "1 Gebiet oder Raum" : `alle ${count} Gebiete und Räume`;
  return `Trifft ${what} mit diesem Ort und den Fundort gefangener Pokémon.`;
}

/** „Ort umbenennen“: a new name for the place at `index`, with how far the rename reaches. Mounted anew for every opening. */
export function RenamePlaceDialog(props: RenamePlaceDialogProps) {
  return props.open ? <RenameForm {...props} /> : null;
}

function RenameForm({ index, headers, onClose, onRenamed, onError }: RenamePlaceDialogProps) {
  const carriers = headers.filter((h) => h.location_index === index);
  const [name, setName] = useState(() => carriers[0]?.location_name ?? "");
  const [bank, setBank] = useState<number | null>(null);
  const [working, setWorking] = useState(false);
  const fail = useRef(onError);
  fail.current = onError;

  useEffect(() => {
    let live = true;
    placeSlots().then(
      (slots) => live && setBank(slots.bank),
      (e) => live && fail.current(e),
    );
    return () => {
      live = false;
    };
  }, []);

  async function rename() {
    if (working) return;
    setWorking(true);
    try {
      await renamePlace(index, name);
    } catch (e) {
      setWorking(false);
      onError(e);
      return;
    }
    onClose();
    await onRenamed(bank);
  }
  const leave = () => {
    if (!working) onClose();
  };
  const long = nameWarning(name);

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) leave();
      }}
      title="Ort umbenennen"
      actions={[
        { label: "Abbrechen", onClick: leave },
        { label: "Umbenennen", kind: "primary", onClick: () => void rename() },
      ]}
    >
      <div className={styles.form} data-rename-place="" aria-busy={working}>
        <TextField label="Name des Orts" showLabel value={name} onChange={setName} onCommit={setName} disabled={working} />
        <Notice kind="info">{reachText(carriers.length)}</Notice>
        {long && <Notice kind="warning">{long}</Notice>}
      </div>
    </Dialog>
  );
}
