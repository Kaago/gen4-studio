import { useState } from "react";
import { splitRoom } from "../../api";
import type { SplitReport } from "../../api";
import { Badge, Dialog } from "../../ui/primitives";
import styles from "./SplitRoomDialog.module.css";

/** What splitting does to the room `name` with `maps` maps (null: not known, so no number), and that the copies stay. */
function splitText(name: string, maps: number | null): string {
  const copies = maps === null ? "eigene Kopien seiner Karten" : maps === 1 ? "eine eigene Kopie seiner Karte" : `eigene Kopien seiner ${maps} Karten`;
  const stay = maps === 1 ? "Die Kopie lässt sich nicht zurücknehmen." : "Die Kopien lassen sich nicht zurücknehmen.";
  return `Der Raum „${name}“ bekommt ${copies}. Danach ändern Änderungen an ihm keinen anderen Raum mehr. ${stay}`;
}

export interface SplitRoomDialogProps {
  open: boolean;
  /** The room's header. */
  header: number;
  /** The room's name, as the world tree calls it (also the label of the undo step). */
  name: string;
  /** How many maps the room has (each gets a copy); null when that is not known: the text names no number. */
  maps: number | null;
  onClose: () => void;
  /** The room has copies of its maps on a matrix of its own (`report`); the dialog has closed. */
  onSplit: (report: SplitReport) => void | Promise<void>;
  /** A refusal of the kernel (the room shares nothing, a world map) or a failure; the dialog stays open. */
  onError: (e: unknown) => void;
}

/** „Raum abspalten?“: gives a shared room copies of its maps; the copies cannot be undone. Mounted anew for every opening. */
export function SplitRoomDialog(props: SplitRoomDialogProps) {
  return props.open ? <SplitForm {...props} /> : null;
}

function SplitForm({ header, name, maps, onClose, onSplit, onError }: SplitRoomDialogProps) {
  const [working, setWorking] = useState(false);

  async function split() {
    if (working) return;
    setWorking(true);
    let report: SplitReport;
    try {
      report = await splitRoom(header, name);
    } catch (e) {
      setWorking(false);
      onError(e);
      return;
    }
    onClose();
    try {
      await onSplit(report);
    } catch (e) {
      onError(e);
    }
  }
  const leave = () => {
    if (!working) onClose();
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open) leave();
      }}
      title="Raum abspalten?"
      actions={[
        { label: "Abbrechen", onClick: leave },
        { label: "Abspalten", kind: "primary", onClick: () => void split(), disabled: working },
      ]}
    >
      <div className={styles.form} data-split-room="" aria-busy={working}>
        <Badge kind="irreversible" />
        <p className={styles.text}>{splitText(name, maps)}</p>
      </div>
    </Dialog>
  );
}
