import { memo, useCallback, useEffect, useMemo, useRef, useState, useSyncExternalStore } from "react";
import { useTranslation } from "react-i18next";
import { t } from "../../i18n";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { ListPane } from "../../ui/frame/ListPane";
import { Badge, EmptyState, Icon, IconButton, TextField, TreeRow, useRovingList } from "../../ui/primitives";
import type { ProblemRow } from "../problems/problems";
import { textArchiveOf } from "../problems/problems";
import { unnamedPlace } from "../places";
import { id4, termLabel, termPlural } from "../terms";
import { DialogList } from "./DialogList";
import { useTextArea } from "./TextAreaContext";
import { filterArchives } from "./textModel";
import type { ArchiveRow } from "./textModel";
import styles from "./TextList.module.css";

type Group = "orte" | "spielweit";

/** The word of a group: the places, and the archives the whole game uses. */
const groupLabel = (group: Group) => (group === "orte" ? termPlural("place") : t("workbench:textList.gameWide"));

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

/** „3 Texte zu lang, in 2 geprüften Archiven“. */
function overlongSum(counts: ReadonlyMap<number, number>): string {
  let texts = 0;
  for (const n of counts.values()) texts += n;
  const archives = counts.size;
  return t("workbench:textList.overlongSum", { texts: t("workbench:textList.overlong", { count: texts }), archives: t("workbench:textList.checkedArchives", { count: archives }) });
}

/** The name an archive row shows: its place, else what it is. */
const rowLabel = (row: ArchiveRow) => row.place ?? (row.users.length > 0 ? unnamedPlace() : termLabel("textArchive"));

/** One archive of the list. Memoised: a selection or filter change renders the rows whose props changed, not all of them. */
const ArchiveItem = memo(function ArchiveItem({ row, selected, count, overlong, onSelect }: { row: ArchiveRow; selected: boolean; count: number | null; overlong: number; onSelect: (index: number) => void }) {
  const { t } = useTranslation("workbench");
  const label = rowLabel(row);
  const name = `${termLabel("textArchive")} ${id4(row.index)}`;
  // Only the open archive's message count is known: reading all of them to list it would decode every archive. The row shows the bare
  // number (the name keeps its room), its hint and the accessible name say what it counts.
  const messages = count === null ? null : `${count} ${count === 1 ? termLabel("message") : termPlural("message")}`;
  const trailing =
    count !== null || overlong > 0 ? (
      <>
        {count !== null && (
          <span className={styles.count} title={messages ?? undefined}>
            {count}
          </span>
        )}
        {overlong > 0 && (
          <Badge kind="warning" title={t("textList.overlong", { count: overlong })}>
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
  const { t } = useTranslation("workbench");
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

  const actions = <IconButton label={t("textList.create")} icon="plus" disabled={project === null || archiveCount === undefined} onClick={createArchive} />;
  return (
    <ListPane title={termPlural("textArchive")} subtitle={project ? t("textList.archiveCount", { count: rows.length }) : undefined} actions={actions} open={open} onToggle={onToggle}>
      {project === null ? (
        <EmptyState what={t("textList.noProject.what")} why={t("textList.noProject.why")} />
      ) : (
        <div className={styles.column}>
          <div className={styles.top}>
            <TextField type="search" label={t("textList.filter.label")} placeholder={t("textList.filter.placeholder")} value={filter} onChange={setFilter} onCommit={setFilter} />
          </div>
          <div ref={tree} role="tree" aria-label={termPlural("textArchive")} className={styles.host}>
            {shown.length === 0 && filter.trim() !== "" && <p className={styles.note}>{t("textList.noMatch")}</p>}
            {(["orte", "spielweit"] as const).map((group) => {
              const list = groups[group];
              if (list.length === 0) return null;
              const expanded = filter.trim() !== "" || !closed.has(group);
              return (
                <div key={group} role="group" aria-label={groupLabel(group)}>
                  <TreeRow
                    depth={0}
                    label={groupLabel(group)}
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
