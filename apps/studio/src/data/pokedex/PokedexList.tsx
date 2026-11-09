import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { DataRowDto, SpeciesRowExtra } from "../../api";
import { useReset } from "../../original/OriginalContext";
import { resetEntry } from "../../original/resetEntry";
import { FilterChips, pokemonTypes } from "../../ui/primitives";
import { typeName } from "../../ui/primitives/pokemonTypes";
import { DataList, RowIcon } from "../DataList";
import { monIcon, monIconKey } from "../icons";
import { useDataArea, useDataSearch } from "../useDataArea";
import { TAB_ASSETS } from "./pokedexModel";
import styles from "./Pokedex.module.css";

/** The menu icon of a species row. */
const speciesIcon = (row: DataRowDto, label: string) => <RowIcon iconKey={monIconKey(row.index)} load={() => monIcon(row.index)} label={label} area="pokemon" />;

/** The assets a species is made of: a row is unsaved when one of them is. */
const PARTS = new Set(TAB_ASSETS.raw);

const typesOf = (row: DataRowDto): readonly number[] => (row.extra as SpeciesRowExtra | null)?.types ?? [];

/** The list of the Pokédex: search by name or number, type chips with their counts, icon, name and number, and a dot on unsaved species. */
export function PokedexList() {
  const { t } = useTranslation("data");
  const { rows, dirty } = useDataArea("pokemon");
  const { filter, setFilter } = useDataSearch("pokemon");
  const options = useMemo(
    () =>
      pokemonTypes.map((type) => ({
        value: String(type.id),
        label: typeName(type.id),
        type: type.id,
        count: rows.filter((r) => typesOf(r).includes(type.id)).length,
      })),
    [rows, t],
  );
  const chosen = useMemo(() => new Set(filter.types.map(String)), [filter.types]);
  const onTypes = useCallback((next: Set<string>) => setFilter({ ...filter, types: [...next].map(Number) }), [filter, setFilter]);
  const unsaved = useMemo(() => new Set(dirty.filter((a) => PARTS.has(a.kind)).map((a) => a.index)), [dirty]);
  const trailing = useCallback(
    (row: DataRowDto) => (unsaved.has(row.index) ? <span className={styles.dot} role="img" aria-label={t("unsaved")} /> : null),
    [unsaved, t],
  );
  // A species with its data, learnset and evolution as in the base ROM.
  const { status, open } = useReset();
  const contextMenu = useCallback((row: DataRowDto) => [resetEntry({ kind: "species", index: row.index }, status, open)], [status, open]);
  return (
    <DataList
      contextMenu={contextMenu}
      area="pokemon"
      label={t("dataModel.sub.pokedex")}
      attr="data-species"
      leading={speciesIcon}
      trailing={trailing}
      filters={<FilterChips ariaLabel={t("type")} options={options} value={chosen} onChange={onTypes} />}
    />
  );
}
