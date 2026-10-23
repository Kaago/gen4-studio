import type { KeyboardEvent, ReactNode } from "react";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import styles from "./ListRow.module.css";

/** Pass-through `data-*` attributes (`data-archive`, `data-script`, …) that tests and scripts address rows by. */
export type RowDataAttrs = Record<`data-${string}`, string | number | boolean | undefined>;

export type ListRowProps = {
  label: string;
  /** Technical name or number, shown in mono and grey after the label. */
  tech?: string;
  icon?: IconName;
  selected: boolean;
  onSelect: () => void;
  trailing?: ReactNode;
  dataAttrs?: RowDataAttrs;
};

/** Enter and Space select, but only when the row itself has focus (not a control inside `trailing`). */
export function selectOnKey(event: KeyboardEvent<HTMLElement>, onSelect: () => void): void {
  if (event.target !== event.currentTarget) return;
  if (event.key !== "Enter" && event.key !== " ") return;
  event.preventDefault();
  onSelect();
}

/** One row of a flat list. Put the rows inside a host that calls `useRovingList`. */
export function ListRow({ label, tech, icon, selected, onSelect, trailing, dataAttrs }: ListRowProps) {
  return (
    <div
      {...dataAttrs}
      role="option"
      aria-selected={selected}
      data-roving=""
      className={`${styles.row} ${selected ? styles.selected : ""}`}
      onClick={onSelect}
      onKeyDown={(e) => selectOnKey(e, onSelect)}
    >
      {icon && <Icon name={icon} />}
      <span className={styles.label}>{label}</span>
      {tech && <span className={styles.tech}>{tech}</span>}
      {trailing && <span className={styles.trailing}>{trailing}</span>}
    </div>
  );
}
