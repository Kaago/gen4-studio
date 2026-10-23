import { memo, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import type { RowDataAttrs } from "../../ui/primitives";
import styles from "./TextTable.module.css";

/** The height of a row while none could be measured (jsdom has no layout). */
const ROW_FALLBACK = 28;
/** How many rows the view holds while its height cannot be measured. */
const VIEW_FALLBACK_ROWS = 24;
/** Rows rendered beyond the view, half above and half below it. */
const BUFFER = 20;

export interface TextTableColumn {
  title: string;
}

export interface TextTableProps {
  /** The grid's accessible name. */
  label: string;
  /** The column layout: the messages of an archive (Nr., Text, Längste Zeile) or the hits of all archives (Archiv, Nr., Text). */
  kind: "messages" | "hits";
  columns: readonly TextTableColumn[];
  count: number;
  /** The selected row (it holds the tab stop); null: none, the first row holds it. */
  selected: number | null;
  /** The `data-*` attributes of a row (`data-message`, `data-hit`). */
  rowAttrs: (index: number) => RowDataAttrs;
  /** The cells of a row, one per column. */
  cells: (index: number) => ReactNode[];
  onSelect: (index: number) => void;
  /** Enter on a row. */
  onEnter?: (index: number) => void;
}

/**
 * A table of many rows as a grid: only the rows in view and `BUFFER` more are rendered (its own windowing over `scrollTop`, rows of
 * one height). The selected row holds the tab stop (roving tabindex); the arrow keys, Home, End and the page keys move the selection
 * and the focus, Enter calls `onEnter`. A selection from outside is scrolled into view. Memoised: it renders when its props change.
 */
export const TextTable = memo(function TextTable({ label, kind, columns, count, selected, rowAttrs, cells, onSelect, onEnter }: TextTableProps) {
  const body = useRef<HTMLDivElement>(null);
  const [scrollTop, setScrollTop] = useState(0);
  const [viewHeight, setViewHeight] = useState(0);
  const rowHeight = useRef(ROW_FALLBACK);
  // The row that takes the focus after the render that shows it (a key moved the selection).
  const focusAfter = useRef<number | null>(null);

  // The height of the view, and of a row, as laid out.
  useLayoutEffect(() => {
    const el = body.current;
    if (!el) return;
    const measure = () => {
      const row = el.querySelector<HTMLElement>("[data-row]");
      if (row && row.offsetHeight > 0) rowHeight.current = row.offsetHeight;
      setViewHeight(el.clientHeight);
    };
    measure();
    if (typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, []);

  const height = rowHeight.current;
  const view = viewHeight > 0 ? viewHeight : VIEW_FALLBACK_ROWS * height;
  const inView = Math.ceil(view / height);
  const first = Math.min(Math.floor(scrollTop / height), Math.max(0, count - 1));
  const start = Math.max(0, first - BUFFER / 2);
  const end = Math.min(count, first + inView + BUFFER / 2);
  const active = selected !== null && selected < count ? selected : 0;

  /** Scrolls the least distance that shows row `index` in full. */
  function reveal(index: number) {
    const el = body.current;
    if (!el) return;
    const top = index * height;
    let next = el.scrollTop;
    if (top < next) next = top;
    else if (top + height > next + view) next = top + height - view;
    if (next !== el.scrollTop) el.scrollTop = next;
    setScrollTop(next);
  }

  // A selection made elsewhere (a link, a hit, a new message) comes into view.
  useEffect(() => {
    if (selected !== null && selected < count) reveal(selected);
  }, [selected, count]);

  useLayoutEffect(() => {
    if (focusAfter.current === null) return;
    body.current?.querySelector<HTMLElement>(`[data-row="${focusAfter.current}"]`)?.focus();
    focusAfter.current = null;
  });

  function onKeyDown(event: KeyboardEvent<HTMLElement>, index: number) {
    if (event.target !== event.currentTarget || event.altKey || event.metaKey || event.ctrlKey) return;
    const page = Math.max(1, inView - 1);
    const moves: Record<string, number> = { ArrowDown: index + 1, ArrowUp: index - 1, Home: 0, End: count - 1, PageDown: index + page, PageUp: index - page };
    if (event.key === "Enter") {
      event.preventDefault();
      onEnter?.(index);
      return;
    }
    if (event.key === " ") {
      event.preventDefault();
      onSelect(index);
      return;
    }
    const to = moves[event.key];
    if (to === undefined) return;
    event.preventDefault();
    const next = Math.min(Math.max(0, to), count - 1);
    reveal(next);
    focusAfter.current = next;
    onSelect(next);
  }

  const rows: ReactNode[] = [];
  for (let i = start; i < end; i++) {
    const selectedRow = i === selected;
    rows.push(
      <div
        key={i}
        {...rowAttrs(i)}
        role="row"
        aria-rowindex={i + 2}
        aria-selected={selectedRow}
        data-row={i}
        tabIndex={i === active || (active < start && i === start) || (active >= end && i === start) ? 0 : -1}
        className={selectedRow ? `${styles.row} ${styles.selected}` : styles.row}
        onClick={() => onSelect(i)}
        onKeyDown={(e) => onKeyDown(e, i)}
      >
        {cells(i).map((cell, c) => (
          <div key={c} role="gridcell" className={styles.cell}>
            {cell}
          </div>
        ))}
      </div>,
    );
  }

  return (
    <div role="grid" aria-label={label} aria-rowcount={count + 1} className={`${styles.table} ${styles[kind]}`}>
      <div role="rowgroup" className={styles.head}>
        <div role="row" aria-rowindex={1} className={styles.headRow}>
          {columns.map((c) => (
            <div key={c.title} role="columnheader" className={styles.cell}>
              {c.title}
            </div>
          ))}
        </div>
      </div>
      <div ref={body} role="rowgroup" data-text-table-body="" className={styles.body} onScroll={(e) => setScrollTop(e.currentTarget.scrollTop)}>
        <div aria-hidden="true" style={{ height: start * height } /* dyn */} />
        {rows}
        <div aria-hidden="true" style={{ height: (count - end) * height } /* dyn */} />
      </div>
    </div>
  );
});
