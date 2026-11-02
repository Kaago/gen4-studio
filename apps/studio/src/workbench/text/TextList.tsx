import { memo, useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { ListPane } from "../../ui/frame/ListPane";
import { Badge, EmptyState, Icon, IconButton, TextField, TreeRow, useRovingList } from "../../ui/primitives";
import type { ProblemRow } from "../problems/problems";
import { textArchiveOf } from "../problems/problems";
import { TERMS, id4 } from "../terms";
import { DialogList } from "./DialogList";
import { useTextArea } from "./TextAreaContext";
import { filterArchives } from "./textModel";
import type { ArchiveRow } from "./textModel";
import styles from "./TextList.module.css";

type Group = "orte" | "spielweit";

const GROUP_LABEL: Record<Group, string> = { orte: "Orte", spielweit: "Spielweit" };

/** The overlong texts the problems name, per archive: only archives a save checked are in it. */
function overlongByArchive(problems: readonly ProblemRow[]): Map<number, number> {
  const counts = new Map<number, number>();
  for (const p of problems) {
    if (p.source !== "text") continue;
    const archive = textArchiveOf(p.file);
    if (archive !== null) counts.set(archive, (counts.get(archive) ?? 0) + 1);
  }
  return counts;
}

/** "3 Texte zu lang, in 2 geprüften Archiven". */
function overlongSum(counts: ReadonlyMap<number, number>): string {
  let texts = 0;
  for (const n of counts.values()) texts += n;
  const archives = counts.size;
  return `${texts} ${texts === 1 ? "Text" : "Texte"} zu lang, in ${archives} geprüften ${archives === 1 ? "Archiv" : "Archiven"}`;
}

/** The name an archive row shows: its place, else what it is. */
const rowLabel = (row: ArchiveRow) => row.place ?? (row.users.length > 0 ? "Ohne Ortsnamen" : TERMS.textArchive.label);

/** One archive of the list. Memoised: a selection or filter change renders the rows whose props changed, not all of them. */
const ArchiveItem = memo(function ArchiveItem({ row, selected, count, overlong, onSelect }: { row: ArchiveRow; selected: boolean; count: number | null; overlong: number; onSelect: (index: number) => void }) {
  const label = rowLabel(row);
  const name = `${TERMS.textArchive.label} ${id4(row.index)}`;
  // Only the open archive's message count is known: reading all of them to list it would decode every archive. The row shows the bare
  // number (the name keeps its room), its hint and the accessible name say what it counts.
  const messages = count === null ? null : `${count} ${count === 1 ? TERMS.message.label : TERMS.message.plural}`;
  const trailing =
    count !== null || overlong > 0 ? (
      <>
        {count !== null && (
          <span className={styles.count} title={messages ?? undefined}>
            {count}
          </span>
        )}
        {overlong > 0 && (
          <Badge kind="warning" title={`${overlong} ${overlong === 1 ? "Text" : "Texte"} zu lang`}>
            {overlong}
          </Badge>
        )}
      </>
    ) : undefined;
  const reset = useReset();
  return (
    <TreeRow
      depth={1}
      label={label}
      tech={id4(row.index)}
      contextMenu={[resetEntry({ kind: "text_archive", index: row.index }, reset.status, reset.open)]}
      ariaLabel={[row.place !== null || row.users.length > 0 ? `${label}, ${name}` : name, messages].filter(Boolean).join(", ")}
      selected={selected}
      onSelect={() => onSelect(row.index)}
      trailing={trailing}
      dataAttrs={{ "data-archive": row.index }}
    />
  );
});

/** The list of the texts: in Erzählen the dialogs by place (`DialogList`), from Bauen on the text archives. */
export function TextList({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  return useTextArea().narrating ? <DialogList open={open} onToggle={onToggle} /> : <ArchiveList open={open} onToggle={onToggle} />;
}

/**
 * The list of the archives (Bauen, Tüfteln): the archives in two groups, *Orte* (an archive a header uses, named after the place of its first header)
 * and *Spielweit* (all others), a filter by place or number, `+` for a new archive, and the overlong texts the problems know of.
 */
function ArchiveList({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  const { project, rows, archive, loaded, archiveCount, problems, select, createArchive } = useTextArea();
  const [filter, setFilter] = useState("");
  const [closed, setClosed] = useState<ReadonlySet<Group>>(() => new Set());
  const tree = useRef<HTMLDivElement>(null);
  useRovingList(tree);
  const problemRows = useSyncExternalStore(problems.subscribe, problems.list);
  const overlong = useMemo(() => overlongByArchive(problemRows), [problemRows]);
  const shown = useMemo(() => filterArchives(rows, filter), [rows, filter]);
  const groups = useMemo(() => {
    const byGroup: Record<Group, ArchiveRow[]> = { orte: [], spielweit: [] };
    for (const row of shown) byGroup[row.users.length > 0 ? "orte" : "spielweit"].push(row);
    return byGroup;
  }, [shown]);
  const selectArchive = useCallback((index: number) => select(index), [select]);
  const toggle = (group: Group) =>
    setClosed((before) => {
      const next = new Set(before);
      if (!next.delete(group)) next.add(group);
      return next;
    });

  // The archive that opens (a click, a link, a new one) comes into view: a filter that hides it is cleared and its group opened first.
  const revealed = useRef<number | null>(null);
  useEffect(() => {
    if (archive === null) {
      revealed.current = null;
      return;
    }
    if (revealed.current === archive) return;
    const row = rows[archive];
    if (!row) return;
    if (filterArchives([row], filter).length === 0) {
      setFilter("");
      return;
    }
    const group: Group = row.users.length > 0 ? "orte" : "spielweit";
    if (filter.trim() === "" && closed.has(group)) {
      toggle(group);
      return;
    }
    const element = tree.current?.querySelector(`[data-archive="${archive}"]`);
    if (element) {
      revealed.current = archive;
      element.scrollIntoView({ block: "nearest" });
    }
  });

  const actions = <IconButton label="Textarchiv anlegen" icon="plus" disabled={project === null || archiveCount === undefined} onClick={createArchive} />;
  return (
    <ListPane title="Textarchive" subtitle={project ? `${rows.length} Archive` : undefined} actions={actions} open={open} onToggle={onToggle}>
      {project === null ? (
        <EmptyState what="Kein Projekt geöffnet." why="Öffne ein Projekt, dann stehen hier seine Textarchive." />
      ) : (
        <div className={styles.column}>
          <div className={styles.top}>
            <TextField type="search" label="Textarchive filtern" placeholder="Ort oder Archivnummer" value={filter} onChange={setFilter} onCommit={setFilter} />
          </div>
          <div ref={tree} role="tree" aria-label="Textarchive" className={styles.host}>
            {shown.length === 0 && filter.trim() !== "" && <p className={styles.note}>Kein Archiv passt zum Filter.</p>}
            {(["orte", "spielweit"] as const).map((group) => {
              const list = groups[group];
              if (list.length === 0) return null;
              const expanded = filter.trim() !== "" || !closed.has(group);
              return (
                <div key={group} role="group" aria-label={GROUP_LABEL[group]}>
                  <TreeRow
                    depth={0}
                    label={GROUP_LABEL[group]}
                    selected={false}
                    expanded={expanded}
                    onToggle={() => toggle(group)}
                    onSelect={() => toggle(group)}
                    trailing={<span className={styles.count}>{list.length}</span>}
                    dataAttrs={{ "data-text-group": group }}
                  />
                  {expanded &&
                    list.map((row) => (
                      <ArchiveItem
                        key={row.index}
                        row={row}
                        selected={row.index === archive}
                        count={row.index === archive && loaded ? loaded.messages.length : null}
                        overlong={overlong.get(row.index) ?? 0}
                        onSelect={selectArchive}
                      />
                    ))}
                </div>
              );
            })}
          </div>
          {overlong.size > 0 && (
            <p className={styles.sum} role="note">
              <Icon name="warning" weight="fill" />
              {overlongSum(overlong)}
            </p>
          )}
        </div>
      )}
    </ListPane>
  );
}
