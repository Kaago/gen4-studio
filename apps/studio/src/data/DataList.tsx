import { memo, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import type { DataRowDto } from "../api";
import { Button, EmptyState, ListRow, Sprite, TextField } from "../ui/primitives";
import type { RowDataAttrs } from "../ui/primitives";
import { useVisible } from "../workbench/useVisible";
import { filterText, isFiltered } from "./dataModel";
import type { DataArea } from "./dataModel";
import { useIcon } from "./icons";
import { useDataArea } from "./useDataArea";
import styles from "./DataArea.module.css";

/** How a list shows its rows: the attribute that names a row's member, and what stands before and after its name. */
export interface DataListRows {
  /** `data-species`, `data-move`, `data-item`: tests and scripts find a row by it. */
  attr: `data-${string}`;
  leading?: (row: DataRowDto, label: string) => ReactNode;
  trailing?: (row: DataRowDto) => ReactNode;
}

export interface DataListProps extends DataListRows {
  area: DataArea;
  /** What the list holds ("Pokédex", "Attacken", "Gegenstände"): the list's name and its search's ("Pokédex durchsuchen"). */
  label: string;
  /** The filters of the part (chips) between the search and the rows. */
  filters?: ReactNode;
}

/** One row. Memoised: a selection or search renders the rows whose props changed, not all of them. */
const DataRow = memo(function DataRow({ row, pos, label, selected, onSelect, rows }: { row: DataRowDto; pos: number; label: string; selected: boolean; onSelect: (index: number) => void; rows: DataListRows }) {
  // `data-pos`: the row's place in the shown rows, for the windowed list's keys.
  const attrs = { [rows.attr]: row.index, "data-pos": pos } as RowDataAttrs;
  return (
    <ListRow
      label={label}
      tech={row.tech}
      leading={rows.leading?.(row, label)}
      trailing={rows.trailing?.(row)}
      selected={selected}
      onSelect={() => onSelect(row.index)}
      dataAttrs={attrs}
      ariaLabel={`${label}, ${row.tech}`}
    />
  );
});

/**
 * The list of a data part (the mount point of the Pokédex, moves and items lists): a search over name and number, the part's filters, the
 * filter in words with "Filter lösen", and the rows. The selection comes into view when it is made elsewhere (a link).
 */
export function DataList({ area, label, filters, attr, leading, trailing }: DataListProps) {
  const { shown, selected, select, query, setQuery, filter, clearFilter, labelOf } = useDataArea(area);
  // The callbacks of the rows; a new object only when the list's look changes, so the memoised rows keep still otherwise.
  const rows = useRowsSpec(attr, leading, trailing);
  const onSelect = useCallback((index: number) => select(index), [select]);
  const filtered = filterText(filter);
  const searching = query.trim() !== "" || isFiltered(filter);
  return (
    <div className={styles.column}>
      <div className={styles.top}>
        <TextField type="search" label={`${label} durchsuchen`} placeholder="Name oder Nummer" value={query} onChange={setQuery} onCommit={setQuery} />
        {filters}
        {filtered !== null && (
          <p className={styles.filterLine}>
            <span>{`${filtered}: ${shown.length}`}</span>
            <Button kind="quiet" size="s" onClick={clearFilter}>
              Filter lösen
            </Button>
          </p>
        )}
      </div>
      {shown.length === 0 && searching ? (
        <EmptyState what={`Nichts gefunden für „${query.trim() || filtered}“`} why="Ändere die Suche oder löse den Filter." action={filtered === null ? { label: "Filter lösen", onClick: clearFilter } : undefined} />
      ) : (
        <WindowedRows label={label} shown={shown} selected={selected} onSelect={onSelect} labelOf={labelOf} rows={rows} />
      )}
    </div>
  );
}

/** The height of a row while none could be measured (jsdom has no layout). */
const ROW_FALLBACK = 32;
/** How many rows the view holds while its height cannot be measured. */
const VIEW_FALLBACK_ROWS = 24;
/** Rows rendered beyond the view, half above and half below it. */
const BUFFER = 20;

/**
 * The rows as a listbox of which only the rows in view and `BUFFER` more are rendered (a list of 537 rows took 40 to 90 ms to render
 * whole, see the report of Task 13; windowing over `scrollTop` like the texts' `TextTable`, rows of one height). One row holds the tab
 * stop (the focused one, else the selected one, else the first); the arrow keys, Home, End and the page keys move the focus through all
 * rows, rendering the one they reach; Enter and Space select. A selection made elsewhere (a link, a cleared search) comes into view.
 */
function WindowedRows({ label, shown, selected, onSelect, labelOf, rows }: { label: string; shown: readonly DataRowDto[]; selected: number | null; onSelect: (index: number) => void; labelOf: (row: DataRowDto) => string; rows: DataListRows }) {
  const body = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewHeight, setViewHeight] = useState(0);
  const [height, setHeight] = useState(ROW_FALLBACK);
  // The position (in `shown`) of the row that holds the tab stop; null: the selected one, else the first.
  const [active, setActive] = useState<number | null>(null);
  // The position that takes the focus after the render that shows it (a key moved it).
  const focusAfter = useRef<number | null>(null);

  const measure = useCallback(() => {
    const el = body.current;
    if (!el) return;
    const row = el.querySelector<HTMLElement>("[data-pos]");
    if (row && row.offsetHeight > 0) setHeight(row.offsetHeight);
    setViewHeight(el.clientHeight);
  }, []);
  useLayoutEffect(() => {
    if (typeof ResizeObserver === "undefined" || !body.current) return;
    const watch = new ResizeObserver(measure);
    watch.observe(body.current);
    return () => watch.disconnect();
  }, [measure]);
  // After every render: cheap, state only changes when a size did (the density changes the row height).
  useLayoutEffect(measure);

  const count = shown.length;
  const selectedPos = useMemo(() => (selected === null ? -1 : shown.findIndex((r) => r.index === selected)), [shown, selected]);
  const tabStop = active !== null && active < count ? active : selectedPos >= 0 ? selectedPos : 0;
  const view = viewHeight > 0 ? viewHeight : VIEW_FALLBACK_ROWS * height;
  const inView = Math.ceil(view / height);
  const first = Math.min(Math.floor(scrollTop / height), Math.max(0, count - 1));
  const start = Math.max(0, first - BUFFER / 2);
  const end = Math.min(count, first + inView + BUFFER / 2);

  /** Scrolls the least distance that shows the row at `pos` in full. */
  function reveal(pos: number) {
    const el = body.current;
    if (!el) return;
    const top = pos * height;
    let next = el.scrollTop;
    if (top < next) next = top;
    else if (top + height > next + view) next = top + height - view;
    if (next !== el.scrollTop) el.scrollTop = next;
    setScrollTop(next);
  }

  // A selection made elsewhere comes into view once (not when the user scrolls away from it later).
  const revealed = useRef<number | null>(null);
  useEffect(() => {
    if (selected === null || selectedPos < 0) {
      revealed.current = null;
      return;
    }
    if (revealed.current === selected) return;
    revealed.current = selected;
    reveal(selectedPos);
  }, [selected, selectedPos]);

  // The tab stop: one rendered row; the others leave the tab order.
  useLayoutEffect(() => {
    const el = body.current;
    if (!el) return;
    const stop = tabStop >= start && tabStop < end ? tabStop : start;
    for (const row of el.querySelectorAll<HTMLElement>("[data-pos]")) row.tabIndex = Number(row.dataset.pos) === stop ? 0 : -1;
    if (focusAfter.current !== null) {
      el.querySelector<HTMLElement>(`[data-pos="${focusAfter.current}"]`)?.focus();
      focusAfter.current = null;
    }
  });

  function onKeyDown(event: KeyboardEvent<HTMLElement>) {
    const target = event.target as HTMLElement;
    if (target.dataset.pos === undefined || event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    const pos = Number(target.dataset.pos);
    const page = Math.max(1, inView - 1);
    const moves: Record<string, number> = { ArrowDown: pos + 1, ArrowUp: pos - 1, Home: 0, End: count - 1, PageDown: pos + page, PageUp: pos - page };
    const to = moves[event.key];
    if (to === undefined) return;
    event.preventDefault();
    const next = Math.min(Math.max(0, to), count - 1);
    reveal(next);
    setActive(next);
    focusAfter.current = next;
  }

  const items: ReactNode[] = [];
  for (let pos = start; pos < end; pos++) {
    const row = shown[pos]!;
    items.push(<DataRow key={row.index} row={row} pos={pos} label={labelOf(row)} selected={row.index === selected} onSelect={onSelect} rows={rows} />);
  }
  return (
    <div
      ref={body}
      role="listbox"
      aria-label={label}
      className={styles.host}
      onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}
      onKeyDown={onKeyDown}
      onFocus={(e) => {
        const pos = (e.target as HTMLElement).dataset.pos;
        if (pos !== undefined) setActive(Number(pos));
      }}
    >
      <div aria-hidden="true" style={{ height: start * height } /* dyn */} />
      {items}
      <div aria-hidden="true" style={{ height: (count - end) * height } /* dyn */} />
    </div>
  );
}

/** The rows' spec as one object that keeps its identity while its parts do. */
function useRowsSpec(attr: `data-${string}`, leading: DataListRows["leading"], trailing: DataListRows["trailing"]): DataListRows {
  const spec = useRef<DataListRows>({ attr, leading, trailing });
  if (spec.current.attr !== attr || spec.current.leading !== leading || spec.current.trailing !== trailing) spec.current = { attr, leading, trailing };
  return spec.current;
}

/**
 * The icon of a row, loaded once the row comes near the view (`useVisible`); the first letter of the name until then, and for a member
 * without an icon. Decorative: the row names the member.
 */
export function RowIcon({ iconKey, load, label, area }: { iconKey: string | null; load: () => Promise<string | null>; label: string; area: "pokemon" | "gegenstaende" }) {
  const box = useRef<HTMLSpanElement>(null);
  const visible = useVisible(box, iconKey !== null);
  const url = useIcon(iconKey, load, visible);
  return (
    <span ref={box} className={styles.icon}>
      <Sprite url={url} label={label} size={24} area={area} decorative />
    </span>
  );
}
