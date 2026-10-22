import type { Diagnostic } from "@codemirror/lint";
import { ask } from "@tauri-apps/plugin-dialog";
import { memo, useCallback, useEffect, useRef, useState } from "react";
import type { CSSProperties } from "react";
import type { ProjectInfo } from "../../api";
import { useStableCallback } from "../../app/useStable";
import { scriptPath } from "../problems/problems";
import type { ProblemsStore } from "../problems/problems";
import { TERMS, id4 } from "../terms";
import { AdoptBox } from "./AdoptBox";
import { scriptUri } from "./documents";
import type { ScriptDocument } from "./documents";
import { documentSymbols } from "./lspExtras";
import { ScriptEditor } from "./ScriptEditor";
import type { ScriptsApi } from "./useScripts";
import { useLsp } from "./useLsp";
import type { LspStatus } from "./useLsp";

/** What the script panel needs from App. */
export interface ScriptPanelContext {
  /** The open project; another object means another one was opened. */
  project: ProjectInfo | null;
  scripts: ScriptsApi;
  /** Where the live diagnostics of the shown document go. */
  problems: ProblemsStore;
  onError: (e: unknown) => void;
}

const bar: CSSProperties = { display: "flex", alignItems: "center", gap: 8, padding: "4px 8px", borderBottom: "1px solid #e5e7eb", flexWrap: "wrap" };
const note: CSSProperties = { margin: 0, padding: "8px 12px", color: "#6b7280" };
const tab: CSSProperties = { all: "unset", cursor: "pointer", padding: "2px 8px", borderRadius: 4, border: "1px solid #d1d5db", whiteSpace: "nowrap" };
const activeTab: CSSProperties = { ...tab, background: "#eff6ff", borderColor: "#2563eb", fontWeight: 700 };

/** "Scriptdatei 0042" / "Level-Script 0042": how documents are named in questions and lists. */
export const docName = (doc: Pick<ScriptDocument, "file" | "kind">) => `${doc.kind === "script" ? TERMS.scriptFile.label : TERMS.levelScript.label} ${id4(doc.file)}`;
/** The header title: a level script says it is shown as JSON. */
export const docTitle = (doc: Pick<ScriptDocument, "file" | "kind">) => `${docName(doc)}${doc.kind === "levelScript" ? " (JSON)" : ""}`;

const isDirty = (doc: ScriptDocument) => doc.text !== doc.savedText;

/** The problems of a shown document's diagnostics, in the problems store's words (1-based line, UTF-16 column like the server). */
function liveProblems(doc: ScriptDocument, diagnostics: Diagnostic[]) {
  const text = doc.state?.doc;
  if (!text) return [];
  const file = scriptPath(doc.file, doc.kind);
  return diagnostics.flatMap((d) => {
    // Information and hints are not problems.
    if (d.severity !== "error" && d.severity !== "warning") return [];
    const line = text.lineAt(Math.min(d.from, text.length));
    return [{ severity: d.severity, file, line: line.number, col: Math.min(d.from, text.length) - line.from + 1, message: d.message }];
  });
}

/** The outline menu: the symbols of the shown document from the language server, loaded when the menu opens. */
function Outline({ lsp, uri, onJump }: { lsp: LspStatus; uri: string; onJump: (line: number) => void }) {
  const [menu, setMenu] = useState<{ items: { name: string; detail?: string; line: number }[] } | { error: string } | null>(null);
  const [seenUri, setSeenUri] = useState(uri);
  if (seenUri !== uri) {
    setSeenUri(uri);
    setMenu(null);
  }
  async function toggle() {
    if (menu || !lsp.client) return setMenu(null);
    setMenu({ items: [] });
    try {
      setMenu({ items: await documentSymbols(lsp.client, uri) });
    } catch {
      setMenu({ error: "Gliederung nicht verfügbar" });
    }
  }
  return (
    <span style={{ position: "relative" }}>
      <button onClick={() => void toggle()} disabled={!lsp.client} title={lsp.client ? "Springt zu einem Script der Datei" : "Braucht rotom-lsp"}>
        Gliederung
      </button>
      {menu && (
        <ul role="menu" style={{ position: "absolute", zIndex: 10, margin: 0, padding: 4, listStyle: "none", background: "#fff", border: "1px solid #d1d5db", borderRadius: 4, minWidth: 180, maxHeight: 240, overflowY: "auto" }}>
          {"error" in menu ? (
            <li style={{ padding: "2px 8px", color: "#6b7280" }}>{menu.error}</li>
          ) : (
            menu.items.map((s) => (
              <li key={`${s.line}:${s.name}`}>
                <button
                  role="menuitem"
                  onClick={() => {
                    setMenu(null);
                    onJump(s.line);
                  }}
                  style={{ all: "unset", cursor: "pointer", display: "block", padding: "2px 8px", width: "100%", boxSizing: "border-box" }}
                >
                  {s.name}
                  {s.detail ? ` ${s.detail}` : ""} · Zeile {s.line}
                </button>
              </li>
            ))
          )}
        </ul>
      )}
    </span>
  );
}

function lspText(lsp: LspStatus): string | null {
  switch (lsp.state) {
    case "starting":
      return "Sprachdienst startet …";
    case "running":
      return "rotom-lsp läuft";
    default:
      return lsp.note;
  }
}

/** The Script panel (spec §8.2): the adoption box until rotom has taken the scripts over, then the documents with their editor. */
export const ScriptPanel = memo(function ScriptPanel({ context }: { context: ScriptPanelContext }) {
  const { project, scripts, problems, onError } = context;
  const { status, documents, active, open, save, close, add, adopt, onChange } = scripts;
  const adopted = status?.adopted === true;
  const lsp = useLsp(project, adopted);
  // The box stays after the adoption until its report was read.
  const [reading, setReading] = useState(false);
  const [seenProject, setSeenProject] = useState(project);
  if (seenProject !== project) {
    setSeenProject(project);
    setReading(false);
  }

  const doc = active === null ? undefined : documents.get(active);
  const shown = useRef<ScriptDocument | undefined>(undefined);
  shown.current = doc;

  const onDiagnostics = useCallback(
    (diagnostics: Diagnostic[]) => problems.replaceLive(shown.current ? liveProblems(shown.current, diagnostics) : []),
    [problems],
  );
  // No document shown: no live diagnostics.
  useEffect(() => {
    if (!doc) problems.replaceLive([]);
  }, [doc, problems]);

  const onEditorChange = useStableCallback(() => {
    if (shown.current) onChange(shown.current.file);
  });
  const onSave = useStableCallback(() => {
    if (shown.current) save(shown.current.file).catch(onError);
  });

  async function closeDocument(target: ScriptDocument) {
    if (isDirty(target) && !(await ask(`Änderungen an ${docName(target)} verwerfen?`, { kind: "warning" }))) return;
    close(target.file);
  }

  if (!project || !status) return <p style={note}>{project ? "Scripts werden geprüft …" : "Kein Projekt geöffnet."}</p>;
  if (!adopted || reading) {
    return (
      <div style={{ height: "100%", overflowY: "auto" }}>
        <AdoptBox
          fileCount={status.file_count}
          adopt={async (onProgress) => {
            setReading(true);
            try {
              return await adopt(onProgress);
            } catch (e) {
              // Nothing was adopted: back to the box in its start state.
              setReading(false);
              throw e;
            }
          }}
          onClose={() => setReading(false)}
        />
      </div>
    );
  }

  const open_ = documents.list();
  const uri = doc ? scriptUri(project.root, doc.file, doc.kind) : "";
  const lspNote = lspText(lsp);
  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%", minWidth: 0, fontFamily: "system-ui, sans-serif", fontSize: 13 }}>
      <div style={bar}>
        {open_.length === 0 && <span style={{ color: "#6b7280" }}>Kein Script geöffnet</span>}
        {open_.map((d) => (
          <span key={d.file} style={{ display: "inline-flex", gap: 2 }}>
            <button data-script={d.file} onClick={() => void open(d.file)} aria-current={d.file === active ? "true" : undefined} style={d.file === active ? activeTab : tab}>
              {isDirty(d) ? "● " : ""}
              {docName(d)}
            </button>
            <button aria-label={`${docName(d)} schließen`} onClick={() => void closeDocument(d)} style={{ ...tab, padding: "2px 5px" }}>
              ×
            </button>
          </span>
        ))}
      </div>
      <div style={bar}>
        {doc ? (
          <>
            <strong>{docTitle(doc)}</strong>
            <Outline lsp={lsp} uri={uri} onJump={(line) => void open(doc.file, line)} />
            <button onClick={() => onSave()}>Speichern</button>
            <button onClick={() => void add(doc.file)} disabled={doc.kind !== "script"} title="Hängt ein leeres Script an die Datei an">
              Script anfügen
            </button>
          </>
        ) : (
          <span style={{ color: "#6b7280" }}>Eine {TERMS.scriptFile.label} öffnet sich über den Header, ein Event oder ein Problem.</span>
        )}
        <span style={{ marginLeft: "auto", color: lsp.state === "running" ? "#6b7280" : "#b45309" }} data-lsp={lsp.state}>
          {lspNote}
        </span>
        {lsp.state === "stopped" && <button onClick={lsp.restart}>Neu starten</button>}
      </div>
      <div style={{ flex: 1, minHeight: 0 }}>
        {doc && (
          <ScriptEditor
            doc={doc}
            language={doc.kind === "script" ? "rotom" : "json"}
            lsp={lsp.client}
            uri={uri}
            onChange={onEditorChange}
            onSave={onSave}
            onDiagnostics={onDiagnostics}
            theme="light"
          />
        )}
      </div>
    </div>
  );
});

export default ScriptPanel;
