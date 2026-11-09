import { memo, useRef } from "react";
import { useTranslation } from "react-i18next";
import type { FlagEntryDto, FlagKind, FlagListDto } from "../api";
import type { LiveValues } from "../app/liveStore";
import { Badge, ListRow, Tooltip, useRovingList } from "../ui/primitives";
import type { RowDataAttrs } from "../ui/primitives";
import { t as translate } from "../i18n";
import { displayName, liveCellText, liveCellTip, numberText, outsideTheSave, usageText } from "./flagWords";
import styles from "./FlagsSurface.module.css";

/** At most this many rows are rendered (a list of 537 rows takes 40 to 90 ms, a full Platinum list has 4 097); search and filters reach the rest. */
export const ROW_LIMIT = 500;

/** Why the cell of the column „Live“ is empty outside play mode. */
export function liveOff(): string {
  return translate("flags:flagList.liveOff");
}

/** What the row says on the right: the places of a map-local number („an 54 Orten“), else how often it is set and checked. */
function usageOf(entry: FlagEntryDto): string {
  if (entry.area === "map_local" && entry.places > 0) return translate("flags:flagList.places", { count: entry.places });
  return usageText(entry.usage);
}

/**
 * The cell „Live“ of a row. `value` undefined: no snapshot, „–“ with `pending` in play mode, else `liveOff()`; null: the snapshot does not hold the number (outside the
 * save), „–“ with `OUTSIDE_THE_SAVE`. A flag is „1“ or „0“ as a button that asks for the other value (a click does not choose the row; the
 * keyboard's way is the detail's switch, so the button is no tab stop of its own); a variable shows its value, changed in the detail.
 */
function LiveCell({
  kind,
  number,
  value,
  pending,
  onToggle,
}: {
  kind: FlagKind;
  number: number;
  value: number | null | undefined;
  /** Play mode before its first snapshot: why the cell waits (the hinge's text). */
  pending?: string;
  onToggle?: (n: number) => void;
}) {
  if (value === undefined || value === null) {
    return (
      <Tooltip content={value === null ? outsideTheSave() : (pending ?? liveOff())}>
        <span data-flag-live="" className={styles.live}>
          –
        </span>
      </Tooltip>
    );
  }
  const text = liveCellText(kind, value);
  const tip = liveCellTip(kind, value);
  if (kind === "var") {
    return (
      <Tooltip content={tip}>
        <span data-flag-live={text} className={styles.live}>
          {text}
        </span>
      </Tooltip>
    );
  }
  return (
    <Tooltip content={tip}>
      <button
        type="button"
        tabIndex={-1}
        aria-label={tip}
        data-flag-live={text}
        className={`${styles.live} ${styles.liveButton}`}
        onClick={(e) => {
          e.stopPropagation();
          onToggle?.(number);
        }}
      >
        {text}
      </button>
    </Tooltip>
  );
}

/**
 * One number: its own name in bold (else the pret name, else kind and number), below it in mono the pret name under an own one and the
 * number in hex and decimal; on the right its usage, „reserviert“ (the recipe as title) or „frei“, and the cell of „Live“. Memoised: a
 * selection renders the two rows it moves between, a new snapshot the rows whose value moved.
 */
const FlagRow = memo(function FlagRow({
  entry,
  kind,
  selected,
  onSelect,
  live,
  livePending,
  onLiveToggle,
}: {
  entry: FlagEntryDto;
  kind: FlagKind;
  selected: boolean;
  onSelect: (n: number) => void;
  /** The live value (a flag 0 or 1); undefined outside play mode, null outside the snapshot. */
  live: number | null | undefined;
  livePending?: string;
  onLiveToggle?: (n: number) => void;
}) {
  const { t } = useTranslation("flags");
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
              <Badge kind="info">{t("flagList.reserved")}</Badge>
            </span>
          )}
          {entry.free && (
            <span data-flag-badge="free" className={styles.badge}>
              <Badge kind="success">{t("flagList.free")}</Badge>
            </span>
          )}
          <LiveCell kind={kind} number={entry.number} value={live} pending={livePending} onToggle={onLiveToggle} />
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
  if (rendered >= matching) return translate("flags:flagList.foot.counted", { matching, total });
  return matching === total ? translate("flags:flagList.foot.cut", { rendered, total }) : translate("flags:flagList.foot.cutFiltered", { rendered, matching, total });
}

export type FlagListProps = {
  /** The list as read (its kind and the counts of the foot). */
  list: FlagListDto;
  /** The entries filter and search let through. */
  entries: readonly FlagEntryDto[];
  selected: number | null;
  /** Keep it stable: the rows are memoised. */
  onSelect: (number: number) => void;
  /** The snapshot's values in play mode (the column „Live“ fills); absent: the column waits for „Hier spielen“. */
  live?: LiveValues | null;
  /** Play mode without a snapshot yet (connecting, waiting for the save, lost): the hinge's text, the tooltip of the empty cells. */
  livePending?: string;
  /** A click on a flag's live value: write the other one (keep it stable, the rows are memoised). */
  onLiveToggle?: (number: number) => void;
};

/** The live value of `entry`: undefined without `live`, null outside the save or the snapshot, a flag as 0 or 1. */
function liveValueOf(live: LiveValues | null | undefined, kind: FlagKind, entry: FlagEntryDto): number | null | undefined {
  if (!live) return undefined;
  if (!entry.in_save) return null;
  if (kind === "var") return live.var(entry.number);
  const set = live.flag(entry.number);
  return set === null ? null : set ? 1 : 0;
}

/**
 * The numbers of a kind as a list with the column „Live“ (the snapshot's values in play mode, its head in the play colour; else empty),
 * „Nichts passt zu Filter und Suche.“ when nothing is left, and the foot: how many of how many (when capped, how many rows show), and what scripts use without a number (script-local
 * variables, flags from variables). The first `ROW_LIMIT` rows show, and the chosen one wherever it is.
 */
export function FlagList({ list, entries, selected, onSelect, live, livePending, onLiveToggle }: FlagListProps) {
  const { t } = useTranslation("flags");
  const host = useRef<HTMLDivElement>(null);
  useRovingList(host);
  const shown = entries.length > ROW_LIMIT ? entries.slice(0, ROW_LIMIT) : entries;
  const chosen = selected === null || shown === entries ? undefined : entries.slice(ROW_LIMIT).find((e) => e.number === selected);
  const rows = chosen === undefined ? shown : [...shown, chosen];
  return (
    <div className={styles.list} data-flag-list="">
      <div className={styles.listHead}>
        <span data-flag-live-head={live || livePending !== undefined ? "on" : ""} className={styles.liveHead}>
          {t("flagList.live")}
        </span>
      </div>
      {entries.length === 0 ? (
        <p className={styles.empty}>{t("flagList.empty")}</p>
      ) : (
        <div ref={host} role="listbox" aria-label={list.kind === "flag" ? t("flagList.listFlags") : t("flagList.listVars")} className={styles.rows}>
          {rows.map((e) => (
            <FlagRow
              key={e.number}
              entry={e}
              kind={list.kind}
              selected={e.number === selected}
              onSelect={onSelect}
              live={liveValueOf(live, list.kind, e)}
              livePending={livePending}
              onLiveToggle={onLiveToggle}
            />
          ))}
        </div>
      )}
      {entries.length > ROW_LIMIT && <p className={styles.note}>{t("flagList.rowLimitNote", { limit: ROW_LIMIT })}</p>}
      <p className={styles.foot} data-flag-list-foot="">
        {t("flagList.foot.summary", {
          shown: shownText(rows.length, entries.length, list.entries.length),
          scriptLocal: list.counts.script_local,
          dynamic: list.counts.dynamic,
        })}
      </p>
    </div>
  );
}
