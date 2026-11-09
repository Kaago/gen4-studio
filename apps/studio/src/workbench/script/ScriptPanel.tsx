import type { Diagnostic } from "@codemirror/lint";
import { ask } from "../../platform/dialogs";
import { memo, useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { ProjectInfo } from "../../api";
import { toolStatus } from "../../app/toolStatus";
import type { LspToolState } from "../../app/toolStatus";
import { useStableCallback } from "../../app/useStable";
import { HeadActions } from "../../original/HeadActions";
import { useReset } from "../../original/OriginalContext";
import { RESET_LABEL, resetEntry } from "../../original/resetEntry";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { Badge, Button, EmptyState, Menu, PanelHead, Tabs } from "../../ui/primitives";
import type { BadgeKind, MenuEntry, TabItem } from "../../ui/primitives";
import { effectiveTheme, onThemeChange } from "../../ui/tokens/theme";
import type { EditBus } from "../../bus/editBus";
import { scriptPath } from "../problems/problems";
import type { ProblemsStore } from "../problems/problems";
import { id4, termFor, termLabel } from "../terms";
import { AdoptBox } from "./AdoptBox";
import { scriptUri } from "./documents";
import type { ScriptDocument } from "./documents";
import { documentSymbols } from "./lspExtras";
import { ScriptEditor } from "./ScriptEditor";
import { scriptDirtyStore } from "./scriptDirtyStore";
import type { ScriptsApi } from "./useScripts";
import { useLsp } from "./useLsp";
import type { LspState, LspStatus } from "./useLsp";
import styles from "./ScriptPanel.module.css";

/** What the script panel needs from App. */
export interface ScriptPanelContext {
  /** The open project; another object means another one was opened. */
  project: ProjectInfo | null;
  scripts: ScriptsApi;
  /** Where the live diagnostics of the shown document go. */
  problems: ProblemsStore;
  /** Where saved text archives announce themselves to the language server. A stable object. */
  bus: EditBus;
  /** Tüfteln shows the path of the shown file under its title. */
  depth: Depth;
  onError: (e: unknown) => void;
}

/** "Scriptdatei 0042" / "Level-Script 0042": how documents are named in questions and lists. */
export const docName = (doc: Pick<ScriptDocument, "file" | "kind">) => `${doc.kind === "script" ? termLabel("scriptFile") : termLabel("levelScript")} ${id4(doc.file)}`;
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

const note = (label: string): MenuEntry => ({ label, disabled: true, onSelect: () => {} });
const LOADING = [note("Gliederung wird geladen …")];

/** The outline menu: the symbols of the shown document from the language server, loaded each time the menu opens. */
function Outline({ lsp, uri, onJump }: { lsp: LspStatus; uri: string; onJump: (line: number) => void }) {
  const [entries, setEntries] = useState<readonly MenuEntry[]>(LOADING);
  // Only the answer to the latest opening counts.
  const request = useRef(0);
  const client = lsp.client;
  if (!client) {
    return (
      <Button kind="quiet" size="s" disabled title="Braucht rotom-lsp">
        Gliederung
      </Button>
    );
  }
  function onOpenChange(open: boolean) {
    if (!open || !client) return;
    const mine = ++request.current;
    setEntries(LOADING);
    documentSymbols(client, uri).then(
      (symbols) => {
        if (mine !== request.current) return;
        setEntries(
          symbols.length === 0
            ? [note("Keine Scripts in der Datei")]
            : symbols.map((s) => ({ label: `${s.name}${s.detail ? ` ${s.detail}` : ""}, Z. ${s.line}`, onSelect: () => onJump(s.line) })),
        );
      },
      () => mine === request.current && setEntries([note("Gliederung nicht verfügbar")]),
    );
  }
  return <Menu trigger="Gliederung" items={entries} onOpenChange={onOpenChange} />;
}

const LSP_BADGE: Record<LspState, BadgeKind> = { starting: "neutral", running: "success", missing: "warning", stopped: "error" };
/** The badge's short words; the whole note is its tooltip (a long note would wrap the panel head in a narrow hand). */
const LSP_SHORT: Record<LspState, string> = { starting: "rotom-lsp startet", running: "rotom-lsp läuft", missing: "rotom-lsp fehlt", stopped: "rotom-lsp gestoppt" };
/** The panel's states in the words of Technik's tool status. */
const LSP_TOOL: Record<LspState, LspToolState> = { starting: "startet", running: "bereit", missing: "fehlt", stopped: "fehler" };

function lspText(lsp: LspStatus): string {
  switch (lsp.state) {
    case "starting":
      return "Sprachdienst startet …";
    case "running":
      return "rotom-lsp läuft";
    default:
      return lsp.note ?? "rotom-lsp läuft nicht";
  }
}

/** Why Technik › Werkzeuge shows the server off while the project's scripts are not adopted. */
const NOT_ADOPTED = "Läuft, sobald die Scripts des Projekts übernommen sind.";

/** The Script panel (spec §7.3): the adoption box until rotom has taken the scripts over, then the documents as tabs with their editor. */
export const ScriptPanel = memo(function ScriptPanel({ context }: { context: ScriptPanelContext }) {
  const { project, scripts, problems, bus, depth, onError } = context;
  const { status, documents, active, open, save, close, add, adopt, onChange } = scripts;
  const adopted = status?.adopted === true;
  const lsp = useLsp(project, adopted, bus);
  const reset = useReset();
  // Technik › Werkzeuge shows the server's state and restarts it through the panel. Without an adopted project it does not run.
  const serving = project !== null && adopted;
  useEffect(() => {
    toolStatus.set({ lsp: serving ? { state: LSP_TOOL[lsp.state], note: lsp.note ?? undefined, restart: lsp.restart } : { state: "aus", note: NOT_ADOPTED } });
  }, [serving, lsp.state, lsp.note, lsp.restart]);
  // The panel's server ends with the panel.
  useEffect(() => () => toolStatus.set({ lsp: { state: "aus" } }), []);
  const theme = useSyncExternalStore(onThemeChange, effectiveTheme);
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

  // The dock tab "Script" shows a dot while a document is unsaved; this panel renders again with every change of the dirty set.
  useEffect(() => scriptDirtyStore.set(documents.dirty().length));
  useEffect(() => () => scriptDirtyStore.set(0), []);

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

  async function closeDocument(target: ScriptDocument | undefined) {
    if (!target) return;
    if (isDirty(target) && !(await ask(`Änderungen an ${docName(target)} verwerfen?`, { kind: "warning" }))) return;
    close(target.file);
  }

  if (!project || !status) return <p className={styles.note}>{project ? "Scripts werden geprüft …" : "Kein Projekt geöffnet."}</p>;
  if (!adopted || reading) {
    return (
      <div className={styles.scroll}>
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

  const openDocs = documents.list();
  const uri = doc ? scriptUri(project.root, doc.file, doc.kind) : "";
  const tabs: TabItem[] = openDocs.map((d) => ({
    id: String(d.file),
    label: docName(d),
    unsaved: isDirty(d),
    closeLabel: `${docName(d)} schließen`,
    dataAttrs: { "data-script": d.file },
  }));
  const lspNote = lspText(lsp);
  return (
    <section className={styles.panel} aria-label="Script">
      <PanelHead
        // Without a document the head names nothing: the dock tab above it says "Script".
        title={doc ? docTitle(doc) : undefined}
        subtitle={doc && depthAtLeast(depth, "tuefteln") ? scriptPath(doc.file, doc.kind) : undefined}
        actions={
          <>
            {doc && (
              <>
                <Outline lsp={lsp} uri={uri} onJump={(line) => void open(doc.file, line)} />
                <Button kind="secondary" size="s" icon="save" onClick={() => onSave()}>
                  Speichern
                </Button>
                <Button kind="quiet" size="s" icon="plus" onClick={() => void add(doc.file)} disabled={doc.kind !== "script"} title="Hängt ein leeres Script an die Datei an">
                  Script anfügen
                </Button>
              </>
            )}
            {lsp.state === "stopped" && (
              <Button kind="quiet" size="s" onClick={lsp.restart}>
                Neu starten
              </Button>
            )}
            <HeadActions
              push={false}
              items={[
                // The shown file as in the base ROM; the dialog warns when the editor holds unsaved text of it (that text is lost).
                doc
                  ? resetEntry({ kind: "script", file: doc.file }, reset.status, (target) => reset.open(target, { scriptDirty: isDirty(doc) }))
                  : { label: RESET_LABEL, icon: "undo", disabled: true, detail: "Kein Script geöffnet", onSelect: () => {} },
              ]}
            />
          </>
        }
        status={
          <span className={styles.lsp} data-lsp={lsp.state}>
            <Badge kind={LSP_BADGE[lsp.state]} title={lspNote}>
              {LSP_SHORT[lsp.state]}
            </Badge>
          </span>
        }
      />
      {openDocs.length === 0 ? (
        <EmptyState what="Kein Script geöffnet." why={`Eine ${termLabel("scriptFile")} öffnet sich über den ${termFor("header", depth)}, ein Event oder ein Problem.`} />
      ) : (
        <div className={styles.tabs}>
          <Tabs
            items={tabs}
            value={active === null ? "" : String(active)}
            onChange={(id) => void open(Number(id))}
            onClose={(id) => void closeDocument(documents.get(Number(id)))}
            ariaLabel="Offene Scripts"
          >
            {doc && (
              <ScriptEditor
                doc={doc}
                language={doc.kind === "script" ? "rotom" : "json"}
                lsp={lsp.client}
                uri={uri}
                onChange={onEditorChange}
                onDiagnostics={onDiagnostics}
                theme={theme}
              />
            )}
          </Tabs>
        </div>
      )}
    </section>
  );
});

export default ScriptPanel;
