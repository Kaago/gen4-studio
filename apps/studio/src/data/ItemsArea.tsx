import { memo } from "react";
import type { ReactNode } from "react";
import { dataStatus } from "../app/statusContext";
import { DataListPane, DataPanel } from "./DataParts";
import type { DataPart } from "./DataParts";
import type { DataSub } from "./dataModel";
import { ItemHand } from "./items/ItemHand";
import { ItemsList } from "./items/ItemsList";
import { ItemView } from "./items/ItemView";
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

/** The part of the area: the items editor (list, view and hand in `data/items/`). */
export const ITEM_PARTS: Partial<Record<DataSub, DataPart>> = {
  items: { List: ItemsList, View: ItemView, Hand: ItemHand },
};

/** The list column of the Gegenstände area. */
export function ItemsAreaList({ open, onToggle }: { open: boolean; onToggle: () => void }) {
  return <DataListPane area="gegenstaende" parts={ITEM_PARTS} open={open} onToggle={onToggle} />;
}

/** The view (Schau) of the Gegenstände area: the chosen item. */
export function ItemsView() {
  return <DataPanel area="gegenstaende" parts={ITEM_PARTS} place="View" />;
}

/** The hand of the Gegenstände area: the editor of the chosen item. */
export function ItemsHand() {
  return <DataPanel area="gegenstaende" parts={ITEM_PARTS} place="Hand" />;
}

/** The middle of the status line while Gegenstände shows: "Trank, 0017". */
export function ItemsStatus() {
  const { row, labelOf } = useDataArea("gegenstaende");
  return <>{dataStatus(row === null ? null : { label: labelOf(row), tech: row.tech })}</>;
}
