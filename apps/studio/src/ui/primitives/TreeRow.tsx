import { useId } from "react";
import type { KeyboardEvent } from "react";
import { Icon } from "./Icon";
import { RowElement, rowClass, selectOnKey } from "./ListRow";
import type { ListRowProps } from "./ListRow";
import listStyles from "./ListRow.module.css";
import styles from "./TreeRow.module.css";

export type TreeRowProps = ListRowProps & {
  /** Zero-based nesting depth; `aria-level` is one more. */
  depth: number;
  /** Absent for a leaf; otherwise the open state of a branch. */
  expanded?: boolean;
  onToggle?: () => void;
};

/** One row of a tree: indent by depth, a caret for branches, Right opens and Left closes a branch; `subtitle` and `contextMenu` as in `ListRow`. */
export function TreeRow({ label, tech, icon, subtitle, selected, onSelect, trailing, dataAttrs, ariaLabel, title, disabled, depth, expanded, onToggle, contextMenu }: TreeRowProps) {
  const branch = expanded !== undefined;
  const id = useId();
  const labelId = `${id}-label`;
  const subtitleId = `${id}-subtitle`;
  const named = subtitle !== undefined && ariaLabel === undefined;
  const onKeyDown = (event: KeyboardEvent<HTMLElement>) => {
    if (event.target === event.currentTarget && onToggle && branch) {
      const opens = event.key === "ArrowRight" && !expanded;
      const closes = event.key === "ArrowLeft" && expanded;
      if (opens || closes) {
        event.preventDefault();
        onToggle();
        return;
      }
    }
    if (!disabled) selectOnKey(event, onSelect);
  };
  return (
    <RowElement
      {...dataAttrs}
      menu={contextMenu}
      role="treeitem"
      aria-level={depth + 1}
      aria-selected={selected}
      aria-expanded={expanded}
      aria-label={ariaLabel}
      aria-labelledby={named ? labelId : undefined}
      aria-describedby={subtitle !== undefined ? subtitleId : undefined}
      aria-disabled={disabled ? true : undefined}
      title={title}
      data-roving=""
      className={rowClass(selected, disabled)}
      style={{ paddingLeft: `calc(var(--g4-dichte-zeile-innen) + ${depth} * var(--g4-raum-16))` }} /* dyn */
      onClick={disabled ? undefined : onSelect}
      onKeyDown={onKeyDown}
    >
      <span
        className={styles.caret}
        aria-hidden="true"
        onClick={(e) => {
          if (!branch) return;
          e.stopPropagation();
          onToggle?.();
        }}
      >
        {branch && <Icon name={expanded ? "caretDown" : "caretRight"} />}
      </span>
      {icon && <Icon name={icon} />}
      {subtitle !== undefined ? (
        <span className={listStyles.text}>
          <span id={labelId} className={listStyles.label}>
            {label}
          </span>
          <span id={subtitleId} className={listStyles.subtitle}>
            {subtitle}
          </span>
        </span>
      ) : (
        <span className={listStyles.label}>{label}</span>
      )}
      {tech && <span className={listStyles.tech}>{tech}</span>}
      {trailing && <span className={listStyles.trailing}>{trailing}</span>}
    </RowElement>
  );
}
