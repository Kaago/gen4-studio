import type { AssetId, AssetKind, DataKind } from "../api";

/**
 * A data member may have changed behind the data areas (an edit, undo, redo, reload): the lists that show its kind are read again, an
 * editor showing it reads it again. `index` is the member (items: the data member, not the item id); null: any member of the kind.
 */
export type DataChangedEvent = { topic: "dataChanged"; kind: DataKind; index: number | null };

const KIND_OF: Partial<Record<AssetKind, DataKind>> = {
  Personal: "species",
  Learnset: "learnset",
  Evolution: "evolution",
  Move: "move",
  Item: "item",
  Encounter: "encounter",
  TrainerProperties: "trainer",
  TrainerParty: "trainer",
  TrainerTextTable: "trainer",
  TrainerTextOffsets: "trainer",
  Trainer: "trainer",
};

/** Kinds shared by every member of their data kind: a change may concern any member (the trainer text table holds every trainer's texts). */
const SHARED: ReadonlySet<AssetKind> = new Set<AssetKind>(["TrainerTextTable", "TrainerTextOffsets"]);

/** What `textData` says: a text archive (null: any) may have changed. */
export type TextDataEvent = { topic: "textData"; archive: number | null };

/**
 * The events a data edit that changed `asset` announces: `dataChanged` for a data member, `textData` for a text archive (a trainer's name
 * and battle texts live in text archives: the documents, lists and the text panel that show them read them again). None for other assets.
 */
export function editEventsOf(asset: AssetId): (DataChangedEvent | TextDataEvent)[] {
  if (asset.kind === "TextArchive") return [{ topic: "textData", archive: asset.index }];
  const event = dataChangedOf(asset);
  return event ? [event] : [];
}

/** The event for an asset that undo or redo restored; null for an asset that is no data member. */
export function dataChangedOf(asset: AssetId): DataChangedEvent | null {
  const kind = KIND_OF[asset.kind];
  if (kind === undefined) return null;
  return { topic: "dataChanged", kind, index: SHARED.has(asset.kind) ? null : asset.index };
}
