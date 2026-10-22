import { useMemo, useState } from "react";
import type { HeaderSummary } from "../../api";
import { filterHeaders } from "../headers";

/** The most matches the list shows; a longer list asks for a narrower search. */
const SHOWN = 40;

/** Searchable list of headers; picking one calls `onPick` with its id. */
export function HeaderPicker({ headers, current, disabled, onPick }: { headers: HeaderSummary[]; current: number; disabled?: boolean; onPick: (id: number) => void }) {
  const [query, setQuery] = useState("");
  const matches = useMemo(() => filterHeaders(headers, query), [headers, query]);
  return (
    <div>
      <input
        type="search"
        value={query}
        onChange={(e) => setQuery(e.target.value)}
        placeholder="Zielheader suchen (Ort oder Kürzel)"
        aria-label="Zielheader suchen"
        disabled={disabled}
        style={{ width: "100%", boxSizing: "border-box", padding: "2px 6px" }}
      />
      <ul style={{ listStyle: "none", margin: "4px 0 0", padding: 0, maxHeight: 140, overflowY: "auto", border: "1px solid #e5e7eb" }}>
        {matches.slice(0, SHOWN).map((h) => (
          <li key={h.id}>
            <button
              type="button"
              disabled={disabled}
              onClick={() => onPick(h.id)}
              style={{ all: "unset", boxSizing: "border-box", display: "block", width: "100%", padding: "1px 6px", cursor: disabled ? "default" : "pointer", fontWeight: h.id === current ? 700 : 400, opacity: disabled ? 0.6 : 1 }}
            >
              <span style={{ fontFamily: "ui-monospace, monospace" }}>{String(h.id).padStart(4, "0")}</span> {h.location_name || h.internal_name}
              {h.location_name ? <span style={{ color: "#6b7280" }}> ({h.internal_name})</span> : null}
            </button>
          </li>
        ))}
        {matches.length === 0 && <li style={{ padding: "1px 6px", color: "#6b7280" }}>Kein Header gefunden</li>}
        {matches.length > SHOWN && <li style={{ padding: "1px 6px", color: "#6b7280" }}>{matches.length - SHOWN} weitere, Suche eingrenzen</li>}
      </ul>
    </div>
  );
}
