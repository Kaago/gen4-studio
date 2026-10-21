import { useMemo, useState } from "react";
import type { CSSProperties } from "react";
import type { HeaderSummary, ProjectInfo } from "../api";
import { filterHeaders } from "./headers";

interface Props {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  selected: number | null;
  onSelect: (h: HeaderSummary) => void;
}

/** id, location name, internal name, matrix. */
const COLUMNS = "48px minmax(0, 1fr) minmax(0, 96px) 48px";

const cell: CSSProperties = { whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" };

export function HeaderList({ info, headers, selected, onSelect }: Props) {
  const [query, setQuery] = useState("");
  const shown = useMemo(() => filterHeaders(headers, query), [headers, query]);
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", boxSizing: "border-box", fontFamily: "system-ui, sans-serif", fontSize: 13 }}>
      <div style={{ padding: "12px 16px 8px" }}>
        {info && (
          <p style={{ margin: "0 0 8px", ...cell }}>
            {info.game_code} ({info.family}, {info.language}), {info.header_count} Header
          </p>
        )}
        <input
          type="search"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          placeholder="Suchen nach Ort oder Kürzel"
          aria-label="Header suchen"
          style={{ width: "100%", boxSizing: "border-box", padding: "4px 8px" }}
        />
      </div>
      <ul style={{ flex: 1, minHeight: 0, overflowY: "auto", listStyle: "none", margin: 0, padding: "0 16px 16px" }}>
        {shown.map((h) => (
          <li key={h.id}>
            <button
              onClick={() => onSelect(h)}
              title={`${String(h.id).padStart(4, "0")} ${h.location_name} ${h.internal_name} Matrix ${h.matrix_id}`}
              style={{
                all: "unset",
                boxSizing: "border-box",
                display: "grid",
                gridTemplateColumns: COLUMNS,
                columnGap: 8,
                width: "100%",
                cursor: "pointer",
                lineHeight: 1.6,
                fontWeight: h.id === selected ? 700 : 400,
              }}
            >
              <span style={{ ...cell, fontFamily: "ui-monospace, monospace" }}>{String(h.id).padStart(4, "0")}</span>
              <span style={cell}>{h.location_name}</span>
              <span style={{ ...cell, color: "#6b7280" }}>{h.internal_name}</span>
              <span style={{ ...cell, textAlign: "right" }}>M{h.matrix_id}</span>
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
