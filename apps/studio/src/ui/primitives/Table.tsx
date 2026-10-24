import { useRef } from "react";
import type { KeyboardEvent, ReactNode } from "react";
import styles from "./Table.module.css";
import { useRovingList } from "./useRovingList";

export type TableColumn = {
  key: string;
  label: string;
  /** Fixed width in pixels; without, the column shares the rest. */
  width?: number;
  align?: "start" | "end";
};

export type TableRow = {
  key: string;
  /** The content per column key: text or a field (`NumberField`, `Picker`, …). */
  cells: Record<string, ReactNode>;
  /** Pass-through attributes of the row (`data-move`, …) that tests and scripts address it by. */
  attrs?: Record<string, string>;
  /** Not selectable or deletable, skipped by the arrow keys. */
  disabled?: boolean;
};

export type TableProps = {
  columns: readonly TableColumn[];
  rows: readonly TableRow[];
  ariaLabel: string;
  selected?: string;
  /** Called when a row gets the focus or a click (the selection follows the focus). */
  onSelect?: (key: string) => void;
  /** Delete on a focused row (not inside one of its fields) removes it. */
  onDeleteRow?: (key: string) => void;
  /** A line below the table: a count ("12 von 20"), a warning. */
  footer?: ReactNode;
};

const FIELD = "input, select, textarea, button, [role=checkbox], [role=combobox], [tabindex]:not([data-roving])";

/**
 * A table whose rows are the stops: Tab lands on one row, the arrow keys move between rows, Enter goes into the first
 * field of the row and Escape from a field back to the row. No virtualisation: the data tables have at most a few dozen rows.
 */
export function Table({ columns, rows, ariaLabel, selected, onSelect, onDeleteRow, footer }: TableProps) {
  const ref = useRef<HTMLTableElement>(null);
  useRovingList(ref);
  // A click focuses the row and clicks it: report the new row once, not for both events.
  const reported = useRef(selected);
  reported.current = selected;
  const choose = (row: TableRow) => {
    if (row.disabled || reported.current === row.key) return;
    reported.current = row.key;
    onSelect?.(row.key);
  };

  const onRowKey = (row: TableRow) => (event: KeyboardEvent<HTMLTableRowElement>) => {
    const rowEl = event.currentTarget;
    const target = event.target as Node;
    if (event.target === rowEl) {
      if (event.key === "Enter") {
        const field = rowEl.querySelector<HTMLElement>(FIELD);
        if (!field) return;
        event.preventDefault();
        field.focus();
      } else if (event.key === "Delete" && onDeleteRow && !row.disabled) {
        event.preventDefault();
        onDeleteRow(row.key);
      }
      return;
    }
    // Escape from a field inside the row (not from a popup of it, and not when the field used the key itself).
    if (event.key === "Escape" && !event.defaultPrevented && rowEl.contains(target)) {
      event.preventDefault();
      rowEl.focus();
    }
  };

  return (
    <div className={styles.wrap}>
      <table ref={ref} role="grid" aria-label={ariaLabel} className={styles.table}>
        <colgroup>
          {columns.map((c) => (
            <col key={c.key} style={c.width !== undefined ? { width: c.width } : undefined /* dyn */} />
          ))}
        </colgroup>
        <thead>
          <tr role="row">
            {columns.map((c) => (
              <th key={c.key} role="columnheader" scope="col" className={c.align === "end" ? styles.end : undefined}>
                {c.label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr
              key={row.key}
              {...row.attrs}
              role="row"
              data-roving=""
              aria-selected={row.key === selected}
              aria-disabled={row.disabled ? true : undefined}
              className={styles.row}
              onClick={() => choose(row)}
              onFocus={() => choose(row)}
              onKeyDown={onRowKey(row)}
            >
              {columns.map((c) => (
                <td key={c.key} role="gridcell" className={c.align === "end" ? styles.end : undefined}>
                  {row.cells[c.key]}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
      {footer !== undefined && <div className={styles.footer}>{footer}</div>}
    </div>
  );
}
