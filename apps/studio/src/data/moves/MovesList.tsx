import { useCallback, useMemo } from "react";
import type { DataRowDto, MoveRowExtra } from "../../api";
import { FilterChips, SelectField, TypeBadge, pokemonTypes } from "../../ui/primitives";
import type { SelectOption } from "../../ui/primitives";
import { DataList } from "../DataList";
import { CATEGORY_NAMES } from "../dataModel";
import { useDataArea } from "../useDataArea";
import styles from "./Moves.module.css";

const extraOf = (row: DataRowDto) => row.extra as MoveRowExtra | null;

/** The type of a move row as a small badge; nothing for a locked row. */
function typeBadge(row: DataRowDto) {
  const type = extraOf(row)?.type;
  return type === undefined ? null : <TypeBadge type={type} size="s" />;
}

/** "Alle Kategorien" (no filter, -1) and the three categories. */
const CATEGORY_FILTER: readonly SelectOption<number>[] = [{ value: -1, label: "Alle Kategorien" }, ...CATEGORY_NAMES.map((label, value) => ({ value, label }))];

/**
 * The list of the moves: search by name or number, type chips with their counts, a category choice; a row shows the type, the name, the
 * number and a dot when the move is unsaved (a category word would cut the name short in the list's width).
 */
export function MovesList() {
  const { rows, filter, setFilter, dirty } = useDataArea("pokemon");
  const options = useMemo(
    () => pokemonTypes.map((t) => ({ value: String(t.id), label: t.name, type: t.id, count: rows.filter((r) => extraOf(r)?.type === t.id).length })),
    [rows],
  );
  const chosen = useMemo(() => new Set(filter.types.map(String)), [filter.types]);
  const onTypes = useCallback((next: Set<string>) => setFilter({ ...filter, types: [...next].map(Number) }), [filter, setFilter]);
  const unsaved = useMemo(() => new Set(dirty.filter((a) => a.kind === "Move").map((a) => a.index)), [dirty]);
  const trailing = useCallback(
    (row: DataRowDto) => (unsaved.has(row.index) ? <span className={styles.dot} role="img" aria-label="ungespeichert" /> : null),
    [unsaved],
  );
  return (
    <DataList
      area="pokemon"
      label="Attacken"
      attr="data-move"
      leading={typeBadge}
      trailing={trailing}
      filters={
        <>
          <FilterChips ariaLabel="Typ" options={options} value={chosen} onChange={onTypes} />
          <SelectField label="Kategorie" value={filter.category ?? -1} options={CATEGORY_FILTER} onChange={(v) => setFilter({ ...filter, category: v < 0 ? null : v })} />
        </>
      }
    />
  );
}
