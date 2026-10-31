import { memo, useRef } from "react";
import type { FlagEntryDto, FlagKind, FlagListDto } from "../api";
import { Badge, ListRow, Tooltip, useRovingList } from "../ui/primitives";
import type { RowDataAttrs } from "../ui/primitives";
import { displayName, numberText, usageText } from "./flagWords";
import styles from "./FlagsSurface.module.css";

/** At most this many rows are rendered (a list of 537 rows takes 40 to 90 ms, a full Platinum list has 4 097); search and filters reach the rest. */
export const ROW_LIMIT = 500;

/** Why the cell of the column „Live“ is empty. */
export const LIVE_LATER = "Kommt mit dem Spielmodus (3d.9)";

const de = (n: number) => n.toLocaleString("de-DE");

/** What the row says on the right: the places of a map-local number („an 54 Orten“), else how often it is set and checked. */
function usageOf(entry: FlagEntryDto): string {
  if (entry.area === "map_local" && entry.places > 0) return `an ${de(entry.places)} ${entry.places === 1 ? "Ort" : "Orten"}`;
  return usageText(entry.usage);
}

/**
 * One number: its own name in bold (else the pret name, else kind and number), below it in mono the pret name under an own one and the
 * number in hex and decimal; on the right its usage, „reserviert“ (the recipe as title) or „frei“, and the empty cell of „Live“. Memoised:
 * a selection renders the two rows it moves between.
 */
const FlagRow = memo(function FlagRow({ entry, kind, selected, onSelect }: { entry: FlagEntryDto; kind: FlagKind; selected: boolean; onSelect: (n: number) => void }) {
  const own = entry.own_id !== null;
  const subtitle = `${own && entry.name !== null ? `${entry.name} · ` : ""}${numberText(entry.number)}`;
  return (
    <ListRow
      label={displayName(entry, kind, entry.number)}
      strong={own}
      subtitle={subtitle}
      techSubtitle
      selected={selected}
      onSelect={() => onSelect(entry.number)}
      dataAttrs={{ "data-flag-row": entry.number } as RowDataAttrs}
      trailing={
        <span className={styles.rowEnd}>
          <span className={styles.usage}>{usageOf(entry)}</span>
          {entry.reserved !== null && (
            <span data-flag-badge="reserved" title={entry.reserved} className={styles.badge}>
              <Badge kind="info">reserviert</Badge>
            </span>
          )}
          {entry.free && (
            <span data-flag-badge="free" className={styles.badge}>
              <Badge kind="success">frei</Badge>
            </span>
          )}
          <Tooltip content={LIVE_LATER}>
            <span data-flag-live="" className={styles.live}>
              –
            </span>
          </Tooltip>
        </span>
      }
    />
  );
});

/**
 * How many rows the foot counts: „2 von 5“ (filter and search let 2 of 5 through); when the list is capped at `ROW_LIMIT`, the rows it
 * renders: „500 von 4.097 gezeigt“, after a filter „500 von 2.000 gezeigt, 4.097 insgesamt“.
 */
function shownText(rendered: number, matching: number, total: number): string {
  if (rendered >= matching) return `${de(matching)} von ${de(total)}`;
  return matching === total ? `${de(rendered)} von ${de(total)} gezeigt` : `${de(rendered)} von ${de(matching)} gezeigt, ${de(total)} insgesamt`;
}

export type FlagListProps = {
  /** The list as read (its kind and the counts of the foot). */
  list: FlagListDto;
  /** The entries filter and search let through. */
  entries: readonly FlagEntryDto[];
  selected: number | null;
  /** Keep it stable: the rows are memoised. */
  onSelect: (number: number) => void;
};

/**
 * The numbers of a kind as a list with the column „Live“ (empty until the play mode, 3d.9), „Nichts passt zu Filter und Suche.“ when
 * nothing is left, and the foot: how many of how many (when capped, how many rows show), and what scripts use without a number (script-local
 * variables, flags from variables). The first `ROW_LIMIT` rows show, and the chosen one wherever it is.
 */
export function FlagList({ list, entries, selected, onSelect }: FlagListProps) {
  const host = useRef<HTMLDivElement>(null);
  useRovingList(host);
  const shown = entries.length > ROW_LIMIT ? entries.slice(0, ROW_LIMIT) : entries;
  const chosen = selected === null || shown === entries ? undefined : entries.slice(ROW_LIMIT).find((e) => e.number === selected);
  const rows = chosen === undefined ? shown : [...shown, chosen];
  return (
    <div className={styles.list} data-flag-list="">
      <div className={styles.listHead}>
        <span data-flag-live-head="" className={styles.liveHead}>
          Live
        </span>
      </div>
      {entries.length === 0 ? (
        <p className={styles.empty}>Nichts passt zu Filter und Suche.</p>
      ) : (
        <div ref={host} role="listbox" aria-label={list.kind === "flag" ? "Flags" : "Variablen"} className={styles.rows}>
          {rows.map((e) => (
            <FlagRow key={e.number} entry={e} kind={list.kind} selected={e.number === selected} onSelect={onSelect} />
          ))}
        </div>
      )}
      {entries.length > ROW_LIMIT && <p className={styles.note}>{`Die ersten ${de(ROW_LIMIT)} Zeilen; Suche und Filter erreichen die übrigen.`}</p>}
      <p className={styles.foot} data-flag-list-foot="">
        {`${shownText(rows.length, entries.length, list.entries.length)} · Script-lokale Variablen ${de(list.counts.script_local)} Vorkommen, Flags aus Variablen ${de(list.counts.dynamic)} (nicht im Index)`}
      </p>
    </div>
  );
}
