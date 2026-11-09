import { useTranslation } from "react-i18next";
import { Button } from "./Button";
import { Icon } from "./Icon";
import { Popover } from "./Popover";
import { typeClass } from "./TypeBadge";
import styles from "./FilterChips.module.css";

export type FilterChipOption = {
  value: string;
  label: string;
  /** How many entries the filter would show, after the label. */
  count?: number;
  /** A Pokémon type number: the chip shows the type colour before the label. */
  type?: number;
};

export type FilterChipsProps = {
  options: readonly FilterChipOption[];
  /** The chosen values; empty means no filter. */
  value: ReadonlySet<string>;
  onChange: (next: Set<string>) => void;
  /** Names the group of chips ("Typ", "Taschen"). */
  ariaLabel: string;
  /** How many chips stand in the row; the rest open from "n weitere". */
  visible?: number;
  /**
   * Exactly one chip is chosen (`value` holds one): another chip replaces it, the chosen one pressed again changes nothing, and there is no
   * „Filter lösen“ (an „Alle“ chip is the way back).
   */
  single?: boolean;
};

/**
 * A row of toggle chips for a multiple choice filter (with `single`, a single choice). Chips past `visible` sit in a popover behind "n weitere"; a chosen
 * one of them stays in the row, so every active filter is in sight. „Filter lösen“ clears the choice.
 */
export function FilterChips({ options, value, onChange, ariaLabel, visible = 6, single = false }: FilterChipsProps) {
  const { t } = useTranslation("ui");
  const shown = options.filter((o, i) => i < visible || value.has(o.value));
  const rest = options.filter((o, i) => i >= visible && !value.has(o.value));
  const toggle = (option: string) => {
    if (single) {
      if (!value.has(option)) onChange(new Set([option]));
      return;
    }
    const next = new Set(value);
    if (next.has(option)) next.delete(option);
    else next.add(option);
    onChange(next);
  };
  const chip = (o: FilterChipOption) => {
    const on = value.has(o.value);
    return (
      <button key={o.value} type="button" aria-pressed={on} className={styles.chip} onClick={() => toggle(o.value)}>
        {on && <Icon name="check" />}
        {o.type !== undefined && <span className={`${styles.swatch} ${typeClass(o.type)}`} aria-hidden="true" />}
        <span>{o.label}</span>
        {o.count !== undefined && <span className={styles.count}>{o.count}</span>}
      </button>
    );
  };
  return (
    <div role="group" aria-label={ariaLabel} className={styles.chips}>
      {shown.map(chip)}
      {rest.length > 0 && (
        <Popover trigger={t("filterChips.more", { count: rest.length })}>
          <div role="group" aria-label={t("filterChips.moreGroup", { label: ariaLabel })} className={styles.more}>
            {rest.map(chip)}
          </div>
        </Popover>
      )}
      {!single && value.size > 0 && (
        <Button kind="quiet" size="s" onClick={() => onChange(new Set())}>
          {t("filterChips.clear")}
        </Button>
      )}
    </div>
  );
}
