import type { ReactNode } from "react";
import { useTranslation } from "react-i18next";
import { fieldLabel } from "../../api";
import { InspectorRow, NumberField, SelectField } from "../../ui/primitives";
import type { NumberFieldProps, SelectOption } from "../../ui/primitives";
import type { Editing, NumericField } from "./PokedexHand";
import { COLORS, EGG_GROUPS, EV_YIELD, GENDER, GROWTH_RATES, STATS, hatchText } from "./pokedexModel";
import styles from "./Pokedex.module.css";

/** A column of the tab: a bold title with its grey note ("Basiswerte", "je 0 bis 255"). */
export function Column({ title, note, children }: { title: string; note?: string; children: ReactNode }) {
  return (
    <section className={styles.column} aria-label={title}>
      <h3 className={styles.columnTitle}>
        {title}
        {note && <span className={styles.columnNote}>{note}</span>}
      </h3>
      {children}
    </section>
  );
}

/** One number of the personal data as an inspector row; the bar says it differs from the saved value. */
export function PersonalNumber({ e, field, ...limits }: { e: Editing; field: NumericField } & Pick<NumberFieldProps, "bits" | "max" | "hint">) {
  const label = fieldLabel("species", field);
  const value = e.doc.personal[field];
  return (
    <InspectorRow label={label} deviates={value !== e.doc.saved[field]}>
      <div data-field={field}>
        <NumberField label={label} value={value} onCommit={(v) => e.setPersonal(field, v)} disabled={e.locked.personal} showBase={e.showBase} {...limits} />
      </div>
    </InspectorRow>
  );
}

/** One choice of the personal data (types, growth, gender, egg groups, colour). */
export function PersonalSelect({ e, field, options }: { e: Editing; field: NumericField; options: readonly SelectOption<number>[] }) {
  const label = fieldLabel("species", field);
  const value = e.doc.personal[field];
  return (
    <InspectorRow label={label} deviates={value !== e.doc.saved[field]}>
      <div data-field={field}>
        <SelectField label={label} value={value} options={options} onChange={(v) => e.setPersonal(field, v)} disabled={e.locked.personal} />
      </div>
    </InspectorRow>
  );
}

/** Werte: base stats, EV yield, catch rate, experience and friendship, breeding; flee rate and colour while tinkering. */
export function ValuesTab({ e, tinkering }: { e: Editing; tinkering: boolean }) {
  const { t } = useTranslation("data");
  return (
    <div className={styles.columns}>
      <Column title={t("valuesTab.baseStats")} note={t("valuesTab.baseStatsNote")}>
        {STATS.map((s) => (
          <PersonalNumber key={s.field} e={e} field={s.field} bits={8} />
        ))}
      </Column>
      <Column title={t("valuesTab.evYield")} note={t("valuesTab.evYieldNote")}>
        {EV_YIELD.map((y) => (
          <PersonalNumber key={y.field} e={e} field={y.field as NumericField} max={3} />
        ))}
      </Column>
      <Column title={t("valuesTab.catchAndBattle")}>
        <PersonalNumber e={e} field="catch_rate" bits={8} hint={t("valuesTab.catchHint")} />
        <PersonalNumber e={e} field="base_exp" bits={8} hint={t("valuesTab.expHint")} />
        <PersonalNumber e={e} field="base_friendship" bits={8} />
        {tinkering && <PersonalNumber e={e} field="flee_rate" bits={8} hint={t("valuesTab.fleeHint")} />}
      </Column>
      <Column title={t("valuesTab.breeding")}>
        <PersonalSelect e={e} field="growth_rate" options={GROWTH_RATES} />
        <PersonalSelect e={e} field="gender_ratio" options={GENDER} />
        <PersonalSelect e={e} field="egg_group1" options={EGG_GROUPS} />
        <PersonalSelect e={e} field="egg_group2" options={EGG_GROUPS} />
        <PersonalNumber e={e} field="egg_cycles" bits={8} hint={hatchText(e.doc.personal.egg_cycles)} />
        {tinkering && <PersonalSelect e={e} field="color" options={COLORS} />}
      </Column>
    </div>
  );
}
