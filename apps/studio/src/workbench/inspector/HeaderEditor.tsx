import { useEffect, useRef, useState } from "react";
import { getHeader, headerFamily, setHeader } from "../../api";
import type { HeaderSummary, MapHeader, MemberCounts } from "../../api";
import { headerLabel } from "../headers";
import { Field } from "./Field";
import { fieldValue, headerFieldSpecs, pickMax } from "./fields";
import type { FieldSpec } from "./fields";
import { HeaderSession } from "./headerCommit";

/** What the header editor needs from App besides the header id. */
export interface HeaderEditorContext {
  headers: HeaderSummary[];
  counts: MemberCounts;
  /** Changes whenever the header table was changed behind the editor's back (undo, redo, reload). */
  revision: number;
  /** Called after a field was written, so the list, the scene and the dirty count follow. */
  onChanged: () => void | Promise<void>;
  /** Shows area data `id` in the inspector. */
  onOpenArea: (id: number) => void;
  onError: (e: unknown) => void;
}

/**
 * Editable form of one map header; every committed field is one undoable change. Render it with
 * `key={id}`: all reads and writes of an instance belong to its one header (see `HeaderSession`).
 */
export function HeaderEditor({ id, context }: { id: number; context: HeaderEditorContext }) {
  const { headers, counts, revision, onChanged, onOpenArea, onError } = context;
  const [header, setHeaderState] = useState<MapHeader | null>(null);
  // Bumped when an edit is refused, so the fields drop the rejected text they still show.
  const [refused, setRefused] = useState(0);
  const callbacks = useRef({ onChanged, onError });
  callbacks.current = { onChanged, onError };
  const session = useRef<HeaderSession | null>(null);

  // One session per mount. Disposing only cuts it off from this editor's state: a commit that
  // is already queued (also an uncommitted field flushed on unmount) still writes its own header.
  useEffect(() => {
    const s = new HeaderSession(
      id,
      { getHeader, setHeader },
      {
        onState: setHeaderState,
        onRefused: () => setRefused((n) => n + 1),
        onChanged: () => callbacks.current.onChanged(),
        onError: (e) => callbacks.current.onError(e),
      },
    );
    session.current = s;
    return () => s.dispose();
  }, [id]);

  // Load, and reload after undo/redo (queued behind pending commits). Keeps showing the old
  // values meanwhile.
  useEffect(() => {
    void session.current?.load();
  }, [id, revision]);

  const commit = (spec: FieldSpec, value: number) => void session.current?.commit(spec.key, spec.label, value);

  const summary = headers.find((h) => h.id === id);
  const title = `Header ${String(id).padStart(4, "0")}${summary ? ` · ${headerLabel(summary)}` : ""}`;
  return (
    <div style={{ padding: 16, fontFamily: "system-ui, sans-serif", fontSize: 13, color: "#111827" }}>
      <div style={{ fontSize: 14, fontWeight: 600 }}>{title}</div>
      {header === null ? (
        <p style={{ color: "#6b7280" }}>Lade Header ...</p>
      ) : (
        <div key={refused} style={{ marginTop: 8 }}>
          {headerFieldSpecs(headerFamily(header)).map((spec) => (
            <Field
              key={spec.key}
              spec={spec}
              value={fieldValue(header, spec.key)}
              max={spec.kind === "pick" && spec.pick ? pickMax(spec.pick, counts) : undefined}
              onCommit={commit}
            />
          ))}
        </div>
      )}
      {header !== null && (
        <button type="button" style={{ marginTop: 8 }} onClick={() => onOpenArea(fieldValue(header, "area_data_id"))}>
          Area-Data {String(fieldValue(header, "area_data_id")).padStart(4, "0")} öffnen
        </button>
      )}
      <p style={{ marginTop: 12, fontSize: 12, color: "#6b7280" }}>Neue Header lassen sich in Vanilla-ROMs nicht anlegen.</p>
    </div>
  );
}
