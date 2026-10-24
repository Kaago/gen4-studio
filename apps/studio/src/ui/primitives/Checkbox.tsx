import { Checkbox as BaseCheckbox } from "@base-ui/react/checkbox";
import { Icon } from "./Icon";
import styles from "./Checkbox.module.css";

export type CheckboxProps = {
  label: string;
  /** `"mixed"` is the third state of a parent over partly selected children. */
  checked: boolean | "mixed";
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  /** An accessible name longer than the visible label ("TM40 Aero-Ass" for the label "TM40"); it starts with the label. */
  ariaLabel?: string;
  /** For a host that moves a single tab stop over many boxes (`ToggleGrid`). */
  tabIndex?: number;
};

/** A checkbox with its label; the label is part of the click target. The owner holds the state. */
export function Checkbox({ label, checked, onChange, disabled, ariaLabel, tabIndex }: CheckboxProps) {
  return (
    <label className={styles.row} data-disabled={disabled ? "" : undefined}>
      <BaseCheckbox.Root
        className={styles.box}
        checked={checked === true}
        indeterminate={checked === "mixed"}
        disabled={disabled}
        // Only when given: an explicit `undefined` would replace Base UI's own defaults.
        {...(ariaLabel !== undefined && { "aria-label": ariaLabel })}
        {...(tabIndex !== undefined && { tabIndex })}
        onCheckedChange={(next) => onChange(next)}
      >
        <BaseCheckbox.Indicator className={styles.indicator} keepMounted={false}>
          <Icon name={checked === "mixed" ? "minus" : "check"} />
        </BaseCheckbox.Indicator>
      </BaseCheckbox.Root>
      <span className={styles.text}>{label}</span>
    </label>
  );
}
