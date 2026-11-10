import { useEffect, useRef, useState } from "react";
import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { readScript, saveLevelScript } from "../../api";
import type { FlagKind, HeaderSummary, LevelScriptDoc, ProjectInfo, SaveScriptDto } from "../../api";
import { FlagField } from "../../flags/FlagField";
import type { Depth } from "../../ui/frame/shellState";
import { Button, Notice, NumberField, Section, SelectField } from "../../ui/primitives";
import { id4, termLabel } from "../terms";
import styles from "./Editor.module.css";
import { useSectionOpen } from "./foldState";
import { CONDITION_MAX, ENTRY_SCRIPT_MAX, HEADER_ENTRY_TYPES, LevelScriptSaves, TRIGGER_LABELS, addHeaderEntry, addVarCondition, headerEntries, hintNoMessages, hintNoMessagesDetail, hintZeroVar, removeEntry, runsDuringMapSetup, setEntryScript, setEntryType, setVarCondition, validationHints, varConditions } from "./levelScript";
import type { HeaderEntryType, VarCondition } from "./levelScript";
import table from "./LevelScriptSection.module.css";
import { adoptHint, openScriptNumber } from "./scriptNav";

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
  /** Bauen shows a condition's variable by its name with a picker; Tüfteln adds the number field. */
  depth: Depth;
  /** Opens the list of flags and variables on a number („Im Verzeichnis zeigen“ of the variable pickers). */
  onShowFlag?: (kind: FlagKind, number: number) => void;
}

/** One queue for the whole app (see `LevelScriptSaves`). */
const saves = new LevelScriptSaves((file, doc) => saveLevelScript(file, doc));

type Loaded = { project: ProjectInfo | null; file: number; doc: LevelScriptDoc | null };

/** The choices of a trigger field, named in the language that is shown when called. */
const triggerOptions = () => HEADER_ENTRY_TYPES.map((type) => ({ value: type, label: TRIGGER_LABELS[type] }));

/**
 * The "Level-Script" section of the header editor (spec §8.3): the entries rotom's JSON holds, as rows. Every change is
 * saved (and compiled) at once, in order and one at a time; the problems of a compile that fails go to the problems
 * store through `onSaved`. The reading and saving belong to this component, not to the section body, so folding the
 * section away neither drops the document nor a save that is under way.
 */
export function LevelScriptSection({ header, levelScriptId, project, adopted, revision, onOpenScript, onCreate, onSaved, isCurrentProject, onError, depth, onShowFlag }: Props) {
  const { t } = useTranslation("workbench");
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
    <Section title={termLabel("levelScript")} open={open} onToggle={toggle}>
      {children}
    </Section>
  );

  if (file === 0) {
    return container(
      <div className={styles.actions}>
        <span className={styles.note}>{t("levelScriptSection.none")}</span>
        <Button kind="secondary" size="s" disabled={!adopted} onClick={() => void onCreate()}>
          {t("levelScriptSection.create")}
        </Button>
        {!adopted && (
          <span role="note" className={styles.note}>
            {adoptHint()}
          </span>
        )}
      </div>,
    );
  }
  if (!adopted) return container(<p className={styles.note}>{adoptHint()}</p>);
  // What belongs to another file or project (the header changed, another project opened) is not shown.
  if (loaded === null || loaded.file !== file || loaded.project !== project) return container(<p className={styles.note}>{t("levelScriptSection.loading")}</p>);
  if (loaded.doc === null) {
    return container(
      <div className={styles.actions}>
        <span className={styles.warn}>{t("levelScriptSection.notALevelScript", { file: id4(file) })}</span>
        <Button kind="quiet" size="s" disabled>
          {t("levelScriptSection.openAsJson")}
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
  const options = triggerOptions();
  const triggerField = (label: string, value: HeaderEntryType, onChange: (type: HeaderEntryType) => void) => <SelectField label={label} value={value} options={options} onChange={onChange} />;
  // A condition's variable: its name and the picker, in Tüfteln with the number field (`field` names the chip: `var:1`, `var:new`).
  const showVar = onShowFlag ? (n: number) => onShowFlag("var", n) : undefined;
  const varField = (label: string, field: string, value: number, onCommit: (n: number) => void) => (
    <FlagField kind="var" label={label} value={value} onCommit={onCommit} depth={depth} project={project} revision={revision} source="scripts" onShowInDirectory={showVar} field={field} bare />
  );

  const addEntry = () => {
    const n = type === "on_var_condition" ? 0 : script;
    edit((d) => addHeaderEntry(d, type, n));
    setScript(0);
  };
  const addCondition = () => {
    // var 0 ends the table in the file: such a row is not added.
    if (cond.var === 0) return setAddHint(hintZeroVar());
    edit((d) => addVarCondition(d, { var: cond.var, value: cond.value, script_id: cond.script }));
    setCond({ var: 0, value: 0, script: 0 });
  };
  const patchCondition = (index: number, patch: Partial<VarCondition>) => edit((d) => setVarCondition(d, index, patch));

  return container(
    <>
      <div className={styles.actions}>
        <span className={styles.mono}>
          {termLabel("levelScript")} {id4(file)}
        </span>
        <Button kind="quiet" size="s" onClick={() => onOpenScript(file)} title={t("levelScriptSection.openAsJsonTitle", { file: id4(file) })}>
          {t("levelScriptSection.openAsJson")}
        </Button>
      </div>
      {hints.length > 0 && (
        <div className={styles.pad}>
          <Notice kind="warning">
            {hints.map((hint) => (
              <div key={hint}>{hint}</div>
            ))}
            <div>{t("levelScriptSection.notSaved")}</div>
          </Notice>
        </div>
      )}
      <div className={table.scroll}>
        <table aria-label={t("levelScriptSection.headerEntries")} className={table.table}>
          <thead>
            <tr>
              <th>{t("levelScriptSection.trigger")}</th>
              <th>{termLabel("script")}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {entries.length === 0 && (
              <tr>
                <td colSpan={3} className={table.empty}>
                  {t("levelScriptSection.noHeaderEntries")}
                </td>
              </tr>
            )}
            {entries.map((entry, i) => (
              <tr key={i}>
                <td>
                  {triggerField(t("levelScriptSection.triggerN", { n: i + 1 }), entry.type, (type) => edit((d) => setEntryType(d, i, type)))}
                  {runsDuringMapSetup(entry.type) && (
                    <p role="note" title={hintNoMessagesDetail()} className={`${styles.warn} ${table.hint}`}>
                      {hintNoMessages()}
                    </p>
                  )}
                </td>
                <td>
                  {entry.type === "on_var_condition" ? (
                    <span className={table.empty}>{t("levelScriptSection.seeBelow")}</span>
                  ) : (
                    numberField(`${termLabel("script")} ${i + 1}`, entry.script_id ?? 0, ENTRY_SCRIPT_MAX, (n) => edit((d) => setEntryScript(d, i, n)))
                  )}
                </td>
                <td>
                  <div className={table.buttons}>
                    <Button kind="quiet" size="s" disabled={entry.type === "on_var_condition"} onClick={() => openNumber(entry.script_id ?? 0)}>
                      {t("levelScriptSection.open")}
                    </Button>
                    <Button kind="quiet" size="s" onClick={() => edit((d) => removeEntry(d, { list: "header_entries", index: i }))}>
                      {t("levelScriptSection.remove")}
                    </Button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className={table.add}>
        {triggerField(t("levelScriptSection.newTrigger"), type, setType)}
        {type !== "on_var_condition" && numberField(t("levelScriptSection.newScript"), script, ENTRY_SCRIPT_MAX, setScript)}
        <Button kind="secondary" size="s" onClick={addEntry}>
          {t("levelScriptSection.addEntry")}
        </Button>
      </div>
      <div className={table.scroll}>
        <table aria-label={t("levelScriptSection.varConditions")} className={table.table}>
          <thead>
            <tr>
              <th>{t("levelScriptSection.variable")}</th>
              <th>{t("levelScriptSection.value")}</th>
              <th>{termLabel("script")}</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {conditions.length === 0 && (
              <tr>
                <td colSpan={4} className={table.empty}>
                  {t("levelScriptSection.noVarConditions")}
                </td>
              </tr>
            )}
            {conditions.map((c, i) => (
              <tr key={i}>
                <td>{varField(t("levelScriptSection.variableN", { n: i + 1 }), `var:${i + 1}`, c.var, (n) => patchCondition(i, { var: n }))}</td>
                <td>{numberField(t("levelScriptSection.valueN", { n: i + 1 }), c.value, CONDITION_MAX, (n) => patchCondition(i, { value: n }))}</td>
                <td>{numberField(t("levelScriptSection.conditionScriptN", { n: i + 1 }), c.script_id, CONDITION_MAX, (n) => patchCondition(i, { script_id: n }))}</td>
                <td>
                  <div className={table.buttons}>
                    <Button kind="quiet" size="s" onClick={() => openNumber(c.script_id)}>
                      {t("levelScriptSection.open")}
                    </Button>
                    <Button kind="quiet" size="s" onClick={() => edit((d) => removeEntry(d, { list: "var_conditions", index: i }))}>
                      {t("levelScriptSection.remove")}
                    </Button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
      <div className={table.add}>
        {varField(t("levelScriptSection.newVariable"), "var:new", cond.var, (n) => setCond((c) => ({ ...c, var: n })))}
        {numberField(t("levelScriptSection.newValue"), cond.value, CONDITION_MAX, (n) => setCond({ ...cond, value: n }))}
        {numberField(t("levelScriptSection.newConditionScript"), cond.script, CONDITION_MAX, (n) => setCond({ ...cond, script: n }))}
        <Button kind="secondary" size="s" onClick={addCondition}>
          {t("levelScriptSection.addCondition")}
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
