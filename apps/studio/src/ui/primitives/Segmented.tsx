import { Toggle } from "@base-ui/react/toggle";
import { ToggleGroup } from "@base-ui/react/toggle-group";
import { Icon } from "./Icon";
import { Tooltip } from "./Tooltip";
import type { IconName } from "./icons";
import styles from "./Segmented.module.css";

export type SegmentedOption<V extends string> = { value: V; label: string; icon?: IconName };

export type SegmentedProps<V extends string> = {
  options: readonly SegmentedOption<V>[];
  value: V;
  onChange: (value: V) => void;
  /** Names the group for assistive technology. */
  ariaLabel: string;
  size?: "m" | "s";
  /** Symbols only: each option needs an `icon`; its label stays the accessible name and shows as a tooltip. */
  iconOnly?: boolean;
};

/** Exactly one option is selected: pressing the active one again changes nothing. */
export function Segmented<V extends string>({ options, value, onChange, ariaLabel, size = "m", iconOnly = false }: SegmentedProps<V>) {
  return (
    <ToggleGroup<V>
      className={`${styles.group} ${styles[size]}`}
      aria-label={ariaLabel}
      value={[value]}
      onValueChange={(next) => {
        const picked = next[0];
        if (picked !== undefined && picked !== value) onChange(picked);
      }}
    >
      {options.map((o) => {
        const icon = o.icon && <Icon name={o.icon} weight={o.value === value ? "fill" : "regular"} />;
        return iconOnly && o.icon ? (
          <Tooltip key={o.value} content={o.label}>
            <Toggle<V> value={o.value} className={styles.option} aria-label={o.label}>
              {icon}
            </Toggle>
          </Tooltip>
        ) : (
          <Toggle<V> key={o.value} value={o.value} className={styles.option}>
            {icon}
            {o.label}
          </Toggle>
        );
      })}
    </ToggleGroup>
  );
}
