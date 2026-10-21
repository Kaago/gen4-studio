import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { listHeaders, openProject } from "./api";
import type { HeaderSummary, ProjectInfo } from "./api";

export default function App() {
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [headers, setHeaders] = useState<HeaderSummary[]>([]);
  const [error, setError] = useState<string | null>(null);

  async function onOpen() {
    setError(null);
    try {
      const path = await open({ directory: true });
      if (typeof path !== "string") return;
      const opened = await openProject(path);
      setHeaders(await listHeaders());
      setInfo(opened);
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", padding: 16 }}>
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
            {String(h.id).padStart(4, "0")} {h.internal_name} matrix {h.matrix_id}
          </li>
        ))}
      </ul>
    </main>
  );
}
