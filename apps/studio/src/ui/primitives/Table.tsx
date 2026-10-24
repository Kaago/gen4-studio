import { useEffect, useRef } from "react";
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
  /** Delete or Backspace on a focused row (not inside one of its fields) removes it. */
  onDeleteRow?: (key: string) => void;
  /** A line below the table: a count ("12 von 20"), a warning. */
  footer?: ReactNode;
};

const TABBABLE = "input:not([type=hidden]), select, textarea, button, a[href], [tabindex]";

/** The elements Tab stops at under `root`, in document order (no positive tab indices in this app). */
function tabbables(root: ParentNode): HTMLElement[] {
  return [...root.querySelectorAll<HTMLElement>(TABBABLE)].filter(
    (el) => el.tabIndex >= 0 && !el.matches(":disabled") && el.closest("[hidden], [inert]") === null,
  );
}

/** Moves the focus to the first of `candidates` that takes it (one may be hidden by CSS). */
function focusFirst(candidates: readonly HTMLElement[]): void {
  for (const el of candidates) {
    el.focus();
    if (document.activeElement === el) return;
  }
}

/**
 * A table whose rows are one tab stop: Tab reaches the current row and leaves the table from it, the arrow keys move
 * between rows. Enter goes into the first field of the row, Tab then cycles through that row's fields, and Escape goes
 * back to the row. No virtualisation: the data tables have at most a few dozen rows.
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

  // Tab into the table from outside: the browser stops at the first field of the first row; send the focus on to the
  // row stop instead. A click into a field is no Tab and stays where it lands.
  useEffect(() => {
    const table = ref.current;
    if (!table) return;
    let tabbing = false;
    const onKey = (event: globalThis.KeyboardEvent) => {
      tabbing = event.key === "Tab";
    };
    const onPointer = () => {
      tabbing = false;
    };
    const onFocusIn = (event: FocusEvent) => {
      if (!tabbing) return;
      const from = event.relatedTarget;
      if (from instanceof Node && table.contains(from)) return;
      const target = event.target as HTMLElement;
      const row = target.closest<HTMLElement>("tr[data-roving]");
      if (!row || row === target || !table.contains(row)) return;
      table.querySelector<HTMLElement>('tr[data-roving][tabindex="0"]')?.focus();
    };
    document.addEventListener("keydown", onKey, true);
    document.addEventListener("pointerdown", onPointer, true);
    document.addEventListener("mousedown", onPointer, true);
    table.addEventListener("focusin", onFocusIn);
    return () => {
      document.removeEventListener("keydown", onKey, true);
      document.removeEventListener("pointerdown", onPointer, true);
      document.removeEventListener("mousedown", onPointer, true);
      table.removeEventListener("focusin", onFocusIn);
    };
  }, []);

  /** Tab on a row leaves the table: to the next tabbable element after it (Shift: before it). */
  const leaveTable = (backwards: boolean) => {
    const table = ref.current;
    if (!table) return;
    const outside = tabbables(document).filter((el) => !table.contains(el));
    const after = (el: HTMLElement) => (table.compareDocumentPosition(el) & Node.DOCUMENT_POSITION_FOLLOWING) !== 0;
    focusFirst(backwards ? outside.filter((el) => !after(el)).reverse() : outside.filter(after));
  };

  const onRowKey = (row: TableRow) => (event: KeyboardEvent<HTMLTableRowElement>) => {
    const rowEl = event.currentTarget;
    const target = event.target as HTMLElement;
    const plain = !event.altKey && !event.ctrlKey && !event.metaKey;
    if (target === rowEl) {
      if (event.key === "Tab" && plain) {
        event.preventDefault();
        leaveTable(event.shiftKey);
      } else if (event.key === "Enter") {
        const field = tabbables(rowEl)[0];
        if (!field) return;
        event.preventDefault();
        field.focus();
      } else if ((event.key === "Delete" || event.key === "Backspace") && plain && !event.shiftKey && onDeleteRow && !row.disabled) {
        event.preventDefault();
        onDeleteRow(row.key);
      }
      return;
    }
    // Keys from a popup of a field (a portal) or ones the field used itself are not the table's.
    if (event.defaultPrevented || !rowEl.contains(target)) return;
    if (event.key === "Escape") {
      event.preventDefault();
      rowEl.focus();
    } else if (event.key === "Tab" && plain) {
      // Inside a row, Tab cycles through its fields.
      const fields = tabbables(rowEl);
      const at = fields.findIndex((el) => el === target || el.contains(target));
      const step = event.shiftKey ? -1 : 1;
      const next = fields[(at + step + fields.length) % fields.length];
      if (!next) return;
      event.preventDefault();
      next.focus();
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
