import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { getHeader, headerFamily, setHeader } from "../../api";
import type { HeaderSummary, MapHeader, MemberCounts } from "../../api";
import { headerLabel } from "../headers";
import { clampField, fieldValue, headerFieldSpecs, pickMax, withField } from "./fields";
import type { FieldSpec } from "./fields";

/** What the header editor needs from App besides the header id. */
export interface HeaderEditorContext {
  headers: HeaderSummary[];
  counts: MemberCounts;
  /** Changes whenever the header table was changed behind the editor's back (undo, redo, reload). */
  revision: number;
  /** Called after a field was written, so the list, the scene and the dirty count follow. */
  onChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
}

const row: CSSProperties = { display: "grid", gridTemplateColumns: "minmax(0, 1fr) 96px", alignItems: "center", columnGap: 8, padding: "2px 0" };
const input: CSSProperties = { width: "100%", boxSizing: "border-box", padding: "2px 6px", fontFamily: "ui-monospace, monospace" };

interface FieldProps {
  spec: FieldSpec;
  value: number;
  max: number | undefined;
  onCommit: (spec: FieldSpec, value: number) => void;
}

/** One header field. Numbers are committed on blur or Enter, never on every keystroke. */
function Field({ spec, value, max, onCommit }: FieldProps) {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);

  const commit = () => {
    const parsed = draft.trim() === "" ? Number.NaN : Number(draft);
    const next = clampField(spec, parsed, max);
    // Show what will be stored (clamped), also when nothing changes.
    setDraft(String(next));
    if (next !== value) onCommit(spec, next);
  };

  const hint = spec.kind === "pick" && max !== undefined ? ` (0 bis ${max})` : "";
  return (
    <label style={row} title={`${spec.key}${hint}`}>
      <span>{spec.label}</span>
      {spec.kind === "bool" ? (
        <input type="checkbox" checked={value !== 0} onChange={(e) => onCommit(spec, e.target.checked ? 1 : 0)} style={{ justifySelf: "start" }} />
      ) : (
        <input
          type="number"
          inputMode="numeric"
          min={0}
          max={max}
          value={draft}
         
          onChange={(e) => setDraft(e.target.value)}
          onBlur={commit}
          onKeyDown={(e) => {
            if (e.key === "Enter") e.currentTarget.blur();
            else if (e.key === "Escape") setDraft(String(value));
          }}
          style={input}
        />
      )}
    </label>
  );
}

/** Editable form of one map header; every committed field is one undoable change. */
export function HeaderEditor({ id, context }: { id: number; context: HeaderEditorContext }) {
  const { headers, counts, revision, onChanged, onError } = context;
  const [header, setHeaderState] = useState<MapHeader | null>(null);
  // Bumped when an edit is refused, so the fields drop the rejected text they still show.
  const [refused, setRefused] = useState(0);
  const shownId = useRef<number | null>(null);
  const current = useRef<MapHeader | null>(null);
  // Commits run one after the other so a quick second edit builds on the first one's result.
  const queue = useRef<Promise<void>>(Promise.resolve());
  const callbacks = useRef({ onChanged, onError });
  callbacks.current = { onChanged, onError };

  useEffect(() => {
    let cancelled = false;
    // Keep showing the old values while an undo/redo refetch is under way; another header starts empty.
    if (shownId.current !== id) {
      current.current = null;
      setHeaderState(null);
      shownId.current = id;
    }
    getHeader(id)
      .then((h) => {
        if (cancelled) return;
        current.current = h;
        setHeaderState(h);
      })
      .catch((e) => {
        if (!cancelled) callbacks.current.onError(e);
      });
    return () => {
      cancelled = true;
    };
  }, [id, revision]);

  const commit = (spec: FieldSpec, value: number) => {
    queue.current = queue.current.then(async () => {
      const base = current.current;
      if (!base || fieldValue(base, spec.key) === value) return;
      try {
        const next = withField(base, spec.key, value);
        await setHeader(id, next, spec.label);
        current.current = next;
        setHeaderState(next);
        await callbacks.current.onChanged();
      } catch (e) {
        // The edit was refused: show the stored value again.
        setRefused((n) => n + 1);
        callbacks.current.onError(e);
      }
    });
  };

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
      <p style={{ marginTop: 12, fontSize: 12, color: "#6b7280" }}>Neue Header lassen sich in Vanilla-ROMs nicht anlegen.</p>
    </div>
  );
}
