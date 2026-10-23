import { Select } from "@base-ui/react/select";
import { Icon } from "./Icon";
import type { IconName } from "./icons";
import styles from "./SelectField.module.css";

export type SelectOption<V extends number | string> = { value: V; label: string; icon?: IconName };

export type SelectFieldProps<V extends number | string> = {
  /** Always the accessible name; shown above the field only with `showLabel`. */
  label: string;
  value: V;
  options: readonly SelectOption<V>[];
  onChange: (value: V) => void;
  disabled?: boolean;
  showLabel?: boolean;
};

/** A value the options do not list (a byte the editor has no name for) stays visible and selected as "Wert 7". */
function withUnknown<V extends number | string>(value: V, options: readonly SelectOption<V>[]): readonly SelectOption<V>[] {
  return options.some((o) => o.value === value) ? options : [...options, { value, label: `Wert ${value}` }];
}

export function SelectField<V extends number | string>({ label, value, options, onChange, disabled, showLabel }: SelectFieldProps<V>) {
  const all = withUnknown(value, options);
  const current = all.find((o) => o.value === value);
  return (
    <Select.Root<V> value={value} items={all} disabled={disabled} onValueChange={(next) => next !== null && onChange(next)}>
      <div className={styles.field}>
        {showLabel && <Select.Label className={styles.label}>{label}</Select.Label>}
        <Select.Trigger className={styles.trigger} aria-label={label}>
          <span className={styles.value}>
            {current?.icon && <Icon name={current.icon} />}
            <Select.Value />
          </span>
          <Select.Icon className={styles.caret}>
            <Icon name="caretDown" />
          </Select.Icon>
        </Select.Trigger>
      </div>
      <Select.Portal>
        <Select.Positioner className={styles.positioner} sideOffset={4} alignItemWithTrigger={false}>
          <Select.Popup className={styles.popup}>
            <Select.List className={styles.list}>
              {all.map((o) => (
                <Select.Item key={o.value} value={o.value} className={styles.item}>
                  <Select.ItemIndicator className={styles.mark}>
                    <Icon name="check" />
                  </Select.ItemIndicator>
                  {o.icon && <Icon name={o.icon} />}
                  <Select.ItemText>{o.label}</Select.ItemText>
                </Select.Item>
              ))}
            </Select.List>
          </Select.Popup>
        </Select.Positioner>
      </Select.Portal>
    </Select.Root>
  );
}
