import { Fragment, memo, useMemo, useRef, useState } from "react";
import { createTrainer, errorText } from "../../api";
import type { CreateTrainerReport, DataRowDto } from "../../api";
import { Badge, Dialog, EmptyState, IconButton, Notice, Segmented, SelectField, TextField, TreeRow, useRovingList } from "../../ui/primitives";
import type { SelectOption } from "../../ui/primitives";
import { lockReason } from "../dataModel";
import { useDataArea, useDataSearch } from "../useDataArea";
import { GROUP_OPTIONS, PLACEHOLDER_TEXT, className, groupRows, nameCount, nameError, rowLevel, trainerNameArchive } from "./trainerModel";
import type { GroupMode, RowGroup } from "./trainerModel";
import styles from "./Trainer.module.css";

/** One trainer under its place or class: "Wanderer Jens", its number, "Lv. 12" (the highest level), the unsaved dot, a lock. */
const TrainerRow = memo(function TrainerRow({ row, label, selected, unsaved, onSelect }: { row: DataRowDto; label: string; selected: boolean; unsaved: boolean; onSelect: (index: number) => void }) {
  const reason = row.locked === null ? null : lockReason(row.locked);
  const level = rowLevel(row);
  return (
    <TreeRow
      depth={1}
      label={label}
      tech={row.tech}
      selected={selected}
      onSelect={() => onSelect(row.index)}
      title={reason ?? undefined}
      ariaLabel={reason === null ? `${label}, ${row.tech}` : `${label}, ${row.tech}, gesperrt: ${reason}`}
      dataAttrs={{ "data-trainer": row.index }}
      trailing={
        <span className={styles.rowEnd}>
          {level !== null && <span className={styles.level}>{`Lv. ${level}`}</span>}
          {unsaved && <span className={styles.dot} role="img" aria-label="ungespeichert" />}
          {reason !== null && <Badge kind="locked" title={reason} />}
        </span>
      }
    />
  );
});

/**
 * The question before a trainer is made: its class (from the class bank) and its name (at most 7 characters). Making it is written at
 * once and cannot be undone (badge); a refusal of the backend (scripts not adopted, a full flag range) shows in the dialog.
 */
function CreateDialog({ nextId, defaultClass, onClose, onCreated }: { nextId: number; defaultClass: number; onClose: () => void; onCreated: (report: CreateTrainerReport) => void }) {
  const { names } = useDataArea("pokemon");
  const [cls, setCls] = useState(defaultClass);
  const [name, setName] = useState("");
  const [refusal, setRefusal] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const options = useMemo<SelectOption<number>[]>(() => names.classes.map((_, i) => ({ value: i, label: className(names.classes, i) })), [names.classes]);
  const tooLong = nameError(name);
  const create = async () => {
    if (tooLong !== null || busy) return;
    setBusy(true);
    try {
      onCreated(await createTrainer(cls, name));
    } catch (e) {
      setRefusal(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      title={`Trainer ${nextId} anlegen?`}
      description="Er wird sofort ohne Pokémon angelegt; Team, Texte und Platz in der Welt folgen danach."
      actions={[
        { label: "Abbrechen", onClick: onClose },
        { label: "Anlegen", kind: "primary", onClick: () => void create() },
      ]}
    >
      <div className={styles.dialogBody}>
        <span>
          <Badge kind="irreversible" />
        </span>
        <SelectField label="Klasse" showLabel value={cls} options={options} onChange={setCls} />
        <TextField label="Name" showLabel value={name} onChange={setName} onCommit={setName} hint={`Höchstens 7 Zeichen, jetzt ${nameCount(name)}`} error={tooLong ?? undefined} />
        {refusal !== null && <Notice kind="error">{refusal}</Notice>}
      </div>
    </Dialog>
  );
}

/**
 * The trainers as a tree (Spec §10): by the places whose object events start their battle ("Ohne Ort" last) or by class. A place opens
 * when it holds the selection, every place with a hit while searching (name, class or number). `+` asks for class and name and makes a
 * trainer at the next number, which is then selected. The placeholder of the sight script slot is listed locked.
 */
export function TrainerList() {
  const { rows, selected, select, labelOf, project, bus, onEdited, dirty } = useDataArea("pokemon");
  const { shown, query, setQuery } = useDataSearch("pokemon");
  const [mode, setMode] = useState<GroupMode>("place");
  // Groups the user opened or closed; the others are open when they hold the selection.
  const [toggled, setToggled] = useState<Readonly<Record<string, boolean>>>({});
  const [creating, setCreating] = useState(false);
  const [created, setCreated] = useState<string | null>(null);
  const host = useRef<HTMLDivElement>(null);
  useRovingList(host);

  const searching = query.trim() !== "";
  const groups = useMemo(() => groupRows(shown, mode), [shown, mode]);
  const unsaved = useMemo(() => new Set(dirty.filter((a) => a.kind === "Trainer").map((a) => a.index)), [dirty]);
  const isOpen = (g: RowGroup) => searching || (toggled[g.key] ?? g.rows.some((r) => r.index === selected));
  const toggle = (g: RowGroup) => setToggled((all) => ({ ...all, [g.key]: !isOpen(g) }));

  const nextId = rows.length === 0 ? 0 : Math.max(...rows.map((r) => r.index)) + 1;
  const selectedRow = rows.find((r) => r.index === selected);
  const defaultClass = (selectedRow?.extra as { class?: number } | null)?.class ?? 0;
  const onCreated = (report: CreateTrainerReport) => {
    setCreating(false);
    const made = report.placeholder === null ? `Trainer ${report.id} angelegt.` : `Trainer ${report.id} angelegt; ${report.placeholder} bleibt ${PLACEHOLDER_TEXT}.`;
    setCreated(report.script_extended ? `${made} Das Kampf-Script hat neue Plätze bekommen.` : made);
    // The list, the name bank and the text tables changed behind the editors; the name and text offset are one undo step.
    bus.emit({ topic: "dataChanged", kind: "trainer", index: null });
    bus.emit({ topic: "textData", archive: trainerNameArchive(project) });
    onEdited();
    select(report.id);
  };

  return (
    <div className={styles.column}>
      <div className={styles.top}>
        <TextField type="search" label="Trainer durchsuchen" placeholder="Name, Klasse oder Nummer" value={query} onChange={setQuery} onCommit={setQuery} />
        <div className={styles.topRow}>
          <Segmented ariaLabel="Trainer gruppieren" size="s" options={GROUP_OPTIONS} value={mode} onChange={setMode} />
          <IconButton label="Trainer anlegen…" icon="plus" onClick={() => setCreating(true)} />
        </div>
        {created !== null && <Notice kind="success">{created}</Notice>}
      </div>
      {groups.length === 0 && searching ? (
        <EmptyState what={`Nichts gefunden für „${query.trim()}“`} why="Suche nach Name, Klasse oder Nummer." />
      ) : (
        <div ref={host} role="tree" aria-label="Trainer" className={styles.host}>
          {groups.map((g) => {
            const open = isOpen(g);
            return (
              <Fragment key={g.key}>
                <TreeRow
                  depth={0}
                  label={g.label}
                  selected={false}
                  expanded={open}
                  onToggle={() => toggle(g)}
                  onSelect={() => toggle(g)}
                  dataAttrs={{ "data-group": g.key }}
                  trailing={<span className={styles.count}>{g.rows.length}</span>}
                />
                {open &&
                  g.rows.map((row) => (
                    <TrainerRow key={row.index} row={row} label={labelOf(row)} selected={row.index === selected} unsaved={unsaved.has(row.index)} onSelect={select} />
                  ))}
              </Fragment>
            );
          })}
        </div>
      )}
      {creating && <CreateDialog nextId={nextId} defaultClass={defaultClass} onClose={() => setCreating(false)} onCreated={onCreated} />}
    </div>
  );
}
