import { memo, useMemo, useRef, useState } from "react";
import { createEncounterFile, errorText } from "../../api";
import type { DataRowDto } from "../../api";
import { Badge, Button, Dialog, EmptyState, IconButton, ListRow, Notice, Picker, Segmented, TextField, useRovingList } from "../../ui/primitives";
import type { PickerItem, SegmentedOption } from "../../ui/primitives";
import { id4 } from "../../workbench/terms";
import { lockReason } from "../dataModel";
import { useDataArea, useDataSearch } from "../useDataArea";
import { groupFiles, rowUsers, usedByText } from "./encounterModel";
import styles from "./Encounter.module.css";

/** One file: the places that use it, its number and how many use it; the unsaved dot, a lock. */
const FileRow = memo(function FileRow({ row, label, selected, unsaved, onSelect }: { row: DataRowDto; label: string; selected: boolean; unsaved: boolean; onSelect: (index: number) => void }) {
  const reason = row.locked === null ? null : lockReason(row.locked);
  const users = rowUsers(row).length;
  return (
    <ListRow
      label={label}
      tech={row.tech}
      subtitle={reason ?? (users === 0 ? "von keinem Ort verwendet" : usedByText(users))}
      selected={selected}
      onSelect={() => onSelect(row.index)}
      title={reason ?? undefined}
      dataAttrs={{ "data-encounter": row.index }}
      trailing={
        unsaved || reason !== null ? (
          <span className={styles.rowEnd}>
            {unsaved && <span className={styles.dot} role="img" aria-label="ungespeichert" />}
            {reason !== null && <Badge kind="locked" title={reason} />}
          </span>
        ) : undefined
      }
    />
  );
});

type Source = "empty" | "copy";
const SOURCES: readonly SegmentedOption<Source>[] = [
  { value: "empty", label: "Leer" },
  { value: "copy", label: "Kopie von …" },
];

/**
 * The question before a file is made: empty or a copy of a file chosen in a picker. Making it is written at once and cannot be undone
 * (badge); a refusal of the backend (HeartGold's 255 files) shows in the dialog.
 */
function CreateDialog({ next, onClose, onCreated }: { next: number; onClose: () => void; onCreated: (index: number) => void }) {
  const { rows, labelOf } = useDataArea("pokemon");
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
      title={`Begegnungsdatei ${id4(next)} anlegen?`}
      description="Sie wird sofort angelegt und von keinem Ort verwendet; anbinden kann man sie im Header-Editor (Feld „Begegnungen“)."
      actions={[
        { label: "Abbrechen", onClick: onClose },
        { label: "Anlegen", kind: "primary", onClick: () => void create() },
      ]}
    >
      <div className={styles.dialogBody}>
        <span>
          <Badge kind="irreversible" />
        </span>
        <Segmented ariaLabel="Inhalt der neuen Datei" size="s" options={SOURCES} value={source} onChange={setSource} />
        {source === "copy" && (
          <span>
            <Button kind="secondary" size="s" aria-label={`Vorlage wählen, jetzt ${template ? labelOf(template) : "keine"}`} onClick={() => setPicking(true)}>
              {template ? `${labelOf(template)} · ${template.tech}` : "Vorlage wählen …"}
            </Button>
          </span>
        )}
        {refusal !== null && <Notice kind="error">{refusal}</Notice>}
      </div>
      <Picker open={picking} onOpenChange={setPicking} title="Vorlage wählen" items={items} placeholder="Ort oder Nummer suchen" onPick={(item) => setCopyOf(Number(item.id))} />
    </Dialog>
  );
}

/**
 * The encounter files (Spec §9): each named by the places of the headers that use it, with its number and how many places use it; the
 * files nobody uses (and the locked ones) in the group „Ohne Ort“ below. `+` makes a file (empty or a copy) at the next number, which is
 * then selected.
 */
export function EncounterList() {
  const { rows, selected, select, labelOf, bus, onEdited, dirty } = useDataArea("pokemon");
  const { shown, query, setQuery } = useDataSearch("pokemon");
  const [creating, setCreating] = useState(false);
  const [created, setCreated] = useState<string | null>(null);
  const host = useRef<HTMLDivElement>(null);
  useRovingList(host);

  const { placed, unplaced } = useMemo(() => groupFiles(shown), [shown]);
  const unsaved = useMemo(() => new Set(dirty.filter((a) => a.kind === "Encounter").map((a) => a.index)), [dirty]);
  const next = rows.reduce((max, r) => Math.max(max, r.index + 1), 0);
  const onCreated = (index: number) => {
    setCreating(false);
    setCreated(`Begegnungsdatei ${id4(index)} angelegt.`);
    // The list reads its files again; the file is on disk, no undo step.
    bus.emit({ topic: "dataChanged", kind: "encounter", index: null });
    onEdited();
    select(index);
  };
  const item = (row: DataRowDto) => <FileRow key={row.index} row={row} label={labelOf(row)} selected={row.index === selected} unsaved={unsaved.has(row.index)} onSelect={select} />;

  return (
    <div className={styles.column}>
      <div className={styles.top}>
        <TextField type="search" label="Begegnungen durchsuchen" placeholder="Ort oder Nummer" value={query} onChange={setQuery} onCommit={setQuery} />
        <div className={styles.topRow}>
          <span className={styles.note}>Dateien nach den Orten, die sie nutzen</span>
          <IconButton label="Begegnungsdatei anlegen…" icon="plus" onClick={() => setCreating(true)} />
        </div>
        {created !== null && <Notice kind="success">{created}</Notice>}
      </div>
      {shown.length === 0 && query.trim() !== "" ? (
        <EmptyState what={`Nichts gefunden für „${query.trim()}“`} why="Suche nach einem Ort oder einer Nummer." />
      ) : (
        <div ref={host} role="listbox" aria-label="Begegnungen" className={styles.host}>
          {placed.map(item)}
          {unplaced.length > 0 && (
            <div role="group" aria-label="Ohne Ort" className={styles.group}>
              <div role="presentation" className={styles.groupHead}>
                Ohne Ort <span className={styles.count}>{unplaced.length}</span>
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
