import { useCallback, useMemo } from "react";
import type { DataRowDto, SpeciesRowExtra } from "../../api";
import { FilterChips, pokemonTypes } from "../../ui/primitives";
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
  const { rows, dirty } = useDataArea("pokemon");
  const { filter, setFilter } = useDataSearch("pokemon");
  const options = useMemo(
    () =>
      pokemonTypes.map((t) => ({
        value: String(t.id),
        label: t.name,
        type: t.id,
        count: rows.filter((r) => typesOf(r).includes(t.id)).length,
      })),
    [rows],
  );
  const chosen = useMemo(() => new Set(filter.types.map(String)), [filter.types]);
  const onTypes = useCallback((next: Set<string>) => setFilter({ ...filter, types: [...next].map(Number) }), [filter, setFilter]);
  const unsaved = useMemo(() => new Set(dirty.filter((a) => PARTS.has(a.kind)).map((a) => a.index)), [dirty]);
  const trailing = useCallback(
    (row: DataRowDto) => (unsaved.has(row.index) ? <span className={styles.dot} role="img" aria-label="ungespeichert" /> : null),
    [unsaved],
  );
  return (
    <DataList
      area="pokemon"
      label="Pokédex"
      attr="data-species"
      leading={speciesIcon}
      trailing={trailing}
      filters={<FilterChips ariaLabel="Typ" options={options} value={chosen} onChange={onTypes} />}
    />
  );
}
