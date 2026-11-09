import { useCallback, useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { DataRowDto, MoveRowExtra } from "../../api";
import { FilterChips, SelectField, TypeBadge, pokemonTypes } from "../../ui/primitives";
import { typeName } from "../../ui/primitives/pokemonTypes";
import type { SelectOption } from "../../ui/primitives";
import { DataList } from "../DataList";
import { categoryNames } from "../dataModel";
import { useDataArea, useDataSearch } from "../useDataArea";
import styles from "./Moves.module.css";

const extraOf = (row: DataRowDto) => row.extra as MoveRowExtra | null;

/** The type of a move row as a small badge; nothing for a locked row. */
function typeBadge(row: DataRowDto) {
  const type = extraOf(row)?.type;
  return type === undefined ? null : <TypeBadge type={type} size="s" />;
}

/** "Alle Kategorien" (no filter, -1) and the three categories. */

/**
 * The list of the moves: search by name or number, type chips with their counts, a category choice; a row shows the type, the name, the
 * number and a dot when the move is unsaved (a category word would cut the name short in the list's width).
 */
export function MovesList() {
  const { t } = useTranslation("data");
  const { rows, dirty } = useDataArea("pokemon");
  const { filter, setFilter } = useDataSearch("pokemon");
  const options = useMemo(
    () => pokemonTypes.map((type) => ({ value: String(type.id), label: typeName(type.id), type: type.id, count: rows.filter((r) => extraOf(r)?.type === type.id).length })),
    [rows, t],
  );
  const categories = useMemo<readonly SelectOption<number>[]>(() => [{ value: -1, label: t("movesList.allCategories") }, ...categoryNames().map((label, value) => ({ value, label }))], [t]);
  const chosen = useMemo(() => new Set(filter.types.map(String)), [filter.types]);
  const onTypes = useCallback((next: Set<string>) => setFilter({ ...filter, types: [...next].map(Number) }), [filter, setFilter]);
  const unsaved = useMemo(() => new Set(dirty.filter((a) => a.kind === "Move").map((a) => a.index)), [dirty]);
  const trailing = useCallback(
    (row: DataRowDto) => (unsaved.has(row.index) ? <span className={styles.dot} role="img" aria-label={t("unsaved")} /> : null),
    [unsaved, t],
  );
  return (
    <DataList
      area="pokemon"
      label={t("movesList.list")}
      attr="data-move"
      leading={typeBadge}
      trailing={trailing}
      filters={
        <>
          <FilterChips ariaLabel={t("type")} options={options} value={chosen} onChange={onTypes} />
          <SelectField label={t("movesList.category")} value={filter.category ?? -1} options={categories} onChange={(v) => setFilter({ ...filter, category: v < 0 ? null : v })} />
        </>
      }
    />
  );
}
