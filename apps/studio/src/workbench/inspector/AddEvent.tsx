import { useState } from "react";
import type { EventKind } from "../../api";
import { Button, SelectField } from "../../ui/primitives";
import { EVENT_KINDS, KIND_LABELS } from "../../world/events";
import styles from "./AddEvent.module.css";

const OPTIONS = EVENT_KINDS.map((k) => ({ value: k, label: KIND_LABELS[k] }));

/** Picks the class of a new event and asks for it; it appears on the tile the pointer was last over. */
export function AddEvent({ disabled, onAdd }: { disabled?: boolean; onAdd: (kind: EventKind) => void }) {
  const [kind, setKind] = useState<EventKind>("warp");
  return (
    <span className={styles.add}>
      <SelectField label="Art des neuen Events" value={kind} options={OPTIONS} disabled={disabled} onChange={setKind} />
      <Button kind="quiet" size="s" disabled={disabled} onClick={() => onAdd(kind)} title="Legt das Event auf der Kachel an, über der die Maus zuletzt war">
        Hinzufügen
      </Button>
    </span>
  );
}
