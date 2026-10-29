import { Fragment } from "react";
import type { ReactNode } from "react";
import styles from "./ChangeTable.module.css";

/** A run of a word diff: unchanged, only on the old side, or only on the new one. */
export type ChangeSpan = { op: "same" | "removed" | "added"; text: string };

/** One side of a row: a value, null where the side has none, or the spans of a word diff. */
export type ChangeCell = string | null | readonly ChangeSpan[];

export interface ChangeTableRow {
  key: string;
  /** What changed: a JSON path, a field's name, a cell, a message. Shown in mono. */
  field: ReactNode;
  /** A hover hint on the field (the JSON path behind a German name). */
  title?: string;
  old: ChangeCell;
  new: ChangeCell;
}

export type ChangeTableProps = {
  /** The heads of the three columns: what, the old side, the new side („Feld“, „Commit“, „Projekt“). */
  columns: readonly [string, string, string];
  rows: readonly ChangeTableRow[];
  /** Names the table for assistive technology (its caption, not shown). */
  ariaLabel: string;
  /** A line below the table for the rows past the cap („und 12 weitere Felder“). */
  more?: string;
};

/** A side: „—“ with a hidden „fehlt“ where there is none, the spans as `<del>`/`<ins>`, else the value. */
function Cell({ value }: { value: ChangeCell }) {
  if (value === null) {
    return (
      <code className={styles.value}>
        <span aria-hidden="true">—</span>
        <span className={styles.hidden}>fehlt</span>
      </code>
    );
  }
  if (typeof value === "string") return <code className={styles.value}>{value}</code>;
  return (
    <code className={styles.value}>
      {value.map((span, i) =>
        span.op === "removed" ? (
          <del key={i} className={styles.removed}>
            {span.text}
          </del>
        ) : span.op === "added" ? (
          <ins key={i} className={styles.added}>
            {span.text}
          </ins>
        ) : (
          <span key={i}>{span.text}</span>
        ),
      )}
    </code>
  );
}

/** A field as shown: a string with a soft break (`<wbr>`) after each `.` and before each `[`, so a JSON path wraps only between its
 * parts; anything else as given. */
function Field({ field }: { field: ReactNode }) {
  if (typeof field !== "string") return field;
  const parts = field.split(/(?<=\.)|(?=\[)/).filter((part) => part !== "");
  return parts.map((part, i) => (
    <Fragment key={i}>
      {i > 0 && <wbr />}
      {part}
    </Fragment>
  ));
}

/**
 * „Feld: alt → neu“ as a table of three columns: what changed (the row's header, as wide as it needs up to a limit, a path broken only
 * between its parts), the old and the new side sharing the rest, all in mono. Word spans that
 * went are struck through on the error surface, ones that came underlined on the success surface (not colour alone); a missing side reads
 * „—“, „fehlt“ for a screen reader. Read only: no selection, no tab stop.
 */
export function ChangeTable({ columns, rows, ariaLabel, more }: ChangeTableProps) {
  return (
    <div className={styles.wrap}>
      <table className={styles.table}>
        <caption className={styles.hidden}>{ariaLabel}</caption>
        <thead>
          <tr>
            {columns.map((label, i) => (
              <th key={i} scope="col">
                {label}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.key}>
              <th scope="row" className={styles.field}>
                <code className={`${styles.value} ${styles.path}`} title={row.title}>
                  <Field field={row.field} />
                </code>
              </th>
              <td>
                <Cell value={row.old} />
              </td>
              <td>
                <Cell value={row.new} />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      {more !== undefined && (
        <p className={styles.more} data-change-table-more="">
          {more}
        </p>
      )}
    </div>
  );
}
