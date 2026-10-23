import { Checkbox as BaseCheckbox } from "@base-ui/react/checkbox";
import { Icon } from "./Icon";
import styles from "./Checkbox.module.css";

export type CheckboxProps = {
  label: string;
  /** `"mixed"` is the third state of a parent over partly selected children. */
  checked: boolean | "mixed";
  onChange: (checked: boolean) => void;
  disabled?: boolean;
};

/** A checkbox with its label; the label is part of the click target. The owner holds the state. */
export function Checkbox({ label, checked, onChange, disabled }: CheckboxProps) {
  return (
    <label className={styles.row} data-disabled={disabled ? "" : undefined}>
      <BaseCheckbox.Root
        className={styles.box}
        checked={checked === true}
        indeterminate={checked === "mixed"}
        disabled={disabled}
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
