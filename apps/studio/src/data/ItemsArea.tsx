import { memo } from "react";
import type { ReactNode } from "react";
import type { DataRowDto, ItemEntryDto, ItemRowExtra } from "../api";
import { dataStatus } from "../app/statusContext";
import { DataList, RowIcon } from "./DataList";
import { DataListPane, DataPanel, chosenHead } from "./DataParts";
import type { DataPart } from "./DataParts";
import type { DataSub } from "./dataModel";
import { itemIcon, itemIconKey } from "./icons";
import { dataAreaProvider, useDataAreaModel, useDataArea } from "./useDataArea";
import type { DataAreaProps } from "./useDataArea";

const Provider = dataAreaProvider("gegenstaende");

/**
 * The Gegenstände area's model for its list, view, hand and status parts (`useDataArea("gegenstaende")`). Memoised: it renders when
 * `context` changes or its children do. It has one part (shops come later), so no switcher.
 */
export const ItemsArea = memo(function ItemsArea({ context, children }: { context: DataAreaProps; children?: ReactNode }) {
  const model = useDataAreaModel("gegenstaende", context);
  return <Provider value={model}>{children}</Provider>;
});

/** The bag icon of an item row: only with the item table, and only where the icon archive has the icon. */
function ItemIcon({ row, label }: { row: DataRowDto; label: string }) {
  const { itemTable } = useDataArea("gegenstaende");
  const entry: ItemEntryDto | undefined = itemTable?.[row.index];
  const hasIcon = entry !== undefined && (row.extra as ItemRowExtra | null)?.icon === true;
  return <RowIcon iconKey={hasIcon ? itemIconKey(entry) : null} load={() => (hasIcon ? itemIcon(row.index, entry) : Promise.resolve(null))} label={label} area="gegenstaende" />;
}

const itemLeading = (row: DataRowDto, label: string) => <ItemIcon row={row} label={label} />;

function ItemList() {
  return <DataList area="gegenstaende" label="Gegenstände" attr="data-item" leading={itemLeading} />;
}

const Chosen = chosenHead("gegenstaende");

/** The part of the area; the mount point of the items editor (Task 16 replaces the entry). */
export const ITEM_PARTS: Partial<Record<DataSub, DataPart>> = {
  items: { List: ItemList, View: Chosen, Hand: Chosen },
};

/** The list column of the Gegenstände area. */
export function ItemsList({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  return <DataListPane area="gegenstaende" parts={ITEM_PARTS} open={open} onToggle={onToggle} />;
}

/** The view (Schau) of the Gegenstände area: the chosen item. */
export function ItemView() {
  return <DataPanel area="gegenstaende" parts={ITEM_PARTS} place="View" />;
}

/** The hand of the Gegenstände area: the editor of the chosen item. */
export function ItemHand() {
  return <DataPanel area="gegenstaende" parts={ITEM_PARTS} place="Hand" />;
}

/** The middle of the status line while Gegenstände shows: "Trank, 0017". */
export function ItemsStatus() {
  const { row, labelOf } = useDataArea("gegenstaende");
  return <>{dataStatus(row === null ? null : { label: labelOf(row), tech: row.tech })}</>;
}
