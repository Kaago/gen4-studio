import { useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import { draftToValue } from "./fields";
import type { FieldSpec } from "./fields";

const row: CSSProperties = { display: "grid", gridTemplateColumns: "minmax(0, 1fr) 96px", alignItems: "center", columnGap: 8, padding: "2px 0" };
const input: CSSProperties = { width: "100%", boxSizing: "border-box", padding: "2px 6px", fontFamily: "ui-monospace, monospace" };

export interface FieldProps {
  spec: FieldSpec;
  value: number;
  max: number | undefined;
  onCommit: (spec: FieldSpec, value: number) => void;
}

/** One record field. Numbers are committed on blur or Enter, never on every keystroke. */
export function Field({ spec, value, max, onCommit }: FieldProps) {
  const [draft, setDraft] = useState(String(value));
  useEffect(() => setDraft(String(value)), [value]);

  const latest = useRef({ draft, value, max, spec, onCommit });
  latest.current = { draft, value, max, spec, onCommit };

  const pending = () => {
    const { draft, spec, max } = latest.current;
    return draftToValue(draft, spec, max);
  };

  const commit = () => {
    const next = pending();
    // A blank or non-numeric draft changes nothing; otherwise show what will be stored (clamped).
    setDraft(String(next ?? value));
    if (next !== null && next !== value) onCommit(spec, next);
  };

  // The inspector can swap the form for tile info while a field is still being typed in (hover
  // does not blur it): write what was typed instead of losing it. The session belongs to this
  // record, so this cannot reach another one.
  useEffect(
    () => () => {
      if (spec.kind === "bool" || spec.kind === "select") return;
      const next = pending();
      const { value, spec: s, onCommit } = latest.current;
      if (next !== null && next !== value) onCommit(s, next);
    },
    [],
  );

  const hint = spec.kind === "pick" && max !== undefined ? ` (0 bis ${max})` : "";
  return (
    <label style={row} title={`${spec.key}${hint}`}>
      <span>{spec.label}</span>
      {spec.kind === "bool" ? (
        <input type="checkbox" checked={value !== 0} onChange={(e) => onCommit(spec, e.target.checked ? 1 : 0)} style={{ justifySelf: "start" }} />
      ) : spec.kind === "select" ? (
        <select value={value} onChange={(e) => onCommit(spec, Number(e.target.value))} style={input}>
          {(spec.options ?? []).some((o) => o.value === value) ? null : <option value={value}>{`Wert ${value}`}</option>}
          {(spec.options ?? []).map((o) => (
            <option key={o.value} value={o.value}>
              {o.label}
            </option>
          ))}
        </select>
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
