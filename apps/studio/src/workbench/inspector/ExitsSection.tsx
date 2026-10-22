import type { CSSProperties } from "react";
import type { HeaderSummary } from "../../api";
import { KIND_LABELS, lockedText, warpDestinationLabel } from "../../world/events";
import type { NavTarget } from "../navigation";
import { TERMS } from "../terms";
import type { WizardRequest } from "../WorkbenchContext";
import { useEvents } from "../useEvents";

interface Props {
  header: HeaderSummary;
  headers: HeaderSummary[];
  /** Changes whenever events were changed behind the section's back (a drag, add, delete, undo, redo); it reads them again. */
  revision: number;
  goTo: (target: NavTarget) => void;
  onOpenWizard: (request: WizardRequest) => void;
  /** Adds a warp to the header's event file (the app decides where). */
  onAddWarp: () => void;
  onError: (e: unknown) => void;
}

const heading: CSSProperties = { fontWeight: 600, marginBottom: 4 };
const row: CSSProperties = { display: "flex", alignItems: "center", gap: 8, margin: "2px 0" };

/** The "Ausgänge" of the header editor: the warps of the header's event file, where they lead, and the ways to add one. */
export function ExitsSection({ header, headers, revision, goTo, onOpenWizard, onAddWarp, onError }: Props) {
  const events = useEvents(header.event_file_id, onError, revision);
  const locked = events?.locked ?? null;
  return (
    <section aria-label={TERMS.warp.plural} style={{ marginTop: 12 }}>
      <div style={heading}>{TERMS.warp.plural}</div>
      {events === null ? (
        <p style={{ color: "#6b7280", margin: "4px 0" }}>Lade …</p>
      ) : events.warps.length === 0 ? (
        <p style={{ color: "#6b7280", margin: "4px 0" }}>Keine Ausgänge.</p>
      ) : (
        <ul style={{ listStyle: "none", margin: 0, padding: 0 }}>
          {events.warps.map((w) => {
            const dest = warpDestinationLabel(w.dest_header_id, headers);
            return (
              <li key={w.index} style={row}>
                <span style={{ flex: 1, minWidth: 0 }}>
                  {KIND_LABELS.warp} {w.index} (x {w.x}, z {w.z}) → {dest.text}
                </span>
                <button type="button" disabled={!dest.known} onClick={() => goTo({ kind: "header", id: w.dest_header_id })}>
                  Zum Ziel
                </button>
                <button type="button" onClick={() => goTo({ kind: "event", eventFile: header.event_file_id, eventKind: "warp", index: w.index, header: header.id })}>
                  Bearbeiten
                </button>
              </li>
            );
          })}
        </ul>
      )}
      <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 8 }}>
        <button type="button" onClick={() => onOpenWizard({ kind: "room", prefill: { outsideHeader: header.id } })}>
          {TERMS.room.label} hier anbinden…
        </button>
        <button type="button" disabled={locked !== null} title={locked === null ? "Legt einen Warp auf der Kachel an, über der die Maus zuletzt war" : lockedText(locked)} onClick={onAddWarp}>
          Warp hinzufügen
        </button>
      </div>
    </section>
  );
}
