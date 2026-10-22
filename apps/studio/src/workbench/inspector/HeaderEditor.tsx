import { Fragment, useEffect, useRef, useState } from "react";
import { getHeader, headerFamily, newScriptFile, setHeader } from "../../api";
import type { HeaderSummary, LevelScriptDoc, MapHeader, MemberCounts, ProjectInfo, SaveScriptDto } from "../../api";
import { headerLabel } from "../headers";
import type { NavTarget } from "../navigation";
import { TERMS } from "../terms";
import type { WizardRequest } from "../WorkbenchContext";
import { ExitsSection } from "./ExitsSection";
import { Field } from "./Field";
import { fieldValue, headerFieldSpecs, pickMax } from "./fields";
import type { FieldSpec } from "./fields";
import { HeaderSession } from "./headerCommit";
import { LevelScriptSection } from "./LevelScriptSection";
import { ADOPT_HINT } from "./scriptNav";

/** What the header editor needs from App besides the header id. */
export interface HeaderEditorContext {
  headers: HeaderSummary[];
  counts: MemberCounts;
  /** Changes whenever the header table was changed behind the editor's back (undo, redo, reload). */
  revision: number;
  /** Called after a field was written, so the list, the scene and the dirty count follow. */
  onChanged: () => void | Promise<void>;
  /** Follows a link (area data, matrix panel). */
  goTo: (target: NavTarget) => void;
  /** Changes whenever events were changed behind the exits section's back (a drag, add, delete, undo, redo). */
  eventRevision: number;
  /** Opens a wizard ("Raum hier anbinden…"). */
  onOpenWizard: (request: WizardRequest) => void;
  /** Adds a warp to the selected header's event file. */
  onAddWarp: () => void;
  /** The open project (the level-script form starts over when another one is opened). */
  info: ProjectInfo | null;
  /** Whether rotom has taken the scripts over; without it the script buttons and the level-script form only show a hint. */
  scriptsAdopted: boolean;
  /** Changes when the script panel saved or closed a document (the level-script form reads its file again). */
  scriptsRevision: number;
  /** Opens a script file in the script panel (a level script as JSON). */
  onOpenScript: (file: number, line?: number) => void;
  /** The level-script form saved a document in `project`: its problems and an open document follow. */
  onLevelScriptSaved: (project: ProjectInfo | null, file: number, doc: LevelScriptDoc, dto: SaveScriptDto) => void;
  /** Whether `project` is still the open one (the form drops saves that wait while another project is opened). */
  isCurrentProject: (project: ProjectInfo | null) => boolean;
  /** A script file was added: the member counts (the pickers' limits) follow. */
  onCountsChanged: () => void | Promise<void>;
  onError: (e: unknown) => void;
}

/** The header fields that name a file of the script archive, with what a new file of that field is. */
const SCRIPT_FIELDS: Record<string, "script" | "levelScript"> = { script_file_id: "script", level_script_id: "levelScript" };

/**
 * Editable form of one map header; every committed field is one undoable change. Render it with
 * `key={id}`: all reads and writes of an instance belong to its one header (see `HeaderSession`).
 */
export function HeaderEditor({ id, context }: { id: number; context: HeaderEditorContext }) {
  const { headers, counts, revision, onChanged, goTo, eventRevision, onOpenWizard, onAddWarp, info, scriptsAdopted, scriptsRevision, onOpenScript, onLevelScriptSaved, isCurrentProject, onCountsChanged, onError } = context;
  const [header, setHeaderState] = useState<MapHeader | null>(null);
  // Bumped when an edit is refused, so the fields drop the rejected text they still show.
  const [refused, setRefused] = useState(0);
  const callbacks = useRef({ onChanged, onError });
  callbacks.current = { onChanged, onError };
  const session = useRef<HeaderSession | null>(null);

  // One session per mount. Disposing only cuts it off from this editor's state: a commit that
  // is already queued (also an uncommitted field flushed on unmount) still writes its own header.
  useEffect(() => {
    const s = new HeaderSession(
      id,
      { getHeader, setHeader },
      {
        onState: setHeaderState,
        onRefused: () => setRefused((n) => n + 1),
        onChanged: () => callbacks.current.onChanged(),
        onError: (e) => callbacks.current.onError(e),
      },
    );
    session.current = s;
    return () => s.dispose();
  }, [id]);

  // Load, and reload after undo/redo (queued behind pending commits). Keeps showing the old
  // values meanwhile.
  useEffect(() => {
    void session.current?.load();
  }, [id, revision]);

  const commit = (spec: FieldSpec, value: number) => void session.current?.commit(spec.key, spec.label, value);

  // A new script file: its number goes through the edit path like a typed one (undoable, the header is dirty).
  const createFile = async (key: string, kind: "script" | "levelScript") => {
    try {
      const n = await newScriptFile(kind);
      const label = kind === "script" ? TERMS.scriptFile.label : TERMS.levelScript.label;
      // Only a number that is in the header is worth opening (a refused write was reported; the new file stays unused).
      if (!(await session.current?.commit(key, label, n))) return;
      await onCountsChanged();
      if (kind === "script") onOpenScript(n);
    } catch (e) {
      onError(e);
    }
  };

  const summary = headers.find((h) => h.id === id);
  const title = `Header ${String(id).padStart(4, "0")}${summary ? ` · ${headerLabel(summary)}` : ""}`;
  return (
    <div style={{ padding: 16, fontFamily: "system-ui, sans-serif", fontSize: 13, color: "#111827" }}>
      <div style={{ fontSize: 14, fontWeight: 600 }}>{title}</div>
      {header === null ? (
        <p style={{ color: "#6b7280" }}>Lade Header ...</p>
      ) : (
        <div key={refused} style={{ marginTop: 8 }}>
          {headerFieldSpecs(headerFamily(header)).map((spec) => {
            const kind = SCRIPT_FIELDS[spec.key];
            const value = fieldValue(header, spec.key);
            return (
              <Fragment key={spec.key}>
                <Field spec={spec} value={value} max={spec.kind === "pick" && spec.pick ? pickMax(spec.pick, counts) : undefined} onCommit={commit} />
                {kind !== undefined && (
                  <div style={{ display: "flex", flexWrap: "wrap", alignItems: "center", justifyContent: "flex-end", gap: 8, padding: "2px 0" }}>
                    {!scriptsAdopted && (
                      <span role="note" style={{ fontSize: 12, color: "#6b7280", marginRight: "auto" }}>
                        {ADOPT_HINT}
                      </span>
                    )}
                    <button type="button" aria-label={`${spec.label} öffnen`} disabled={!scriptsAdopted || (kind === "levelScript" && value === 0)} onClick={() => onOpenScript(value)}>
                      Öffnen
                    </button>
                    <button type="button" aria-label={kind === "script" ? `Neue ${TERMS.scriptFile.label} anlegen` : `Neues ${TERMS.levelScript.label} anlegen`} disabled={!scriptsAdopted} onClick={() => void createFile(spec.key, kind)}>
                      Neu
                    </button>
                  </div>
                )}
              </Fragment>
            );
          })}
        </div>
      )}
      {header !== null && (
        <div style={{ display: "flex", flexWrap: "wrap", gap: 8, marginTop: 8 }}>
          <button type="button" onClick={() => goTo({ kind: "area", id: fieldValue(header, "area_data_id") })}>
            Area-Data {String(fieldValue(header, "area_data_id")).padStart(4, "0")} öffnen
          </button>
          <button type="button" onClick={() => goTo({ kind: "matrixCell", matrix: fieldValue(header, "matrix_id"), x: 0, y: 0 })} title="Zeigt die Matrix im Matrix-Panel">
            Matrix {String(fieldValue(header, "matrix_id")).padStart(4, "0")} öffnen
          </button>
        </div>
      )}
      {summary && header !== null && (
        <LevelScriptSection
          // A header of another project or another level script file starts a fresh form.
          key={`${id}:${fieldValue(header, "level_script_id")}`}
          header={summary}
          levelScriptId={fieldValue(header, "level_script_id")}
          project={info}
          adopted={scriptsAdopted}
          revision={scriptsRevision}
          onOpenScript={onOpenScript}
          onCreate={() => createFile("level_script_id", "levelScript")}
          onSaved={onLevelScriptSaved}
          isCurrentProject={isCurrentProject}
          onError={onError}
        />
      )}
      {summary && (
        <ExitsSection header={summary} headers={headers} revision={eventRevision} goTo={goTo} onOpenWizard={onOpenWizard} onAddWarp={onAddWarp} onError={onError} />
      )}
      <p style={{ marginTop: 12, fontSize: 12, color: "#6b7280" }}>Neue Header legen die Assistenten „Gebiet anlegen“ und „Raum anlegen“ an (Schritt „Header“, Option „Neuer Header“).</p>
    </div>
  );
}
