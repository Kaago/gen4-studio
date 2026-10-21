import type { HeaderSummary, ProjectInfo } from "../api";

interface Props {
  info: ProjectInfo | null;
  headers: HeaderSummary[];
  selected: number | null;
  onSelect: (h: HeaderSummary) => void;
}

export function HeaderList({ info, headers, selected, onSelect }: Props) {
  return (
    <div style={{ padding: 16, height: "100%", boxSizing: "border-box", overflowY: "auto", fontFamily: "system-ui, sans-serif" }}>
      {info && (
        <p>
          {info.game_code} ({info.family}, {info.language}), {info.header_count} Header
        </p>
      )}
      <ul style={{ fontFamily: "ui-monospace, monospace", listStyle: "none", padding: 0 }}>
        {headers.map((h) => (
          <li key={h.id}>
            <button
              onClick={() => onSelect(h)}
              style={{
                all: "unset",
                cursor: "pointer",
                width: "100%",
                fontWeight: h.id === selected ? 700 : 400,
              }}
            >
              {String(h.id).padStart(4, "0")} {h.internal_name} matrix {h.matrix_id}
            </button>
          </li>
        ))}
      </ul>
    </div>
  );
}
