import { useRef, useState } from "react";
import type { KeyboardEvent } from "react";
import { Checkbox } from "./Checkbox";
import { Tooltip } from "./Tooltip";
import styles from "./ToggleGrid.module.css";

export type ToggleCell = {
  id: string;
  /** The short visible label ("TM40"). */
  label: string;
  /** What the cell stands for ("Aero-Ass"); part of the accessible name and the tooltip, absent when unknown. */
  name?: string;
  checked: boolean;
  disabled?: boolean;
};

export type ToggleGridProps = {
  cells: readonly ToggleCell[];
  columns: number;
  onToggle: (id: string, checked: boolean) => void;
  ariaLabel: string;
};

const fullName = (cell: ToggleCell) => (cell.name ? `${cell.label} ${cell.name}` : cell.label);

/**
 * A grid of checkboxes (TM and HM compatibility, flags). One tab stop: Left and Right move by one cell, Up and Down by one
 * row, Home and End to the ends; locked cells are skipped. Space toggles. The count ("27 von 100") belongs outside.
 */
export function ToggleGrid({ cells, columns, onToggle, ariaLabel }: ToggleGridProps) {
  const ref = useRef<HTMLDivElement>(null);
  const [active, setActive] = useState<string | null>(null);
  const enabled = cells.filter((c) => !c.disabled);
  const stop = enabled.find((c) => c.id === active)?.id ?? enabled[0]?.id;

  const boxOf = (id: string) => ref.current?.querySelector<HTMLElement>(`[data-cell="${CSS.escape(id)}"] [role=checkbox]`);
  const cellOf = (el: EventTarget) => (el instanceof HTMLElement ? el.closest<HTMLElement>("[data-cell]")?.dataset.cell : undefined);

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.altKey || event.ctrlKey || event.metaKey || event.shiftKey) return;
    const from = cells.findIndex((c) => c.id === cellOf(event.target));
    if (from < 0) return;
    const usable = (i: number) => cells[i] !== undefined && !cells[i].disabled;
    // The next usable cell from `start` in steps of `step`; `fallback` when there is none.
    const walk = (start: number, step: number, fallback: number) => {
      for (let i = start; i >= 0 && i < cells.length; i += step) if (usable(i)) return i;
      return fallback;
    };
    let to: number;
    if (event.key === "ArrowRight") to = walk(from + 1, 1, from);
    else if (event.key === "ArrowLeft") to = walk(from - 1, -1, from);
    else if (event.key === "ArrowDown") to = walk(from + columns, columns, from);
    else if (event.key === "ArrowUp") to = walk(from - columns, -columns, from);
    else if (event.key === "Home") to = walk(0, 1, from);
    else if (event.key === "End") to = walk(cells.length - 1, -1, from);
    else return;
    event.preventDefault();
    const target = cells[to];
    if (!target) return;
    setActive(target.id);
    boxOf(target.id)?.focus();
  };

  return (
    <div
      ref={ref}
      role="group"
      aria-label={ariaLabel}
      className={styles.grid}
      style={{ gridTemplateColumns: `repeat(${columns}, max-content)` } /* dyn */}
      onKeyDown={onKeyDown}
      onFocus={(event) => {
        const id = cellOf(event.target);
        if (id !== undefined) setActive(id);
      }}
    >
      {cells.map((cell) => {
        const name = fullName(cell);
        const box = (
          <Checkbox
            label={cell.label}
            ariaLabel={cell.name ? name : undefined}
            checked={cell.checked}
            disabled={cell.disabled}
            tabIndex={cell.id === stop ? 0 : -1}
            onChange={(next) => onToggle(cell.id, next)}
          />
        );
        return (
          <div key={cell.id} className={styles.cell} data-cell={cell.id}>
            {cell.name ? (
              <Tooltip content={name}>
                <span className={styles.tip}>{box}</span>
              </Tooltip>
            ) : (
              box
            )}
          </div>
        );
      })}
    </div>
  );
}
