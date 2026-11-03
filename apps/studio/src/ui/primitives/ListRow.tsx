import { ContextMenu } from "@base-ui/react/context-menu";
import { useId, useRef } from "react";
import type { HTMLAttributes, KeyboardEvent, ReactNode } from "react";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import { ContextMenuPopup, MenuEntries, openContextMenuByKey } from "./Menu";
import type { MenuEntry } from "./Menu";
import styles from "./ListRow.module.css";

/** Pass-through `data-*` attributes (`data-archive`, `data-script`, …) that tests and scripts address rows by. */
export type RowDataAttrs = Record<`data-${string}`, string | number | boolean | undefined>;

export type ListRowProps = {
  label: string;
  /** Technical name or number, shown in mono and grey after the label. */
  tech?: string;
  /** In a narrow row the name gives way, not `tech` (a number that identifies the row, next to a long trailing badge). */
  keepTech?: boolean;
  icon?: IconName;
  /** Content before the label, in place of or beside the icon (a `Sprite`, a `TypeBadge`). */
  leading?: ReactNode;
  /** A second line below the label (a state, "gebaut vor 4 Minuten"); it describes the row, the label names it. */
  subtitle?: string;
  /** The label emphasised (an own name over a technical one). */
  strong?: boolean;
  /** The second line technical, in mono like `tech` (a name of the game and a number). */
  techSubtitle?: boolean;
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
  /** In a list that renders only some of its rows: how many rows it has, and this row's place among them (1-based). */
  ariaSetSize?: number;
  ariaPosInSet?: number;
  /** Entries a right click on the row lists (also Shift+F10 and the context menu key on the focused row); without them the row has no menu of its own. */
  contextMenu?: readonly MenuEntry[];
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

/** The classes of a row's `tech`: it shrinks before the name, unless `keepTech`. */
export function techClass(keep: boolean | undefined): string {
  return keep ? `${styles.tech} ${styles.techKept}` : styles.tech;
}

/** The attributes of a row's element, with its pass-through `data-*` attributes. */
export type RowElementProps = Omit<HTMLAttributes<HTMLDivElement>, "contextMenu"> & RowDataAttrs & { "data-roving": string };

/**
 * The element of a row: a plain `div`, or with `menu` the trigger of a context menu (the `div` keeps its role and handlers), opened
 * by a right click and by Shift+F10 or the context menu key at the row's lower edge.
 */
export function RowElement({
  menu,
  menuLabel,
  children,
  ...attrs
}: RowElementProps & { menu: readonly MenuEntry[] | undefined; menuLabel?: string; children: ReactNode }) {
  // Whether the menu opening now was opened by key: then its first entry takes the focus (WAI-ARIA menu), as a menu button does.
  const byKey = useRef(false);
  if (menu === undefined) return <div {...attrs}>{children}</div>;
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    byKey.current = true;
    if (!openContextMenuByKey(event)) {
      byKey.current = false;
      attrs.onKeyDown?.(event);
    }
  };
  return (
    <ContextMenu.Root
      onOpenChange={(open) => {
        // A key that opened no new popup (the menu was open already) is not left for the next right click.
        if (!open) byKey.current = false;
      }}
    >
      <ContextMenu.Trigger render={<div {...attrs} onKeyDown={onKeyDown} />}>{children}</ContextMenu.Trigger>
      <ContextMenuPopup label={menuLabel} byKey={byKey}>
        <MenuEntries items={menu} />
      </ContextMenuPopup>
    </ContextMenu.Root>
  );
}

/** One row of a flat list. Put the rows inside a host that calls `useRovingList`. */
export function ListRow({ label, tech, keepTech, icon, leading, subtitle, strong, techSubtitle, selected, onSelect, trailing, dataAttrs, ariaLabel, title, disabled, ariaSetSize, ariaPosInSet, contextMenu }: ListRowProps) {
  const id = useId();
  const labelId = `${id}-label`;
  const subtitleId = `${id}-subtitle`;
  const named = subtitle !== undefined && ariaLabel === undefined;
  const labelClass = strong ? `${styles.label} ${styles.strong}` : styles.label;
  return (
    <RowElement
      {...dataAttrs}
      menu={contextMenu}
      menuLabel={ariaLabel ?? label}
      role="option"
      aria-selected={selected}
      aria-label={ariaLabel}
      aria-labelledby={named ? labelId : undefined}
      aria-describedby={subtitle !== undefined ? subtitleId : undefined}
      aria-disabled={disabled ? true : undefined}
      aria-setsize={ariaSetSize}
      aria-posinset={ariaPosInSet}
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
          <span id={labelId} className={labelClass}>
            {label}
          </span>
          <span id={subtitleId} className={techSubtitle ? `${styles.subtitle} ${styles.techSubtitle}` : styles.subtitle}>
            {subtitle}
          </span>
        </span>
      ) : (
        <span className={labelClass}>{label}</span>
      )}
      {tech && <span className={techClass(keepTech)}>{tech}</span>}
      {trailing && <span className={styles.trailing}>{trailing}</span>}
    </RowElement>
  );
}
