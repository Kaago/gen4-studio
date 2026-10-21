import { useCallback, useEffect, useRef, useState } from "react";
import { ask, open, save as saveDialog } from "@tauri-apps/plugin-dialog";
import { buildRom, dirty, getMatrix, listHeaders, openProject, redo, runRom, save, undo } from "./api";
import type { HeaderSummary, MatrixDto, ProjectInfo } from "./api";
import { StatusBar, Toolbar } from "./Toolbar";
import type { Message } from "./Toolbar";
import MatrixScene from "./world/MatrixScene";

const EMULATOR_KEY = "g4.emulatorPath";

function storedEmulator(): string | null {
  try {
    return localStorage.getItem(EMULATOR_KEY);
  } catch {
    return null;
  }
}

function storeEmulator(path: string) {
  try {
    localStorage.setItem(EMULATOR_KEY, path);
  } catch {
    /* the choice is simply asked again next time */
  }
}

export default function App() {
  const [info, setInfo] = useState<ProjectInfo | null>(null);
  const [headers, setHeaders] = useState<HeaderSummary[]>([]);
  const [selected, setSelected] = useState<number | null>(null);
  const [matrix, setMatrix] = useState<MatrixDto | null>(null);
  const [showPermissions, setShowPermissions] = useState(true);
  const [message, setMessage] = useState<Message | null>(null);
  const [dirtyCount, setDirtyCount] = useState(0);
  const [reloadToken, setReloadToken] = useState(0);
  const [busy, setBusy] = useState(false);
  const [lastRom, setLastRom] = useState<string | null>(null);

  const fail = useCallback((e: unknown) => setMessage({ kind: "error", text: String(e) }), []);
  const refreshDirty = useCallback(async () => {
    try {
      setDirtyCount((await dirty()).length);
    } catch (e) {
      fail(e);
    }
  }, [fail]);

  async function onOpen() {
    try {
      if (dirtyCount > 0 && !(await ask("Ungespeicherte Änderungen verwerfen?", { kind: "warning" }))) return;
      const path = await open({ directory: true });
      if (typeof path !== "string") return;
      const opened = await openProject(path);
      setHeaders(await listHeaders());
      setInfo(opened);
      setSelected(null);
      setMatrix(null);
      setLastRom(null);
      setMessage(null);
      await refreshDirty();
    } catch (e) {
      fail(e);
    }
  }

  async function onSelect(h: HeaderSummary) {
    setSelected(h.id);
    try {
      setMatrix(await getMatrix(h.matrix_id));
      setMessage(null);
    } catch (e) {
      fail(e);
    }
  }

  async function step(action: typeof undo) {
    if (!info || busy) return;
    try {
      if (await action()) setReloadToken((t) => t + 1);
    } catch (e) {
      fail(e);
    }
    await refreshDirty();
  }

  async function onSave() {
    if (!info || busy) return;
    try {
      const report = await save();
      setMessage({ kind: "info", text: `${report.written.length} Datei(en) gespeichert` });
    } catch (e) {
      fail(e);
    }
    await refreshDirty();
  }

  async function onBuild() {
    try {
      const out = await saveDialog({ defaultPath: "out.nds", filters: [{ name: "ROM", extensions: ["nds"] }] });
      if (!out) return;
      setBusy(true);
      setMessage({ kind: "info", text: "Baue ROM ..." });
      const report = await buildRom(out);
      setLastRom(report.rom);
      setMessage({ kind: "info", text: `ROM gebaut: ${report.rom} (${report.saved} Datei(en) vorher gespeichert)` });
    } catch (e) {
      fail(e);
    } finally {
      setBusy(false);
    }
    await refreshDirty();
  }

  async function pickEmulator(): Promise<string | null> {
    const chosen = await open({ directory: false, multiple: false, title: "Emulator wählen (z. B. melonDS)" });
    if (typeof chosen !== "string") return null;
    storeEmulator(chosen);
    return chosen;
  }

  async function onPickEmulator() {
    try {
      const chosen = await pickEmulator();
      if (chosen) setMessage({ kind: "info", text: `Emulator: ${chosen}` });
    } catch (e) {
      fail(e);
    }
  }

  async function onRun() {
    if (!lastRom) return;
    try {
      const emulator = storedEmulator() ?? (await pickEmulator());
      if (!emulator) return;
      await runRom(emulator, lastRom);
      setMessage({ kind: "info", text: `Emulator gestartet: ${lastRom}` });
    } catch (e) {
      fail(e);
    }
  }

  // Shortcuts always call the handlers of the latest render.
  const latest = useRef({ onSave, step });
  latest.current = { onSave, step };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!(e.metaKey || e.ctrlKey) || e.altKey) return;
      const key = e.key.toLowerCase();
      if (key === "z") {
        e.preventDefault();
        void latest.current.step(e.shiftKey ? redo : undo);
      } else if (key === "s") {
        e.preventDefault();
        void latest.current.onSave();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", display: "flex", flexDirection: "column", height: "100vh" }}>
      <Toolbar
        projectOpen={info !== null}
        busy={busy}
        dirtyCount={dirtyCount}
        canRun={lastRom !== null}
        onOpen={onOpen}
        onUndo={() => void step(undo)}
        onRedo={() => void step(redo)}
        onSave={() => void onSave()}
        onBuild={() => void onBuild()}
        onRun={() => void onRun()}
        onPickEmulator={() => void onPickEmulator()}
      />
      <div style={{ flex: 1, minHeight: 0, display: "grid", gridTemplateColumns: "280px 1fr" }}>
        <aside style={{ padding: 16, overflowY: "auto", borderRight: "1px solid #ccc" }}>
          {info && (
            <p>
              {info.game_code} ({info.family}, {info.language}), {info.header_count} Header
            </p>
          )}
          <ul style={{ fontFamily: "ui-monospace, monospace", listStyle: "none", padding: 0 }}>
            {headers.map((h) => (
              <li key={h.id}>
                <button
                  onClick={() => void onSelect(h)}
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
              <MatrixScene
                matrix={matrix}
                showPermissions={showPermissions}
                paintEnabled={!busy}
                reloadToken={reloadToken}
                onEdited={() => void refreshDirty()}
                onError={fail}
              />
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
            <p style={{ padding: 16 }}>Header wählen, um die Matrix anzuzeigen. Klick setzt Kollision, Shift+Klick löscht sie.</p>
          )}
        </section>
      </div>
      <StatusBar message={message} onDismiss={() => setMessage(null)} />
    </main>
  );
}
