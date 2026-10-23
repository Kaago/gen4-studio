import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { readScript, saveLevelScript } from "../../api";
import type { HeaderSummary, LevelScriptDoc, ProjectInfo, SaveScriptDto } from "../../api";
import { Button, Notice, NumberField, Section, SelectField } from "../../ui/primitives";
import { TERMS, id4 } from "../terms";
import styles from "./Editor.module.css";
import { useSectionOpen } from "./foldState";
import { CONDITION_MAX, ENTRY_SCRIPT_MAX, HEADER_ENTRY_TYPES, HINT_NO_MESSAGES, HINT_NO_MESSAGES_DETAIL, HINT_ZERO_VAR, LevelScriptSaves, TRIGGER_LABELS, addHeaderEntry, addVarCondition, headerEntries, removeEntry, runsDuringMapSetup, setEntryScript, setEntryType, setVarCondition, validationHints, varConditions } from "./levelScript";
import type { HeaderEntryType, VarCondition } from "./levelScript";
import table from "./LevelScriptSection.module.css";
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

const TRIGGER_OPTIONS = HEADER_ENTRY_TYPES.map((t) => ({ value: t, label: TRIGGER_LABELS[t] }));

/**
 * The "Level-Script" section of the header editor (spec §8.3): the entries rotom's JSON holds, as rows. Every change is
 * saved (and compiled) at once, in order and one at a time; the problems of a compile that fails go to the problems
 * store through `onSaved`. The reading and saving belong to this component, not to the section body, so folding the
 * section away neither drops the document nor a save that is under way.
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
  const [script, setScript] = useState(0);
  const [cond, setCond] = useState({ var: 0, value: 0, script: 0 });
  // Why the last add was refused.
  const [addHint, setAddHint] = useState<string | null>(null);
  const [open, toggle] = useSectionOpen("header:levelScript", true);

  const container = (children: ReactNode) => (
    <Section title={TERMS.levelScript.label} open={open} onToggle={toggle}>
      {children}
    </Section>
  );

  if (file === 0) {
    return container(
      <div className={styles.actions}>
        <span className={styles.note}>Kein Level-Script — </span>
        <Button kind="secondary" size="s" disabled={!adopted} onClick={() => void onCreate()}>
          Anlegen
        </Button>
        {!adopted && (
          <span role="note" className={styles.note}>
            {ADOPT_HINT}
          </span>
        )}
      </div>,
    );
  }
  if (!adopted) return container(<p className={styles.note}>{ADOPT_HINT}</p>);
  // What belongs to another file or project (the header changed, another project opened) is not shown.
  if (loaded === null || loaded.file !== file || loaded.project !== project) return container(<p className={styles.note}>Lade …</p>);
  if (loaded.doc === null) {
    return container(
      <div className={styles.actions}>
        <span className={styles.warn}>Datei {id4(file)} ist kein Level-Script</span>
        <Button kind="quiet" size="s" disabled>
          Als JSON öffnen
        </Button>
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
  const openNumber = (n: number) => void openScriptNumber(header.id, n, onOpenScript, onError);
  const entries = headerEntries(doc);
  const conditions = varConditions(doc);
  const hints = validationHints(doc);
  const numberField = (label: string, value: number, max: number, onCommit: (n: number) => void) => (
    <NumberField label={label} value={value} max={max} bits={max > CONDITION_MAX ? 32 : 16} onCommit={onCommit} />
  );
  const triggerField = (label: string, value: HeaderEntryType, onChange: (t: HeaderEntryType) => void) => <SelectField label={label} value={value} options={TRIGGER_OPTIONS} onChange={onChange} />;

  const addEntry = () => {
    const n = type === "on_var_condition" ? 0 : script;
    edit((d) => addHeaderEntry(d, type, n));
    setScript(0);
  };
  const addCondition = () => {
    // var 0 ends the table in the file: such a row is not added.
    if (cond.var === 0) return setAddHint(HINT_ZERO_VAR);
    edit((d) => addVarCondition(d, { var: cond.var, value: cond.value, script_id: cond.script }));
    setCond({ var: 0, value: 0, script: 0 });
  };
  const patchCondition = (index: number, patch: Partial<VarCondition>) => edit((d) => setVarCondition(d, index, patch));

  return container(
    <>
      <div className={styles.actions}>
        <span className={styles.mono}>
          {TERMS.levelScript.label} {id4(file)}
        </span>
        <Button kind="quiet" size="s" onClick={() => onOpenScript(file)} title={`Zeigt ${TERMS.levelScript.label} ${id4(file)} im Panel „Script“ als JSON`}>
          Als JSON öffnen
        </Button>
      </div>
      {hints.length > 0 && (
        <div className={styles.pad}>
          <Notice kind="warning">
            {hints.map((hint) => (
              <div key={hint}>{hint}</div>
            ))}
            <div>Nicht gespeichert, bis das Level-Script gültig ist.</div>
          </Notice>
        </div>
      )}
      <div className={table.scroll}>
        <table aria-label="Kopfeinträge" className={table.table}>
          <thead>
            <tr>
              <th>Auslöser</th>
              <th>{TERMS.script.label}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {entries.length === 0 && (
              <tr>
                <td colSpan={3} className={table.empty}>
                  Keine Kopfeinträge.
                </td>
              </tr>
            )}
            {entries.map((entry, i) => (
              <tr key={i}>
                <td>
                  {triggerField(`Auslöser ${i + 1}`, entry.type, (t) => edit((d) => setEntryType(d, i, t)))}
                  {runsDuringMapSetup(entry.type) && (
                    <p role="note" title={HINT_NO_MESSAGES_DETAIL} className={`${styles.warn} ${table.hint}`}>
                      {HINT_NO_MESSAGES}
                    </p>
                  )}
                </td>
                <td>
                  {entry.type === "on_var_condition" ? (
                    <span className={table.empty}>siehe unten</span>
                  ) : (
                    numberField(`${TERMS.script.label} ${i + 1}`, entry.script_id ?? 0, ENTRY_SCRIPT_MAX, (n) => edit((d) => setEntryScript(d, i, n)))
                  )}
                </td>
                <td>
                  <div className={table.buttons}>
                    <Button kind="quiet" size="s" disabled={entry.type === "on_var_condition"} onClick={() => openNumber(entry.script_id ?? 0)}>
                      Öffnen
                    </Button>
                    <Button kind="quiet" size="s" onClick={() => edit((d) => removeEntry(d, { list: "header_entries", index: i }))}>
                      Entfernen
                    </Button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className={table.add}>
        {triggerField("Neuer Auslöser", type, setType)}
        {type !== "on_var_condition" && numberField(`Neues ${TERMS.script.label}`, script, ENTRY_SCRIPT_MAX, setScript)}
        <Button kind="secondary" size="s" onClick={addEntry}>
          Eintrag hinzufügen
        </Button>
      </div>
      <div className={table.scroll}>
        <table aria-label="Variablen-Bedingungen" className={table.table}>
          <thead>
            <tr>
              <th>Variable</th>
              <th>Wert</th>
              <th>{TERMS.script.label}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {conditions.length === 0 && (
              <tr>
                <td colSpan={4} className={table.empty}>
                  Keine Variablen-Bedingungen.
                </td>
              </tr>
            )}
            {conditions.map((c, i) => (
              <tr key={i}>
                <td>{numberField(`Variable ${i + 1}`, c.var, CONDITION_MAX, (n) => patchCondition(i, { var: n }))}</td>
                <td>{numberField(`Wert ${i + 1}`, c.value, CONDITION_MAX, (n) => patchCondition(i, { value: n }))}</td>
                <td>{numberField(`${TERMS.script.label} der Bedingung ${i + 1}`, c.script_id, CONDITION_MAX, (n) => patchCondition(i, { script_id: n }))}</td>
                <td>
                  <div className={table.buttons}>
                    <Button kind="quiet" size="s" onClick={() => openNumber(c.script_id)}>
                      Öffnen
                    </Button>
                    <Button kind="quiet" size="s" onClick={() => edit((d) => removeEntry(d, { list: "var_conditions", index: i }))}>
                      Entfernen
                    </Button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className={table.add}>
        {numberField("Neue Variable", cond.var, CONDITION_MAX, (n) => setCond({ ...cond, var: n }))}
        {numberField("Neuer Wert", cond.value, CONDITION_MAX, (n) => setCond({ ...cond, value: n }))}
        {numberField(`${TERMS.script.label} der neuen Bedingung`, cond.script, CONDITION_MAX, (n) => setCond({ ...cond, script: n }))}
        <Button kind="secondary" size="s" onClick={addCondition}>
          Bedingung hinzufügen
        </Button>
      </div>
      {addHint !== null && (
        <div className={styles.pad}>
          <Notice kind="warning">{addHint}</Notice>
        </div>
      )}
    </>,
  );
}
