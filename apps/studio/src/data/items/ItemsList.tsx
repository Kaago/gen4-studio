import { useCallback, useMemo } from "react";
import type { DataRowDto, ItemEntryDto } from "../../api";
import { FilterChips, Notice } from "../../ui/primitives";
import { DataList, RowIcon } from "../DataList";
import { POCKET_NAMES, itemPocket, itemPrice } from "../dataModel";
import { itemIcon, itemIconKey } from "../icons";
import { useDataArea } from "../useDataArea";
import { dataMemberOf, hasIcon } from "./itemModel";
import styles from "./Items.module.css";

/** The bag icon of an item row, loaded once per icon and palette (`item:<icon>:<palette>`) when the row comes near the view. */
function ItemIcon({ row, label, entry }: { row: DataRowDto; label: string; entry: ItemEntryDto | undefined }) {
  const icon = hasIcon(row, entry);
  return <RowIcon iconKey={icon ? itemIconKey(entry) : null} load={() => (icon ? itemIcon(row.index, entry) : Promise.resolve(null))} label={label} area="gegenstaende" />;
}

/**
 * The list of the items by item id: search by name or number, pocket chips with their counts (one pocket at a time), a row shows the bag
 * icon from the ROM, the name, the number, the price on the right and a dot when its data is unsaved. Without the item table the rows are
 * the data members ("Daten 0017") without icons, and a note says why.
 */
export function ItemsList() {
  const { rows, filter, setFilter, dirty, itemTable } = useDataArea("gegenstaende");
  const options = useMemo(() => POCKET_NAMES.map((label, pocket) => ({ value: String(pocket), label, count: rows.filter((r) => itemPocket(r) === pocket).length })), [rows]);
  const chosen = useMemo(() => new Set(filter.pocket === null ? [] : [String(filter.pocket)]), [filter.pocket]);
  const onPockets = useCallback(
    (next: Set<string>) => {
      // A chip that was not on before is the new pocket; turning the one chosen off (or "Filter lösen") shows all pockets.
      const added = [...next].find((p) => !chosen.has(p));
      setFilter({ ...filter, pocket: added === undefined ? null : Number(added) });
    },
    [chosen, filter, setFilter],
  );
  const unsaved = useMemo(() => new Set(dirty.filter((a) => a.kind === "Item").map((a) => a.index)), [dirty]);
  const leading = useCallback((row: DataRowDto, label: string) => <ItemIcon row={row} label={label} entry={itemTable?.[row.index]} />, [itemTable]);
  const trailing = useCallback(
    (row: DataRowDto) => (
      <span className={styles.trailing}>
        <span className={styles.price}>{itemPrice(row)}</span>
        {unsaved.has(dataMemberOf(row)) && <span className={styles.dot} role="img" aria-label="ungespeichert" />}
      </span>
    ),
    [unsaved],
  );
  return (
    <DataList
      area="gegenstaende"
      label="Gegenstände"
      attr="data-item"
      leading={leading}
      trailing={trailing}
      filters={
        <>
          {itemTable === null && <Notice kind="info">Item-Tabelle nicht lesbar: Namen und Symbole fehlen</Notice>}
          <FilterChips ariaLabel="Taschen" options={options} value={chosen} onChange={onPockets} visible={POCKET_NAMES.length} />
        </>
      }
    />
  );
}
