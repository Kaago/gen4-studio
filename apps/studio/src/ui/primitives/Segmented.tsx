import { Toggle } from "@base-ui/react/toggle";
import { ToggleGroup } from "@base-ui/react/toggle-group";
import { Icon } from "./Icon";
import { Tooltip } from "./Tooltip";
import type { IconName } from "./icons";
import styles from "./Segmented.module.css";

/** `disabled`: the option shows but cannot be picked (e.g. a choice the game does not support). */
export type SegmentedOption<V extends string> = { value: V; label: string; icon?: IconName; disabled?: boolean };

export type SegmentedProps<V extends string> = {
  options: readonly SegmentedOption<V>[];
  value: V;
  onChange: (value: V) => void;
  /** Names the group for assistive technology. */
  ariaLabel: string;
  size?: "m" | "s";
  /** Symbols only: each option needs an `icon`; its label stays the accessible name and shows as a tooltip. */
  iconOnly?: boolean;
  /** Options that do not fit in one row go on to the next instead of squeezing into each other (many or long labels in a narrow dialog). */
  wrap?: boolean;
};

/** Exactly one option is selected: pressing the active one again changes nothing. */
export function Segmented<V extends string>({ options, value, onChange, ariaLabel, size = "m", iconOnly = false, wrap = false }: SegmentedProps<V>) {
  return (
    <ToggleGroup<V>
      className={`${styles.group} ${styles[size]}${wrap ? ` ${styles.wrap}` : ""}`}
      aria-label={ariaLabel}
      value={[value]}
      onValueChange={(next) => {
        const picked = next[0];
        if (picked !== undefined && picked !== value) onChange(picked);
      }}
    >
      {options.map((o) => {
        const icon = o.icon && <Icon name={o.icon} weight={o.value === value ? "fill" : "regular"} />;
        const symbol = iconOnly && o.icon !== undefined;
        // One element tree in both modes (the tooltip only switches off): an option keeps its button, and its focus, when the mode changes.
        return (
          <Tooltip key={o.value} content={o.label} disabled={!symbol}>
            <Toggle<V> value={o.value} className={styles.option} aria-label={symbol ? o.label : undefined} disabled={o.disabled}>
              {icon}
              {symbol ? null : o.label}
            </Toggle>
          </Tooltip>
        );
      })}
    </ToggleGroup>
  );
}
