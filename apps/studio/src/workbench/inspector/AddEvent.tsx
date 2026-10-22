import { useState } from "react";
import type { CSSProperties } from "react";
import type { EventKind } from "../../api";
import { EVENT_KINDS, KIND_LABELS } from "../../world/events";

/** Picks the class of a new event and asks for it; it appears on the tile the pointer was last over. */
export function AddEvent({ disabled, title, onAdd, style }: { disabled?: boolean; title?: string; onAdd: (kind: EventKind) => void; style?: CSSProperties }) {
  const [kind, setKind] = useState<EventKind>("warp");
  return (
    <span title={title} style={{ display: "inline-flex", gap: 4, alignItems: "center", ...style }}>
      <select aria-label="Art des neuen Events" value={kind} disabled={disabled} onChange={(e) => setKind(e.target.value as EventKind)}>
        {EVENT_KINDS.map((k) => (
          <option key={k} value={k}>
            {KIND_LABELS[k]}
          </option>
        ))}
      </select>
      <button type="button" disabled={disabled} onClick={() => onAdd(kind)} title="Legt das Event auf der Kachel an, über der die Maus zuletzt war">
        Hinzufügen
      </button>
    </span>
  );
}
