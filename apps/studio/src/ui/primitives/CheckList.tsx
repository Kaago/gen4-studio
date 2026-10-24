import type { ReactNode } from "react";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import type { RowDataAttrs } from "./ListRow";
import styles from "./CheckList.module.css";

export type CheckState = "ok" | "warning" | "error" | "info";

export type CheckRow = {
  id: string;
  state: CheckState;
  text: ReactNode;
  trailing?: ReactNode;
  /** Further `data-*` attributes of the row; a `data-check` here replaces the id there (a finding: its code, with `data-finding`). */
  dataAttrs?: RowDataAttrs;
};

export type CheckListProps = {
  rows: readonly CheckRow[];
  /** Names the list for assistive technology. */
  ariaLabel?: string;
};

const symbol: Record<CheckState, IconName> = { ok: "success", warning: "warning", error: "error", info: "info" };
const spoken: Record<CheckState, string> = { ok: "In Ordnung", warning: "Warnung", error: "Fehler", info: "Hinweis" };

/** Rows of findings: each says its state by symbol and spoken name, never by colour alone; `trailing` sits at the end of the row. */
export function CheckList({ rows, ariaLabel }: CheckListProps) {
  return (
    <ul role="list" className={styles.list} aria-label={ariaLabel}>
      {rows.map((row) => (
        <li key={row.id} className={`${styles.row} ${styles[row.state]}`} data-check={row.id} data-state={row.state} {...row.dataAttrs}>
          <span className={styles.icon} role="img" aria-label={spoken[row.state]}>
            <Icon name={symbol[row.state]} weight="fill" />
          </span>
          <div className={styles.text}>{row.text}</div>
          {row.trailing !== undefined && <div className={styles.trailing}>{row.trailing}</div>}
        </li>
      ))}
    </ul>
  );
}
