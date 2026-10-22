import { useEffect, useRef, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import { readScript, saveLevelScript } from "../../api";
import type { HeaderSummary, LevelScriptDoc, ProjectInfo, SaveScriptDto } from "../../api";
import { DraftInput } from "../DraftInput";
import { TERMS, id4 } from "../terms";
import { CONDITION_MAX, ENTRY_SCRIPT_MAX, HEADER_ENTRY_TYPES, HINT_ZERO_VAR, LevelScriptSaves, TRIGGER_LABELS, addHeaderEntry, addVarCondition, clampNumber, headerEntries, removeEntry, setEntryScript, setEntryType, setVarCondition, validationHints, varConditions } from "./levelScript";
import type { HeaderEntryType, VarCondition } from "./levelScript";
import { ADOPT_HINT, openScriptNumber } from "./scriptNav";

interface Props {
  header: HeaderSummary;
  /** The header's level script file (0: none). */
  levelScriptId: number;
  /** The open project; another object means another one was opened, and the form starts over. */
  project: ProjectInfo | null;
  /** Whether rotom has taken the scripts over; without it the form only shows a hint. */
  adopted: boolean;
  /** Changes when the script panel saved or closed a document: the form reads the file again. */
  revision: number;
  onOpenScript: (file: number, line?: number) => void;
  /** Creates a level script file and writes its number into the header (the editor's edit path). */
  onCreate: () => Promise<void>;
  /** A form change was saved (compiled): the problems and the open document follow. `project` is the one the form saved in. */
  onSaved: (project: ProjectInfo | null, file: number, doc: LevelScriptDoc, dto: SaveScriptDto) => void;
  /** Whether `project` is still the open one (a save that waits for its turn is dropped when it is not). */
  isCurrentProject: (project: ProjectInfo | null) => boolean;
  onError: (e: unknown) => void;
}

/** One queue for the whole app (see `LevelScriptSaves`). */
const saves = new LevelScriptSaves((file, doc) => saveLevelScript(file, doc));

type Loaded = { project: ProjectInfo | null; file: number; doc: LevelScriptDoc | null };

const heading: CSSProperties = { fontWeight: 600, marginBottom: 4 };
const muted: CSSProperties = { margin: "4px 0", color: "#6b7280" };
const cell: CSSProperties = { padding: "1px 4px 1px 0", textAlign: "left", fontWeight: 400 };
const field: CSSProperties = { width: 72, fontFamily: "ui-monospace, monospace" };
const rowStyle: CSSProperties = { display: "flex", flexWrap: "wrap", alignItems: "center", gap: 8, margin: "4px 0" };

/**
 * The "Level-Script" section of the header editor (spec §8.3): the entries rotom's JSON holds, as rows. Every change is
 * saved (and compiled) at once, in order and one at a time; the problems of a compile that fails go to the problems
 * store through `onSaved`.
 */
export function LevelScriptSection({ header, levelScriptId, project, adopted, revision, onOpenScript, onCreate, onSaved, isCurrentProject, onError }: Props) {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const callbacks = useRef({ onSaved, isCurrentProject, onError });
  callbacks.current = { onSaved, isCurrentProject, onError };
  // The document the form shows, kept synchronously so two changes in one tick build on each other; `edits` counts
  // them, so a read that was started before a change cannot bring the older text back.
  const docRef = useRef<LevelScriptDoc | null>(null);
  const edits = useRef(0);
  // True while the form holds a document rotom would reject: it is not saved, and a re-read must not drop it.
  const held = useRef(false);
  // Bumped when a save failed: the form reads the file again, so it does not show entries that were never written.
  const [failures, setFailures] = useState(0);
  const file = levelScriptId;

  useEffect(() => {
    if (file === 0 || !adopted || held.current) return;
    let current = true;
    const read = () => {
      if (!current) return;
      const seen = edits.current;
      readScript(file).then(
        (source) => {
          if (!current || edits.current !== seen) return;
          const doc = source.kind === "levelScript" ? source.doc : null;
          docRef.current = doc;
          setLoaded({ project, file, doc });
        },
        (e: unknown) => current && callbacks.current.onError(e),
      );
    };
    // A save of the file in flight has newer text than the disk: read when it (and the ones behind it) are done.
    const stopWaiting = saves.busy(file) ? saves.whenIdle(file, read) : (read(), () => {});
    return () => {
      current = false;
      stopWaiting();
    };
  }, [project, file, adopted, revision, failures]);

  const [type, setType] = useState<HeaderEntryType>("on_load");
  const [script, setScript] = useState("");
  const [cond, setCond] = useState({ var: "", value: "", script: "" });
  // Why the last add was refused.
  const [addHint, setAddHint] = useState<string | null>(null);

  const container = (children: ReactNode) => (
    <section aria-label={TERMS.levelScript.label} style={{ marginTop: 12 }}>
      <div style={heading}>{TERMS.levelScript.label}</div>
      {children}
    </section>
  );

  if (file === 0) {
    return container(
      <div style={rowStyle}>
        <span style={{ color: "#6b7280" }}>Kein Level-Script — </span>
        <button type="button" disabled={!adopted} onClick={() => void onCreate()}>
          Anlegen
        </button>
        {!adopted && <span role="note" style={{ color: "#6b7280" }}>{ADOPT_HINT}</span>}
      </div>,
    );
  }
  if (!adopted) return container(<p style={muted}>{ADOPT_HINT}</p>);
  // What belongs to another file or project (the header changed, another project opened) is not shown.
  if (loaded === null || loaded.file !== file || loaded.project !== project) return container(<p style={muted}>Lade …</p>);
  if (loaded.doc === null) {
    return container(
      <div style={rowStyle}>
        <span style={{ color: "#b45309" }}>Datei {id4(file)} ist kein Level-Script</span>
        <button type="button" disabled>
          Als JSON öffnen
        </button>
      </div>,
    );
  }

  const doc = loaded.doc;
  // `change` builds the new document from the latest one, also when an earlier change has not rendered yet.
  const edit = (change: (d: LevelScriptDoc) => LevelScriptDoc) => {
    const next = change(docRef.current ?? doc);
    edits.current += 1;
    docRef.current = next;
    setLoaded({ project, file, doc: next });
    setAddHint(null);
    // A document rotom would reject is shown with its hints and not sent; the next valid one carries every change.
    held.current = validationHints(next).length > 0;
    if (held.current) return;
    saves.push(file, {
      doc: next,
      current: () => callbacks.current.isCurrentProject(project),
      done: (dto) => callbacks.current.onSaved(project, file, next, dto),
      fail: (e) => {
        callbacks.current.onError(e);
        setFailures((n) => n + 1);
      },
    });
  };
  const open = (n: number) => void openScriptNumber(header.id, n, onOpenScript, onError);
  const number = (text: string, max: number) => (text.trim() === "" ? null : clampNumber(Number(text), max));
  const entries = headerEntries(doc);
  const conditions = varConditions(doc);
  const hints = validationHints(doc);
  const numberInput = (label: string, value: number, max: number, onCommit: (n: number) => void) => (
    <DraftInput
      label={label}
      type="number"
      min={0}
      max={max}
      value={String(value)}
      onCommit={(text) => {
        const n = number(text, max);
        if (n !== null && n !== value) onCommit(n);
      }}
      style={field}
    />
  );
  const draftInput = (label: string, value: string, onChange: (v: string) => void, max: number) => (
    <input type="number" aria-label={label} title={label} min={0} max={max} value={value} onChange={(e) => onChange(e.target.value)} style={{ ...field, boxSizing: "border-box", padding: "2px 6px" }} />
  );
  const typeSelect = (label: string, value: HeaderEntryType, onChange: (t: HeaderEntryType) => void) => (
    <select aria-label={label} value={value} onChange={(e) => onChange(e.target.value as HeaderEntryType)}>
      {HEADER_ENTRY_TYPES.map((t) => (
        <option key={t} value={t}>
          {TRIGGER_LABELS[t]}
        </option>
      ))}
    </select>
  );

  const addEntry = () => {
    const n = type === "on_var_condition" ? 0 : (number(script, ENTRY_SCRIPT_MAX) ?? 0);
    edit((d) => addHeaderEntry(d, type, n));
    setScript("");
  };
  const addCondition = () => {
    // var 0 ends the table in the file: such a row is not added.
    if ((number(cond.var, CONDITION_MAX) ?? 0) === 0) return setAddHint(HINT_ZERO_VAR);
    const v = { var: number(cond.var, CONDITION_MAX) ?? 0, value: number(cond.value, CONDITION_MAX) ?? 0, script_id: number(cond.script, CONDITION_MAX) ?? 0 };
    edit((d) => addVarCondition(d, v));
    setCond({ var: "", value: "", script: "" });
  };
  const patchCondition = (index: number, patch: Partial<VarCondition>) => edit((d) => setVarCondition(d, index, patch));

  return container(
    <>
      <div style={rowStyle}>
        <span style={{ color: "#6b7280" }}>
          {TERMS.levelScript.label} {id4(file)}
        </span>
        <button type="button" onClick={() => onOpenScript(file)} title={`Zeigt ${TERMS.levelScript.label} ${id4(file)} im Panel „Script“ als JSON`}>
          Als JSON öffnen
        </button>
      </div>
      {hints.length > 0 && (
        <div role="status" style={{ color: "#b45309", margin: "4px 0" }}>
          {hints.map((hint) => (
            <div key={hint}>{hint}</div>
          ))}
          <div style={{ color: "#6b7280" }}>Nicht gespeichert, bis das Level-Script gültig ist.</div>
        </div>
      )}
      <table aria-label="Kopfeinträge" style={{ borderCollapse: "collapse" }}>
        <thead>
          <tr>
            <th style={cell}>Auslöser</th>
            <th style={cell}>{TERMS.script.label}</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {entries.length === 0 && (
            <tr>
              <td colSpan={3} style={{ ...cell, color: "#6b7280" }}>
                Keine Kopfeinträge.
              </td>
            </tr>
          )}
          {entries.map((entry, i) => (
            <tr key={i}>
              <td style={cell}>{typeSelect(`Auslöser ${i + 1}`, entry.type, (t) => edit((d) => setEntryType(d, i, t)))}</td>
              <td style={cell}>{entry.type === "on_var_condition" ? <span style={{ color: "#6b7280" }}>siehe unten</span> : numberInput(`${TERMS.script.label} ${i + 1}`, entry.script_id ?? 0, ENTRY_SCRIPT_MAX, (n) => edit((d) => setEntryScript(d, i, n)))}</td>
              <td style={cell}>
                <button type="button" disabled={entry.type === "on_var_condition"} onClick={() => open(entry.script_id ?? 0)}>
                  Öffnen
                </button>{" "}
                <button type="button" onClick={() => edit((d) => removeEntry(d, { list: "header_entries", index: i }))}>
                  Entfernen
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div style={rowStyle}>
        {typeSelect("Neuer Auslöser", type, setType)}
        {type !== "on_var_condition" && draftInput(`Neues ${TERMS.script.label}`, script, setScript, ENTRY_SCRIPT_MAX)}
        <button type="button" onClick={addEntry}>
          Eintrag hinzufügen
        </button>
      </div>
      <table aria-label="Variablen-Bedingungen" style={{ borderCollapse: "collapse", marginTop: 8 }}>
        <thead>
          <tr>
            <th style={cell}>Variable</th>
            <th style={cell}>Wert</th>
            <th style={cell}>{TERMS.script.label}</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {conditions.length === 0 && (
            <tr>
              <td colSpan={4} style={{ ...cell, color: "#6b7280" }}>
                Keine Variablen-Bedingungen.
              </td>
            </tr>
          )}
          {conditions.map((c, i) => (
            <tr key={i}>
              <td style={cell}>{numberInput(`Variable ${i + 1}`, c.var, CONDITION_MAX, (n) => patchCondition(i, { var: n }))}</td>
              <td style={cell}>{numberInput(`Wert ${i + 1}`, c.value, CONDITION_MAX, (n) => patchCondition(i, { value: n }))}</td>
              <td style={cell}>{numberInput(`${TERMS.script.label} der Bedingung ${i + 1}`, c.script_id, CONDITION_MAX, (n) => patchCondition(i, { script_id: n }))}</td>
              <td style={cell}>
                <button type="button" onClick={() => open(c.script_id)}>
                  Öffnen
                </button>{" "}
                <button type="button" onClick={() => edit((d) => removeEntry(d, { list: "var_conditions", index: i }))}>
                  Entfernen
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div style={rowStyle}>
        {draftInput("Neue Variable", cond.var, (v) => setCond({ ...cond, var: v }), CONDITION_MAX)}
        {draftInput("Neuer Wert", cond.value, (v) => setCond({ ...cond, value: v }), CONDITION_MAX)}
        {draftInput(`${TERMS.script.label} der neuen Bedingung`, cond.script, (v) => setCond({ ...cond, script: v }), CONDITION_MAX)}
        <button type="button" onClick={addCondition}>
          Bedingung hinzufügen
        </button>
      </div>
      {addHint !== null && (
        <div role="status" style={{ color: "#b45309", margin: "4px 0" }}>
          {addHint}
        </div>
      )}
    </>,
  );
}
