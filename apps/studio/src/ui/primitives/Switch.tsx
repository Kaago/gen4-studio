import { useId } from "react";
import { Switch as BaseSwitch } from "@base-ui/react/switch";
import styles from "./Switch.module.css";

export type SwitchProps = {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
  /** Shown beside the switch and read as its description (`aria-describedby`), e.g. why it is disabled. */
  hint?: string;
};

/** A switch with its label; the label is part of the click target. The owner holds the state. */
export function Switch({ label, checked, onChange, disabled, hint }: SwitchProps) {
  const hintId = useId();
  const row = (
    <label className={styles.row} data-disabled={disabled ? "" : undefined}>
      <BaseSwitch.Root
        className={styles.switch}
        checked={checked}
        disabled={disabled}
        onCheckedChange={(next) => onChange(next)}
        aria-describedby={hint ? hintId : undefined}
      >
        <BaseSwitch.Thumb className={styles.thumb} />
      </BaseSwitch.Root>
      <span className={styles.text}>{label}</span>
    </label>
  );
  if (!hint) return row;
  // The hint stays outside the label: it describes the switch, it is not part of its name.
  return (
    <span className={styles.withHint}>
      {row}
      <span id={hintId} className={styles.hint}>
        {hint}
      </span>
    </span>
  );
}
