import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getMatrix, listHeaders, openProject } from "./api";
import type { HeaderSummary, MatrixDto, ProjectInfo } from "./api";
import MatrixScene from "./world/MatrixScene";

export default function App() {
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [headers, setHeaders] = useState<HeaderSummary[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [matrix, setMatrix] = useState<MatrixDto | null>(null);
  const [showPermissions, setShowPermissions] = useState(true);
  const [error, setError] = useState<string | null>(null);

  async function onOpen() {
    setError(null);
    try {
      const path = await open({ directory: true });
      if (typeof path !== "string") return;
      const opened = await openProject(path);
      setHeaders(await listHeaders());
      setInfo(opened);
      setSelected(null);
      setMatrix(null);
    } catch (e) {
      setError(String(e));
    }
  }

  async function onSelect(h: HeaderSummary) {
    setError(null);
    setSelected(h.id);
    try {
      setMatrix(await getMatrix(h.matrix_id));
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main
      style={{
        fontFamily: "system-ui, sans-serif",
        display: "grid",
        gridTemplateColumns: "280px 1fr",
        height: "100vh",
      }}
    >
      <aside style={{ padding: 16, overflowY: "auto", borderRight: "1px solid #ccc" }}>
        <button onClick={onOpen}>Projekt öffnen</button>
        {error && <p style={{ color: "#d64545" }}>{error}</p>}
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
      </aside>
      <section style={{ position: "relative", minWidth: 0 }}>
        {matrix ? (
          <>
            <MatrixScene matrix={matrix} showPermissions={showPermissions} />
            <label
              style={{
                position: "absolute",
                top: 8,
                left: 8,
                color: "#e5e7eb",
                background: "rgba(0,0,0,0.5)",
                padding: "4px 8px",
                borderRadius: 4,
              }}
            >
              <input
                type="checkbox"
                checked={showPermissions}
                onChange={(e) => setShowPermissions(e.target.checked)}
              />{" "}
              Permissions anzeigen
            </label>
          </>
        ) : (
          <p style={{ padding: 16 }}>Header wählen, um die Matrix anzuzeigen.</p>
        )}
      </section>
    </main>
  );
}
