import { id4 } from "../../workbench/terms";
import { useEffect, useMemo, useState } from "react";
import type { ReactNode } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import type { DataRowDto, ItemEntryDto, MoveRowExtra, SpeciesRowExtra } from "../../api";
import { Button, Picker, TypeBadge } from "../../ui/primitives";
import type { PickerItem } from "../../ui/primitives";
import { RowIcon } from "../DataList";
import { categoryNameOf, rowLabel } from "../dataModel";
import { useDataSnapshot } from "../dataStore";
import { itemIcon, itemIconKey, monIcon, monIconKey } from "../icons";
import { useDataArea } from "../useDataArea";
import { abilityName, itemName, moveName } from "./pokedexModel";
import styles from "./Pokedex.module.css";

/** What a pick field offers: the species, the moves, the items or the abilities of the project. */
export type Choice = "species" | "move" | "item" | "ability";

export interface PickFieldProps {
  /** The field's name: the picker's title („Fähigkeit 1 wählen“) and the button's accessible name with the current value. */
  label: string;
  /** The current value in words. */
  text: string;
  choice: Choice;
  /** Called with the number picked (species member, move, item id, ability). */
  onPick: (value: number) => void;
  disabled?: boolean;
  /** The species to offer (all without it); keep its identity between renders. */
  only?: (index: number) => boolean;
  /** The button's text when the value shows beside it (a reference to the target): „Ändern …“. */
  trigger?: string;
}

/** A value chosen from a long list: a button with the current value that opens a picker over the project's entries. */
export function PickField({ label, text, choice, onPick, disabled, only, trigger }: PickFieldProps) {
  const { t } = useTranslation("data");
  const [open, setOpen] = useState(false);
  const items = useChoices(choice, open, only);
  return (
    <>
      <Button kind={trigger ? "quiet" : "secondary"} size="s" aria-label={t("pickers.pickAria", { label, current: text })} disabled={disabled} onClick={() => setOpen(true)}>
        <span className={styles.pickText}>{trigger ?? text}</span>
      </Button>
      <ChoicePicker open={open} onOpenChange={setOpen} title={t("pickers.pickTitle", { label })} items={items} onPick={onPick} />
    </>
  );
}

/** The picker of a pick field, or of a button that picks once („Zeile hinzufügen“, „Von anderer Art übernehmen …“). */
export function ChoicePicker({ open, onOpenChange, title, items, onPick, footer }: { open: boolean; onOpenChange: (open: boolean) => void; title: string; items: readonly PickerItem[]; onPick: (value: number) => void; footer?: ReactNode }) {
  const { t } = useTranslation("data");
  return <Picker open={open} onOpenChange={onOpenChange} title={title} items={items} placeholder={t("pickers.searchPlaceholder")} footer={footer} onPick={(item) => onPick(Number(item.id))} />;
}

const NONE: readonly PickerItem[] = [];

/**
 * The entries of `choice` while `open` (built only then): species with their types, moves with type and category, items with their bag
 * icon (with the item table), abilities by name. The moves and items lists are read from the store the first time they are offered.
 */
export function useChoices(choice: Choice, open: boolean, only?: (index: number) => boolean): readonly PickerItem[] {
  const { t } = useTranslation("data");
  const { project, store, kind, rows, labelOf, names, itemTable } = useDataArea("pokemon");
  const snapshot = useDataSnapshot(store);
  // The Pokédex offers its own rows; another part (a trainer's team) reads the species from the store.
  const ownSpecies = kind === "species";
  useEffect(() => {
    if (!open || project === null) return;
    if (choice === "move") store.need(project, "move");
    else if (choice === "item") store.need(project, "item");
    else if (choice === "species" && !ownSpecies) store.need(project, "species");
  }, [open, project, choice, store, ownSpecies]);
  const moves = snapshot.project === project ? snapshot.rows.move : undefined;
  const species = ownSpecies ? rows : snapshot.project === project ? snapshot.rows.species : undefined;
  return useMemo(() => {
    if (!open) return NONE;
    switch (choice) {
      case "species": {
        const all = species ?? [];
        const name = (r: DataRowDto) => (ownSpecies ? labelOf(r) : rowLabel("species", r, true));
        return (only ? all.filter((r) => only(r.index)) : all).map((r) => speciesItem(r, name(r)));
      }
      case "move":
        // Move 0 is no move: a learnset or an evolution cannot name it.
        return moves
          ? moves.filter((r) => r.index !== 0).map((r) => moveItem(r, moveName(names.moves, r.index), t))
          : names.moves.flatMap((_, id) => (id === 0 ? [] : [{ id, label: moveName(names.moves, id), tech: id4(id) }]));
      case "item":
        return names.items.map((_, id) => itemItem(id, itemName(names.items, id), itemTable?.[id]));
      case "ability":
        return names.abilities.map((_, id) => ({ id, label: abilityName(names.abilities, id), tech: id4(id) }));
    }
  }, [open, choice, species, ownSpecies, labelOf, only, moves, names, itemTable, t]);
}

/** The types of a species row as badges. */
function typeBadges(row: DataRowDto): ReactNode {
  const extra = row.extra as SpeciesRowExtra | null;
  if (!extra || !Array.isArray(extra.types)) return null;
  const [a, b] = extra.types;
  return (
    <span className={styles.badges}>
      <TypeBadge type={a} size="s" />
      {b !== a && <TypeBadge type={b} size="s" />}
    </span>
  );
}

const speciesItem = (row: DataRowDto, label: string): PickerItem => ({
  id: row.index,
  label,
  tech: row.tech,
  preview: (
    <span className={styles.preview}>
      <RowIcon iconKey={monIconKey(row.index)} load={() => monIcon(row.index)} label={label} area="pokemon" />
      {typeBadges(row)}
    </span>
  ),
});

function moveItem(row: DataRowDto, label: string, t: TFunction<"data">): PickerItem {
  const extra = row.extra as MoveRowExtra | null;
  if (!extra || typeof extra.type !== "number") return { id: row.index, label, tech: row.tech };
  const category = categoryNameOf(extra.category) ?? t("dataModel.filterCategory", { name: extra.category });
  return {
    id: row.index,
    label,
    tech: row.tech,
    description: category,
    preview: (
      <span className={styles.preview}>
        <TypeBadge type={extra.type} size="s" />
      </span>
    ),
  };
}

const itemItem = (id: number, label: string, entry: ItemEntryDto | undefined): PickerItem => ({
  id,
  label,
  tech: id4(id),
  preview: (
    <span className={styles.preview}>
      <RowIcon iconKey={entry ? itemIconKey(entry) : null} load={() => (entry ? itemIcon(id, entry) : Promise.resolve(null))} label={label} area="gegenstaende" />
    </span>
  ),
});
