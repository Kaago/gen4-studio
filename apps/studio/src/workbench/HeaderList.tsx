import { memo, useEffect, useMemo, useRef, useState } from "react";
import type { CSSProperties } from "react";
import type { HeaderSummary, ProjectInfo } from "../api";
import { filterHeaders, showsMatrix } from "./headers";

interface Props {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  selected: number | null;
  /** The matrix the scene shows: headers using it are marked. */
  shownMatrix: number | null;
  /** Bumped by every link to a header: the list scrolls to the selection again, also when it was selected already. */
  revealToken: number;
  onSelect: (h: HeaderSummary) => void;
}

/** id, location name, internal name, matrix. */
const COLUMNS = "48px minmax(0, 1fr) minmax(0, 96px) 48px";

const cell: CSSProperties = { whiteSpace: "nowrap", overflow: "hidden", textOverflow: "ellipsis" };

/** One row of the list. Memoised: a selection or search change renders the rows whose props changed, not all of them. */
const HeaderRow = memo(function HeaderRow({ header: h, selected, marked, onSelect }: { header: HeaderSummary; selected: boolean; marked: boolean; onSelect: (h: HeaderSummary) => void }) {
  return (
    <li>
      <button
        data-header-id={h.id}
        onClick={() => onSelect(h)}
        aria-current={selected ? "true" : undefined}
        title={`${String(h.id).padStart(4, "0")} ${h.location_name} ${h.internal_name} Matrix ${h.matrix_id}${marked ? " (Matrix wird angezeigt)" : ""}`}
        style={{
          all: "unset",
          boxSizing: "border-box",
          display: "grid",
          gridTemplateColumns: COLUMNS,
          columnGap: 8,
          width: "100%",
          cursor: "pointer",
          lineHeight: 1.6,
          fontWeight: selected ? 700 : 400,
          background: marked ? "#eff6ff" : "transparent",
          boxShadow: marked ? "inset 3px 0 0 #2563eb" : "none",
        }}
      >
        <span style={{ ...cell, fontFamily: "ui-monospace, monospace" }}>{String(h.id).padStart(4, "0")}</span>
        <span style={cell}>{h.location_name}</span>
        <span style={{ ...cell, color: "#6b7280" }}>{h.internal_name}</span>
        <span style={{ ...cell, textAlign: "right" }}>M{h.matrix_id}</span>
      </button>
    </li>
  );
});

/**
 * Memoised, and its rows too: it renders when the list, the selection, the matrix shown, the
 * reveal token or the callback change, not when the scene is hovered or a message arrives.
 * `onSelect` must keep its identity for that.
 */
export const HeaderList = memo(function HeaderList({ info, headers, selected, shownMatrix, revealToken, onSelect }: Props) {
  const [query, setQuery] = useState("");
  const shown = useMemo(() => filterHeaders(headers, query), [headers, query]);
  const list = useRef<HTMLUListElement>(null);
  // The selection and token last scrolled to, so a search that hides the selection later does not pull it back.
  const revealed = useRef<string | null>(null);
  // A newly selected header (also one chosen by a link) is scrolled into view; a search that hides
  // it is cleared first.
  useEffect(() => {
    if (selected === null) {
      revealed.current = null;
      return;
    }
    const key = `${selected}:${revealToken}`;
    if (revealed.current === key) return;
    // Looked up by id when needed, instead of a ref callback per row.
    const row = list.current?.querySelector(`[data-header-id="${selected}"]`);
    if (row) {
      revealed.current = key;
      row.scrollIntoView({ block: "nearest" });
    } else if (query !== "") {
      setQuery("");
    }
  });
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
      <ul ref={list} style={{ flex: 1, minHeight: 0, overflowY: "auto", listStyle: "none", margin: 0, padding: "0 16px 16px" }}>
        {shown.map((h) => (
          <HeaderRow key={h.id} header={h} selected={h.id === selected} marked={showsMatrix(h, shownMatrix)} onSelect={onSelect} />
        ))}
      </ul>
    </div>
  );
});
