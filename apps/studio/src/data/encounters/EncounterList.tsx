import { memo, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { createEncounterFile, errorText } from "../../api";
import type { DataRowDto } from "../../api";
import { restoreEntry, useDelete } from "../../delete/DeleteContext";
import { ShowDeleted } from "../../delete/ShowDeleted";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { FindingMark, useFindingMarks } from "../../checks/FindingMark";
import type { MarkSeverity } from "../../checks/FindingMark";
import { Badge, Button, Dialog, EmptyState, EntityStateBadge, IconButton, ListRow, Notice, Picker, Segmented, TextField, useRovingList } from "../../ui/primitives";
import type { PickerItem, SegmentedOption } from "../../ui/primitives";
import { id4 } from "../../workbench/terms";
import { lockReason } from "../dataModel";
import { useDataArea, useDataSearch } from "../useDataArea";
import { groupFiles, rowUsers, usedByText } from "./encounterModel";
import styles from "./Encounter.module.css";

/**
 * One file: the places that use it, its number and how many use it; what the checks found, the unsaved dot, a lock; a tombstone with the
 * badge „gelöscht“ and „Wiederherstellen“ before the reset in its context menu.
 */
const FileRow = memo(function FileRow({
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
  const users = rowUsers(row).length;
  const reset = useReset();
  const { restore } = useDelete();
  const deleted = row.deleted === true;
  const resetItem = resetEntry({ kind: "encounter", index: row.index }, reset.status, reset.open);
  return (
    <ListRow
      contextMenu={deleted ? [restoreEntry({ kind: "encounter", index: row.index }, restore), resetItem] : [resetItem]}
      label={label}
      tech={row.tech}
      subtitle={reason ?? (users === 0 ? t("encounterList.usedByNone") : usedByText(users))}
      selected={selected}
      onSelect={() => onSelect(row.index)}
      title={reason ?? undefined}
      dataAttrs={{ "data-encounter": row.index, "data-deleted": deleted ? "true" : undefined }}
      trailing={
        unsaved || reason !== null || mark || deleted ? (
          <span className={styles.rowEnd}>
            {deleted && <EntityStateBadge state="deleted" />}
            {mark && <FindingMark severity={mark} />}
            {unsaved && <span className={styles.dot} role="img" aria-label={t("unsaved")} />}
            {reason !== null && <Badge kind="locked" title={reason} />}
          </span>
        ) : undefined
      }
    />
  );
});

type Source = "empty" | "copy";

/**
 * The question before a file is made: empty or a copy of a file chosen in a picker. Making it is written at once and cannot be undone
 * (badge); a refusal of the backend (HeartGold's 255 files) shows in the dialog.
 */
function CreateDialog({ next, onClose, onCreated }: { next: number; onClose: () => void; onCreated: (index: number) => void }) {
  const { t } = useTranslation("data");
  const { rows, labelOf } = useDataArea("pokemon");
  const sources: readonly SegmentedOption<Source>[] = [
    { value: "empty", label: t("encounterList.sourceEmpty") },
    { value: "copy", label: t("encounterList.sourceCopy") },
  ];
  const [source, setSource] = useState<Source>("empty");
  const [copyOf, setCopyOf] = useState<number | null>(null);
  const [picking, setPicking] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const items = useMemo<PickerItem[]>(
    () => (picking ? rows.filter((r) => r.locked === null).map((r) => ({ id: r.index, label: labelOf(r), tech: r.tech, area: "pokemon" as const, description: usedByText(rowUsers(r).length) })) : []),
    [picking, rows, labelOf],
  );
  const template = copyOf === null ? null : rows.find((r) => r.index === copyOf);
  const create = async () => {
    if (busy || (source === "copy" && copyOf === null)) return;
    setBusy(true);
    try {
      onCreated(await createEncounterFile(source === "copy" ? copyOf : null));
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
      title={t("encounterList.createTitle", { id: id4(next) })}
      description={t("encounterList.createDescription")}
      actions={[
        { label: t("trainerList.cancel"), onClick: onClose },
        { label: t("trainerList.create"), kind: "primary", onClick: () => void create() },
      ]}
    >
      <div className={styles.dialogBody}>
        <span>
          <Badge kind="irreversible" />
        </span>
        <Segmented ariaLabel={t("encounterList.sourceLabel")} size="s" options={sources} value={source} onChange={setSource} />
        {source === "copy" && (
          <span>
            <Button kind="secondary" size="s" aria-label={t("encounterList.templateAria", { current: template ? labelOf(template) : t("encounterList.templateNone") })} onClick={() => setPicking(true)}>
              {template ? `${labelOf(template)} · ${template.tech}` : t("encounterList.chooseTemplate")}
            </Button>
          </span>
        )}
        {refusal !== null && <Notice kind="error">{refusal}</Notice>}
      </div>
      <Picker open={picking} onOpenChange={setPicking} title={t("encounterList.pickTemplate")} items={items} placeholder={t("encounterList.pickPlaceholder")} onPick={(item) => setCopyOf(Number(item.id))} />
    </Dialog>
  );
}

/**
 * The encounter files (Spec §9): each named by the places of the headers that use it, with its number and how many places use it; the
 * files nobody uses (and the locked ones) in the group „Ohne Ort“ below. `+` makes a file (empty or a copy) at the next number, which is
 * then selected.
 */
export function EncounterList() {
  const { t } = useTranslation("data");
  const { rows, selected, select, labelOf, bus, onEdited, dirty, problems } = useDataArea("pokemon");
  // The rules of the files are the checks `encounter.*`: a file they found an error or warning in is marked.
  const marks = useFindingMarks(problems, "encounter");
  const { shown, query, setQuery } = useDataSearch("pokemon");
  const [creating, setCreating] = useState(false);
  const [created, setCreated] = useState<string | null>(null);
  // „Gelöschte zeigen“: off, the tombstones are left out.
  const [showDeleted, setShowDeleted] = useState(false);
  const host = useRef<HTMLDivElement>(null);
  useRovingList(host);

  const { placed, unplaced } = useMemo(() => groupFiles(shown, showDeleted), [shown, showDeleted]);
  const unsaved = useMemo(() => new Set(dirty.filter((a) => a.kind === "Encounter").map((a) => a.index)), [dirty]);
  const next = rows.reduce((max, r) => Math.max(max, r.index + 1), 0);
  const onCreated = (index: number) => {
    setCreating(false);
    setCreated(t("encounterList.created", { id: id4(index) }));
    // The list reads its files again; the file is on disk, no undo step.
    bus.emit({ topic: "dataChanged", kind: "encounter", index: null });
    onEdited();
    select(index);
  };
  const item = (row: DataRowDto) => <FileRow key={row.index} row={row} label={labelOf(row)} selected={row.index === selected} unsaved={unsaved.has(row.index)} mark={marks.get(row.index)} onSelect={select} />;

  return (
    <div className={styles.column}>
      <div className={styles.top}>
        <TextField type="search" label={t("encounterList.search")} placeholder={t("encounterList.placePlaceholder")} value={query} onChange={setQuery} onCommit={setQuery} />
        <div className={styles.topRow}>
          <span className={styles.note}>{t("encounterList.sortNote")}</span>
          <IconButton label={t("encounterList.createFile")} icon="plus" onClick={() => setCreating(true)} />
        </div>
        <ShowDeleted checked={showDeleted} onChange={setShowDeleted} />
        {created !== null && <Notice kind="success">{created}</Notice>}
      </div>
      {shown.length === 0 && query.trim() !== "" ? (
        <EmptyState what={t("dataList.nothingFound", { query: query.trim() })} why={t("encounterList.searchHint")} />
      ) : (
        <div ref={host} role="listbox" aria-label={t("encounterList.list")} className={styles.host}>
          {placed.map(item)}
          {unplaced.length > 0 && (
            <div role="group" aria-label={t("encounterList.noPlace")} className={styles.group}>
              <div role="presentation" className={styles.groupHead}>
                {t("encounterList.noPlace")} <span className={styles.count}>{unplaced.length}</span>
              </div>
              {unplaced.map(item)}
            </div>
          )}
        </div>
      )}
      {creating && <CreateDialog next={next} onClose={() => setCreating(false)} onCreated={onCreated} />}
    </div>
  );
}
