import { Toggle } from "@base-ui/react/toggle";
import { ToggleGroup } from "@base-ui/react/toggle-group";
import { Icon } from "./Icon";
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
};

/** Exactly one option is selected: pressing the active one again changes nothing. */
export function Segmented<V extends string>({ options, value, onChange, ariaLabel, size = "m" }: SegmentedProps<V>) {
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
      {options.map((o) => (
        <Toggle<V> key={o.value} value={o.value} className={styles.option}>
          {o.icon && <Icon name={o.icon} weight={o.value === value ? "fill" : "regular"} />}
          {o.label}
        </Toggle>
      ))}
    </ToggleGroup>
  );
}
