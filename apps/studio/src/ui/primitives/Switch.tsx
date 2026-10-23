import { Switch as BaseSwitch } from "@base-ui/react/switch";
import styles from "./Switch.module.css";

export type SwitchProps = {
  label: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  disabled?: boolean;
};

/** A switch with its label; the label is part of the click target. The owner holds the state. */
export function Switch({ label, checked, onChange, disabled }: SwitchProps) {
  return (
    <label className={styles.row} data-disabled={disabled ? "" : undefined}>
      <BaseSwitch.Root className={styles.switch} checked={checked} disabled={disabled} onCheckedChange={(next) => onChange(next)}>
        <BaseSwitch.Thumb className={styles.thumb} />
      </BaseSwitch.Root>
      <span className={styles.text}>{label}</span>
    </label>
  );
}
