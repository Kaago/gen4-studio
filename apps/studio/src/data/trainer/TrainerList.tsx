import { Fragment, memo, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { createTrainer, errorText, isCmdError } from "../../api";
import type { CreateTrainerReport, DataRowDto, TrainerRowExtra } from "../../api";
import { FindingMark, useFindingMarks } from "../../checks/FindingMark";
import { restoreEntry, useDelete } from "../../delete/DeleteContext";
import { ShowDeleted } from "../../delete/ShowDeleted";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import type { MarkSeverity } from "../../checks/FindingMark";
import { useShell } from "../../ui/frame/ShellContext";
import { depthAtLeast, viewDepth } from "../../ui/frame/shellState";
import { Badge, Button, Dialog, EmptyState, EntityStateBadge, IconButton, Notice, Segmented, SelectField, TextField, TreeRow, useRovingList } from "../../ui/primitives";
import type { SelectOption } from "../../ui/primitives";
import { recipeRequest } from "../../workbench/recipes";
import { lockReason } from "../dataModel";
import { useDataArea, useDataSearch } from "../useDataArea";
import { TRAINER_SCRIPTS, classOptions, groupOptions, groupRows, nameCount, nameError, nextTrainer, partialTrainer, rowLevel, trainerNameArchive } from "./trainerModel";
import type { GroupMode, RowGroup } from "./trainerModel";
import styles from "./Trainer.module.css";

/**
 * One trainer under its place or class: "Wanderer Jens", its number, "Lv. 12" (the highest level), what the checks found, the unsaved dot,
 * a lock; a tombstone with the badge „gelöscht“ (the name gives way to the number and the badge) and „Wiederherstellen“ before the reset in its context menu.
 */
const TrainerRow = memo(function TrainerRow({
  row,
  label,
  selected,
  unsaved,
  mark,
  onSelect,
}: {
  row: DataRowDto;
  label: string;
  selected: boolean;
  unsaved: boolean;
  mark: MarkSeverity | undefined;
  onSelect: (index: number) => void;
}) {
  const { t } = useTranslation("data");
  const reason = row.locked === null ? null : lockReason(row.locked);
  const level = rowLevel(row);
  const reset = useReset();
  const { restore } = useDelete();
  const deleted = row.deleted === true;
  const resetItem = resetEntry({ kind: "trainer", index: row.index }, reset.status, reset.open);
  return (
    <TreeRow
      contextMenu={deleted ? [restoreEntry({ kind: "trainer", index: row.index }, restore), resetItem] : [resetItem]}
      depth={1}
      label={label}
      tech={row.tech}
      keepTech
      selected={selected}
      onSelect={() => onSelect(row.index)}
      title={reason ?? undefined}
      ariaLabel={[label, row.tech, ...(deleted ? [t("trainerList.deleted")] : []), ...(reason === null ? [] : [t("trainerList.locked", { reason })])].join(", ")}
      dataAttrs={{ "data-trainer": row.index, "data-deleted": deleted ? "true" : undefined }}
      trailing={
        <span className={styles.rowEnd}>
          {deleted && <EntityStateBadge state="deleted" />}
          {level !== null && <span className={styles.level}>{t("trainerList.level", { level })}</span>}
          {mark && <FindingMark severity={mark} />}
          {unsaved && <span className={styles.dot} role="img" aria-label={t("unsaved")} />}
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
  const { t } = useTranslation("data");
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
  return (
    <Dialog
      open
      onOpenChange={(open) => !open && onClose()}
      title={t("trainerList.createTitle", { id: next.id })}
      description={next.placeholder === null ? t("trainerList.createDescription") : t("trainerList.createDescriptionPlaceholder", { id: next.placeholder })}
      actions={[
        { label: t("trainerList.cancel"), onClick: onClose },
        { label: t("trainerList.create"), kind: "primary", onClick: () => void create() },
      ]}
    >
      <div className={styles.dialogBody}>
        <span>
          <Badge kind="irreversible" />
        </span>
        <SelectField label={t("trainerList.class")} showLabel value={cls} options={options} onChange={setCls} />
        <TextField label={t("trainerList.name")} showLabel value={name} onChange={setName} onCommit={setName} hint={t("trainerList.nameHint", { current: nameCount(name) })} error={tooLong ?? undefined} />
        {refusal !== null && <Notice kind="error">{refusal}</Notice>}
      </div>
    </Dialog>
  );
}

/**
 * The trainers as a tree (Spec §10): by the places whose object events start their battle ("Ohne Ort" last) or by class. A place opens
 * when it holds the selection, every place with a hit while searching (name, class or number). `+` asks for class and name and makes a
 * trainer at the next number, which is then selected. The placeholder of the sight script slot is listed locked. In Erzählen „Neuer
 * Trainer…“ takes the place of `+`: the recipe „Neuer Trainer hier“ (in the world), at the place of the chosen trainer's first figure, else
 * at the place the world shows or showed last.
 */
export function TrainerList() {
  const { t } = useTranslation("data");
  const { rows, selected, select, labelOf, project, bus, onEdited, dirty, scriptFiles, problems, openWizard, worldHeader } = useDataArea("pokemon");
  // The depth of this area, not of the one in front (the list of a deck kept behind keeps its head).
  const narrating = !depthAtLeast(viewDepth("pokemon", useShell().state.depth), "bauen");
  const marks = useFindingMarks(problems, "trainer");
  const { shown, query, setQuery } = useDataSearch("pokemon");
  const [mode, setMode] = useState<GroupMode>("place");
  // Groups the user opened or closed; the others are open when they hold the selection.
  const [toggled, setToggled] = useState<Readonly<Record<string, boolean>>>({});
  const [creating, setCreating] = useState(false);
  const [created, setCreated] = useState<{ kind: "success" | "warning"; text: string } | null>(null);
  // „Gelöschte zeigen“: off, the tombstones are left out.
  const [showDeleted, setShowDeleted] = useState(false);
  const host = useRef<HTMLDivElement>(null);
  useRovingList(host);

  const searching = query.trim() !== "";
  const groups = useMemo(() => groupRows(shown, mode, showDeleted), [shown, mode, showDeleted]);
  const unsaved = useMemo(() => new Set(dirty.filter((a) => a.kind === "Trainer").map((a) => a.index)), [dirty]);
  const isOpen = (g: RowGroup) => searching || (toggled[g.key] ?? g.rows.some((r) => r.index === selected));
  const toggle = (g: RowGroup) => setToggled((all) => ({ ...all, [g.key]: !isOpen(g) }));

  const family = project?.family ?? "Pt";
  const next = nextTrainer(rows, family);
  const battleFile = TRAINER_SCRIPTS[family]?.battleFile ?? null;
  const selectedRow = rows.find((r) => r.index === selected);
  const defaultClass = (selectedRow?.extra as { class?: number } | null)?.class ?? 0;
  /** „Neuer Trainer…“: the recipe where the chosen trainer stands (its first figure's place), else where the world is. */
  const newTrainerHere = () => {
    const place = (selectedRow?.extra as TrainerRowExtra | null)?.places?.[0]?.header ?? worldHeader();
    const request = recipeRequest("new_trainer_here", place);
    if (request !== null) openWizard?.(request);
  };
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
    // The members are written at once: the saved project differs now (Technik › Änderungen).
    bus.emit({ topic: "savedFiles" });
    if (id !== null) select(id);
  };
  const onCreated = (report: CreateTrainerReport) => {
    const made = report.placeholder === null ? t("trainerList.created", { id: report.id }) : t("trainerList.createdPlaceholder", { id: report.id, slot: report.placeholder });
    const extended = battleFile === null ? t("trainerList.scriptExtended", { made }) : t("trainerList.scriptFileExtended", { made, file: String(battleFile).padStart(4, "0") });
    written(report.id, report.script_extended, { kind: "success", text: report.script_extended ? extended : made });
  };
  const onPartial = (message: string) => {
    const partial = partialTrainer(message);
    const id = partial.written.includes(next.id) ? next.id : (partial.written[partial.written.length - 1] ?? null);
    written(id, partial.extended, { kind: "warning", text: partial.text });
  };

  return (
    <div className={styles.column}>
      <div className={styles.top}>
        <TextField type="search" label={t("trainerList.search")} placeholder={t("trainerList.searchPlaceholder")} value={query} onChange={setQuery} onCommit={setQuery} />
        <div className={styles.topRow}>
          <Segmented ariaLabel={t("trainerList.groupBy")} size="s" options={groupOptions()} value={mode} onChange={setMode} />
          {!narrating && <IconButton label={t("trainerList.createTrainer")} icon="plus" onClick={() => setCreating(true)} />}
        </div>
        {narrating && (
          // A row of its own: next to „Nach Ort · Nach Klasse“ the labelled key does not fit the default list width.
          <div className={styles.recipeRow} data-new-trainer-row="">
            <Button kind="secondary" size="s" icon="plus" disabled={openWizard === null} onClick={newTrainerHere} data-new-trainer-recipe="">
              {t("trainerList.newTrainer")}
            </Button>
          </div>
        )}
        <ShowDeleted checked={showDeleted} onChange={setShowDeleted} />
        {created !== null && <Notice kind={created.kind}>{created.text}</Notice>}
      </div>
      {groups.length === 0 && searching ? (
        <EmptyState what={t("trainerList.nothingFound", { query: query.trim() })} why={t("trainerList.searchHint")} />
      ) : (
        <div ref={host} role="tree" aria-label={t("trainerList.list")} className={styles.host}>
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
                    <TrainerRow key={row.index} row={row} label={labelOf(row)} selected={row.index === selected} unsaved={unsaved.has(row.index)} mark={marks.get(row.index)} onSelect={select} />
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
