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

/** Elements that can take part in the Tab order (whether they do is their `tabIndex`). */
const FOCUSABLE =
  'input:not([type=hidden]), select, textarea, button, a[href], summary, iframe, audio[controls], video[controls], [contenteditable]:not([contenteditable="false"]), [tabindex]';

/** Marks a field the table took out of the Tab order; the value is its own `tabindex` attribute ("" for none). */
const HELD = "data-table-tab";

/**
 * Whether an element under a row is one of its fields. Not: the focus guards a popup puts beside its trigger (they hand
 * Tab over between trigger and popup and must keep their own tab index) and anything hidden from assistive technology.
 */
function isField(el: HTMLElement): boolean {
  return !el.matches("[data-base-ui-focus-guard], [data-floating-ui-focus-guard]") && el.closest('[aria-hidden="true"], [hidden], [inert]') === null;
}

/** The fields of a row, in document order: Tab stops now, or ones the table holds out of the Tab order. */
function fieldsOf(row: HTMLElement): HTMLElement[] {
  return [...row.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(
    (el) => (el.tabIndex >= 0 || el.hasAttribute(HELD)) && !el.matches(":disabled") && isField(el),
  );
}

/** Whether a field of the row has its popup open (a select's list, a picker): the focus in that popup still belongs to the row. */
const hasOpenPopup = (row: HTMLElement) => row.querySelector('[aria-expanded="true"]') !== null;

/**
 * A table whose rows are one tab stop: Tab reaches the current row and leaves the table from it, the arrow keys move
 * between rows. The fields of a row join the Tab order only while the focus is inside that row: Enter goes into its first
 * field (a click into any field does too), Tab then cycles through that row's fields, and Escape goes back to the row.
 * No virtualisation: the data tables have at most a few dozen rows.
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

  // The Tab order: every field of a row the focus is not inside gets tabindex -1 (its own value is kept in HELD and
  // put back when the focus enters the row). Clicks and `focus()` still reach such a field. The focus in a popup of a
  // field (a portal outside the table) still counts as inside the row while that popup is open.
  useEffect(() => {
    const table = ref.current;
    if (!table) return;
    let entered: HTMLElement | null = null;
    let observer: MutationObserver | null = null;
    const apply = () => {
      for (const row of table.querySelectorAll<HTMLElement>("tr[data-roving]")) {
        for (const el of row.querySelectorAll<HTMLElement>(FOCUSABLE)) {
          if (!isField(el)) continue;
          const held = el.getAttribute(HELD);
          if (row === entered) {
            if (held === null) continue;
            el.removeAttribute(HELD);
            if (held === "") el.removeAttribute("tabindex");
            else el.setAttribute("tabindex", held);
          } else if (held === null ? el.tabIndex >= 0 : el.getAttribute("tabindex") !== "-1") {
            el.setAttribute(HELD, el.getAttribute("tabindex") ?? "");
            el.setAttribute("tabindex", "-1");
          }
        }
      }
      // The table's own writes are no news for the observer.
      observer?.takeRecords();
    };
    const enter = (row: HTMLElement | null) => {
      entered = row;
      // A field entered by a click makes its row the row stop, as the arrow keys would.
      if (row) for (const r of table.querySelectorAll<HTMLElement>("tr[data-roving]")) r.tabIndex = r === row ? 0 : -1;
      apply();
    };
    // The focus is outside the table (or on its way): the row stays entered only while one of its popups is open.
    const settle = () => {
      if (entered && !table.contains(document.activeElement) && !hasOpenPopup(entered)) entered = null;
      apply();
    };
    const onFocusIn = (event: FocusEvent) => {
      const target = event.target as HTMLElement;
      const row = target.closest<HTMLElement>("tr[data-roving]");
      enter(row && row !== target && table.contains(row) ? row : null);
    };
    const onFocusOut = (event: FocusEvent) => {
      const next = event.relatedTarget;
      if (!(next instanceof Node) || !table.contains(next)) settle();
    };
    const onFocusElsewhere = (event: FocusEvent) => {
      if (entered && event.target instanceof Node && !table.contains(event.target)) settle();
    };
    observer = new MutationObserver((records) => {
      for (const record of records) {
        // A component changed the tab index of a held field itself (a roving group in a cell): that is now its own value.
        const el = record.target as HTMLElement;
        if (record.attributeName === "tabindex" && el.hasAttribute(HELD)) el.setAttribute(HELD, el.getAttribute("tabindex") ?? "");
      }
      // A popup that closed with the focus elsewhere ends the row; new or changed fields are held.
      settle();
    });
    apply();
    observer.observe(table, { subtree: true, childList: true, attributes: true, attributeFilter: ["tabindex", "aria-expanded"] });
    table.addEventListener("focusin", onFocusIn);
    table.addEventListener("focusout", onFocusOut);
    document.addEventListener("focusin", onFocusElsewhere, true);
    return () => {
      observer?.disconnect();
      table.removeEventListener("focusin", onFocusIn);
      table.removeEventListener("focusout", onFocusOut);
      document.removeEventListener("focusin", onFocusElsewhere, true);
    };
  }, []);

  const onRowKey = (row: TableRow) => (event: KeyboardEvent<HTMLTableRowElement>) => {
    const rowEl = event.currentTarget;
    const target = event.target as HTMLElement;
    const plain = !event.altKey && !event.ctrlKey && !event.metaKey;
    if (target === rowEl) {
      // Tab on the row itself is the browser's: the row's fields are out of the Tab order, so it leaves the table.
      if (event.key === "Enter") {
        const field = fieldsOf(rowEl)[0];
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
      const fields = fieldsOf(rowEl);
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
