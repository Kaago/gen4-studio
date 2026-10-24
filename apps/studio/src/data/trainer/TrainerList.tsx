import { Fragment, memo, useMemo, useRef, useState } from "react";
import { createTrainer, errorText, isCmdError } from "../../api";
import type { CreateTrainerReport, DataRowDto } from "../../api";
import { Badge, Dialog, EmptyState, IconButton, Notice, Segmented, SelectField, TextField, TreeRow, useRovingList } from "../../ui/primitives";
import type { SelectOption } from "../../ui/primitives";
import { lockReason } from "../dataModel";
import { useDataArea, useDataSearch } from "../useDataArea";
import { GROUP_OPTIONS, PLACEHOLDER_TEXT, TRAINER_SCRIPTS, classOptions, groupRows, nameCount, nameError, nextTrainer, partialTrainer, rowLevel, trainerNameArchive } from "./trainerModel";
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
function CreateDialog({
  next,
  battleFile,
  defaultClass,
  onClose,
  onCreated,
  onPartial,
}: {
  next: { id: number; placeholder: number | null };
  /** The battle script `create_trainer` may extend: its open document is saved first (null: not known). */
  battleFile: number | null;
  defaultClass: number;
  onClose: () => void;
  onCreated: (report: CreateTrainerReport) => void;
  /** The backend failed after it wrote something (code "partial"): its message. */
  onPartial: (message: string) => void;
}) {
  const { names, scriptFiles } = useDataArea("pokemon");
  const [cls, setCls] = useState(defaultClass);
  const [name, setName] = useState("");
  const [refusal, setRefusal] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const options = useMemo<SelectOption<number>[]>(() => classOptions(names.classes, defaultClass), [names.classes, defaultClass]);
  const tooLong = nameError(name);
  const create = async () => {
    if (tooLong !== null || busy) return;
    setBusy(true);
    try {
      // The backend may write the battle script's source: unsaved text in the editor would overwrite it on the next save.
      if (battleFile !== null) await scriptFiles?.saveIfDirty(battleFile);
      onCreated(await createTrainer(cls, name));
    } catch (e) {
      if (isCmdError(e) && e.code === "partial") onPartial(e.message);
      else setRefusal(errorText(e));
    } finally {
      setBusy(false);
    }
  };
  const placeholder = next.placeholder === null ? "" : `${next.placeholder} wird ${PLACEHOLDER_TEXT}. `;
  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      title={`Trainer ${next.id} anlegen?`}
      description={`${placeholder}Er wird sofort ohne Pokémon angelegt; Team, Texte und Platz in der Welt folgen danach.`}
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
  const { rows, selected, select, labelOf, project, bus, onEdited, dirty, scriptFiles } = useDataArea("pokemon");
  const { shown, query, setQuery } = useDataSearch("pokemon");
  const [mode, setMode] = useState<GroupMode>("place");
  // Groups the user opened or closed; the others are open when they hold the selection.
  const [toggled, setToggled] = useState<Readonly<Record<string, boolean>>>({});
  const [creating, setCreating] = useState(false);
  const [created, setCreated] = useState<{ kind: "success" | "warning"; text: string } | null>(null);
  const host = useRef<HTMLDivElement>(null);
  useRovingList(host);

  const searching = query.trim() !== "";
  const groups = useMemo(() => groupRows(shown, mode), [shown, mode]);
  const unsaved = useMemo(() => new Set(dirty.filter((a) => a.kind === "Trainer").map((a) => a.index)), [dirty]);
  const isOpen = (g: RowGroup) => searching || (toggled[g.key] ?? g.rows.some((r) => r.index === selected));
  const toggle = (g: RowGroup) => setToggled((all) => ({ ...all, [g.key]: !isOpen(g) }));

  const family = project?.family ?? "Pt";
  const next = nextTrainer(rows, family);
  const battleFile = TRAINER_SCRIPTS[family]?.battleFile ?? null;
  const selectedRow = rows.find((r) => r.index === selected);
  const defaultClass = (selectedRow?.extra as { class?: number } | null)?.class ?? 0;
  /** What a create that wrote something leaves to catch up on, whether it went through or failed late. */
  const written = (id: number | null, extended: boolean, notice: { kind: "success" | "warning"; text: string }) => {
    setCreating(false);
    setCreated(notice);
    // The list, the name bank and the text tables changed behind the editors. Their history entry is an undo barrier: creating is not
    // undoable, and undo there says so (the kernel's reason, shown like any failed undo).
    bus.emit({ topic: "dataChanged", kind: "trainer", index: null });
    bus.emit({ topic: "textData", archive: trainerNameArchive(project) });
    if (extended) {
      // The battle script's source changed on disk, and its compile may have appended to text archives.
      if (battleFile !== null) void scriptFiles?.reload(battleFile);
      bus.emit({ topic: "textData", archive: null });
    }
    onEdited();
    if (id !== null) select(id);
  };
  const onCreated = (report: CreateTrainerReport) => {
    const made = report.placeholder === null ? `Trainer ${report.id} angelegt.` : `Trainer ${report.id} angelegt; ${report.placeholder} bleibt ${PLACEHOLDER_TEXT}.`;
    written(report.id, report.script_extended, { kind: "success", text: report.script_extended ? `${made} Das Kampf-Script hat neue Plätze bekommen.` : made });
  };
  const onPartial = (message: string) => {
    const partial = partialTrainer(message);
    const id = partial.written.includes(next.id) ? next.id : (partial.written[partial.written.length - 1] ?? null);
    written(id, partial.extended, { kind: "warning", text: partial.text });
  };

  return (
    <div className={styles.column}>
      <div className={styles.top}>
        <TextField type="search" label="Trainer durchsuchen" placeholder="Name, Klasse oder Nummer" value={query} onChange={setQuery} onCommit={setQuery} />
        <div className={styles.topRow}>
          <Segmented ariaLabel="Trainer gruppieren" size="s" options={GROUP_OPTIONS} value={mode} onChange={setMode} />
          <IconButton label="Trainer anlegen…" icon="plus" onClick={() => setCreating(true)} />
        </div>
        {created !== null && <Notice kind={created.kind}>{created.text}</Notice>}
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
      {creating && <CreateDialog next={next} battleFile={battleFile} defaultClass={defaultClass} onClose={() => setCreating(false)} onCreated={onCreated} onPartial={onPartial} />}
    </div>
  );
}
