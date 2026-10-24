import { useId } from "react";
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
  /** Content before the label, in place of or beside the icon (a `Sprite`, a `TypeBadge`). */
  leading?: ReactNode;
  /** A second line below the label (a state, "gebaut vor 4 Minuten"); it describes the row, the label names it. */
  subtitle?: string;
  selected: boolean;
  onSelect: () => void;
  trailing?: ReactNode;
  dataAttrs?: RowDataAttrs;
  /** The accessible name when the visible parts (label, tech, trailing) would read badly run together. */
  ariaLabel?: string;
  /** A hover hint with more than the row has room for. */
  title?: string;
  /** Not selectable (`aria-disabled`, no click, skipped by the arrow keys); `title` can say why. */
  disabled?: boolean;
};

/** Enter and Space select, but only when the row itself has focus (not a control inside `trailing`). */
export function selectOnKey(event: KeyboardEvent<HTMLElement>, onSelect: () => void): void {
  if (event.target !== event.currentTarget) return;
  if (event.key !== "Enter" && event.key !== " ") return;
  event.preventDefault();
  onSelect();
}

/** The classes of a row: selected and disabled are looks of their own. */
export function rowClass(selected: boolean, disabled: boolean | undefined): string {
  return [styles.row, selected ? styles.selected : "", disabled ? styles.disabled : ""].filter(Boolean).join(" ");
}

/** One row of a flat list. Put the rows inside a host that calls `useRovingList`. */
export function ListRow({ label, tech, icon, leading, subtitle, selected, onSelect, trailing, dataAttrs, ariaLabel, title, disabled }: ListRowProps) {
  const id = useId();
  const labelId = `${id}-label`;
  const subtitleId = `${id}-subtitle`;
  const named = subtitle !== undefined && ariaLabel === undefined;
  return (
    <div
      {...dataAttrs}
      role="option"
      aria-selected={selected}
      aria-label={ariaLabel}
      aria-labelledby={named ? labelId : undefined}
      aria-describedby={subtitle !== undefined ? subtitleId : undefined}
      aria-disabled={disabled ? true : undefined}
      title={title}
      data-roving=""
      className={rowClass(selected, disabled)}
      onClick={disabled ? undefined : onSelect}
      onKeyDown={disabled ? undefined : (e) => selectOnKey(e, onSelect)}
    >
      {leading !== undefined && <span className={styles.leading}>{leading}</span>}
      {icon && <Icon name={icon} />}
      {subtitle !== undefined ? (
        <span className={styles.text}>
          <span id={labelId} className={styles.label}>
            {label}
          </span>
          <span id={subtitleId} className={styles.subtitle}>
            {subtitle}
          </span>
        </span>
      ) : (
        <span className={styles.label}>{label}</span>
      )}
      {tech && <span className={styles.tech}>{tech}</span>}
      {trailing && <span className={styles.trailing}>{trailing}</span>}
    </div>
  );
}
