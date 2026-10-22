import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { getEvents, updateEvent } from "../../api";
import type { BgDto, EventKind, HeaderSummary, ObjectDto, WarpDto } from "../../api";
import type { EventData } from "../../world/events";
import { KIND_LABELS, eventTitle, lockedText, warpDestinationLabel } from "../../world/events";
import { DraftInput } from "../DraftInput";
import { parseNumberField } from "./buildingFields";
import { AddEvent } from "./AddEvent";
import { EVENT_FIELDS, clampEventValue, eventBackend, eventFieldSpecs } from "./eventFields";
import type { EventFieldSpec } from "./eventFields";
import { EditSession } from "./headerCommit";
import { HeaderPicker } from "./HeaderPicker";

/** What the event editor needs from App besides the event's file, class and index. */
export interface EventEditorContext {
  headers: HeaderSummary[];
  /** Game family of the project ("Dp", "Pt", "Hgss"): decides which fields exist. */
  family: string | null;
  /** Changes whenever events may have changed behind the editor's back (a drag, undo, redo, add, delete). */
  revision: number;
  /** A field was written: the scene shows the file's events again and the dirty count follows. */
  onChanged: (eventFile: number) => void | Promise<void>;
  /** Adds an event of a class on the tile last hovered and selects it. */
  onAdd: (kind: EventKind) => void | Promise<void>;
  /** Asks for confirmation, then deletes the event. */
  onDelete: (eventFile: number, kind: EventKind, index: number) => void | Promise<void>;
  /** Selects a header in the list, as clicking it does. */
  onOpenHeader: (header: HeaderSummary) => void;
  onError: (e: unknown) => void;
}

const row: CSSProperties = { display: "grid", gridTemplateColumns: "minmax(0, 1fr) 96px", alignItems: "center", columnGap: 8, padding: "2px 0" };
const input: CSSProperties = { width: "100%", fontFamily: "ui-monospace, monospace" };
const muted: CSSProperties = { fontSize: 12, color: "#6b7280" };

/**
 * Editable form of one event: its fields by class, a warp's destination header as a searchable
 * picker with "Zum Ziel", add and delete. Each committed field is one undoable change. Render it
 * with `key` per (file, class, index): all reads and writes of an instance belong to that one event
 * (see `EditSession`). A locked event file is shown with its reason and cannot be changed.
 */
export function EventEditor({ file, kind, index, context }: { file: number; kind: EventKind; index: number; context: EventEditorContext }) {
  const { headers, family, revision, onChanged, onAdd, onDelete, onOpenHeader, onError } = context;
  const [event, setEvent] = useState<EventData | null>(null);
  const [locked, setLocked] = useState<string | null>(null);
  // Bumped when an edit is refused, so the fields drop the rejected text they still show.
  const [refused, setRefused] = useState(0);
  const callbacks = useRef({ onChanged, onError });
  callbacks.current = { onChanged, onError };
  const session = useRef<EditSession<EventData> | null>(null);

  useEffect(() => {
    const backend = eventBackend(file, kind, { getEvents, update: updateEvent, onEvents: (events) => setLocked(events.locked) });
    const s = new EditSession<EventData>(index, backend, EVENT_FIELDS, {
      onState: setEvent,
      onRefused: () => setRefused((n) => n + 1),
      onChanged: () => callbacks.current.onChanged(file),
      onError: (e) => callbacks.current.onError(e),
    });
    session.current = s;
    return () => s.dispose();
  }, [file, kind, index]);

  useEffect(() => {
    void session.current?.load();
  }, [file, kind, index, revision]);

  const commit = (key: string, label: string, value: number) => void session.current?.commit(key, label, value);
  const commitSpec = (spec: EventFieldSpec) => (text: string) => {
    const v = parseNumberField(text);
    if (v !== null) commit(spec.key, spec.label, clampEventValue(spec, v));
  };

  const title = `${KIND_LABELS[kind]} ${index} · Event-Datei ${String(file).padStart(4, "0")}`;
  if (event === null) return <div style={{ padding: 16, fontSize: 13, color: "#6b7280" }}>{title}</div>;
  const readOnly = locked !== null;
  const warp = kind === "warp" ? (event as WarpDto) : null;
  const destination = warp ? warpDestinationLabel(warp.dest_header_id, headers) : null;
  const destHeader = warp ? headers.find((h) => h.id === warp.dest_header_id) : undefined;

  return (
    <div style={{ padding: 16, fontFamily: "system-ui, sans-serif", fontSize: 13, color: "#111827" }}>
      <div style={{ fontSize: 14, fontWeight: 600 }}>{eventTitle(kind, event, destHeader?.internal_name)}</div>
      <div style={{ ...muted, marginTop: 2 }}>{title}</div>
      {locked !== null && (
        <div role="status" style={{ marginTop: 8, color: "#b91c1c" }}>
          {lockedText(locked)}
        </div>
      )}
      <div key={refused} style={{ marginTop: 8 }}>
        {eventFieldSpecs(kind, family).map((spec) => (
          <label key={spec.key} style={row}>
            <span>{spec.label}</span>
            <DraftInput
              label={spec.label}
              type="number"
              min={spec.min}
              max={spec.max}
              value={String(EVENT_FIELDS.value(event, spec.key))}
              onCommit={commitSpec(spec)}
              disabled={readOnly}
              style={input}
            />
          </label>
        ))}
        {(kind === "object" || kind === "bg") && (
          <div style={row} title="Die Höhe lässt sich hier nicht ändern; sie bleibt unverändert erhalten.">
            <span>y (nur lesbar)</span>
            <span style={{ fontFamily: "ui-monospace, monospace" }}>{(event as ObjectDto | BgDto).y}</span>
          </div>
        )}
      </div>
      {warp && destination && (
        <div style={{ marginTop: 8 }}>
          <div style={{ display: "flex", alignItems: "center", gap: 8, marginBottom: 4 }}>
            <span style={{ flex: 1, minWidth: 0, color: destination.known ? "#111827" : "#b91c1c" }}>Ziel: {destination.text}</span>
            <button type="button" disabled={destHeader === undefined} onClick={() => destHeader && onOpenHeader(destHeader)} title="Wählt den Zielheader in der Liste">
              Zum Ziel
            </button>
          </div>
          <HeaderPicker headers={headers} current={warp.dest_header_id} disabled={readOnly} onPick={(id) => commit("dest_header_id", "Zielheader", id)} />
        </div>
      )}
      <div style={{ display: "flex", gap: 8, marginTop: 12, alignItems: "center" }}>
        <AddEvent disabled={readOnly} onAdd={(k) => void onAdd(k)} />
        <button type="button" disabled={readOnly} onClick={() => void onDelete(file, kind, index)}>
          Löschen
        </button>
      </div>
      <p style={{ marginTop: 8, ...muted }}>
        Ziehen in der 3D-Ansicht verschiebt das Event um ganze Kacheln, Esc bricht das Ziehen ab. Neue Events entstehen auf der zuletzt berührten Kachel; Entf löscht das gewählte Event nach Rückfrage.
      </p>
    </div>
  );
}
