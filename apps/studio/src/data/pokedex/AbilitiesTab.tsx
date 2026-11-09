import { useTranslation } from "react-i18next";
import { fieldLabels } from "../../api";
import { InspectorRow } from "../../ui/primitives";
import { TYPE_OPTIONS } from "../dataModel";
import { PickField } from "./pickers";
import type { Choice } from "./pickers";
import type { Editing, NumericField } from "./PokedexHand";
import { abilityName, itemName } from "./pokedexModel";
import { Column, PersonalSelect } from "./ValuesTab";
import styles from "./Pokedex.module.css";

/** A personal field chosen from a long list (an ability, an item). */
function PersonalPick({ e, field, choice, text }: { e: Editing; field: NumericField; choice: Choice; text: string }) {
  const label = fieldLabels.species[field];
  return (
    <InspectorRow label={label} deviates={e.doc.personal[field] !== e.doc.saved[field]}>
      <div data-field={field}>
        <PickField label={label} text={text} choice={choice} onPick={(v) => e.setPersonal(field, v)} disabled={e.locked.personal} />
      </div>
    </InspectorRow>
  );
}

/** Fähigkeiten und Typen: the two abilities, the two types and the items wild ones hold. */
export function AbilitiesTab({ e }: { e: Editing }) {
  const { t } = useTranslation("data");
  const p = e.doc.personal;
  return (
    <div className={styles.columns}>
      <Column title={t("abilitiesTab.abilities")} note={t("abilitiesTab.abilitiesNote")}>
        <PersonalPick e={e} field="ability1" choice="ability" text={abilityName(e.names.abilities, p.ability1)} />
        <PersonalPick e={e} field="ability2" choice="ability" text={abilityName(e.names.abilities, p.ability2)} />
      </Column>
      <Column title={t("abilitiesTab.types")} note={t("abilitiesTab.typesNote")}>
        <PersonalSelect e={e} field="type1" options={TYPE_OPTIONS} />
        <PersonalSelect e={e} field="type2" options={TYPE_OPTIONS} />
      </Column>
      <Column title={t("abilitiesTab.heldInWild")} note={t("abilitiesTab.heldInWildNote")}>
        <PersonalPick e={e} field="item1" choice="item" text={itemName(e.names.items, p.item1)} />
        <PersonalPick e={e} field="item2" choice="item" text={itemName(e.names.items, p.item2)} />
      </Column>
    </div>
  );
}
