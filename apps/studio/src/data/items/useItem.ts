import type { DataDocDto, DataPatch } from "../../api";
import { useDataDoc } from "../useDataDoc";
import type { DataDoc, DocSpec } from "../useDataDoc";

/** The document of an item: its fields with their widths, its description, its name archive, the ids sharing its data, why it is locked. */
export type ItemDoc = Extract<DataDocDto, { kind: "item" }>;

/** `doc` with an `Item` patch written into its field. */
export function applyItem(doc: ItemDoc, patch: DataPatch): ItemDoc {
  if (!("Item" in patch)) return doc;
  const { field, value } = patch.Item;
  return { ...doc, fields: doc.fields.map((f) => (f.name === field ? { ...f, value } : f)) };
}

const ITEM: DocSpec<ItemDoc> = {
  area: "gegenstaende",
  kind: "item",
  parts: ["item"],
  accept: (doc) => (doc.kind === "item" ? doc : null),
  wrongKind: "Die Antwort enthält keine Gegenstands-Daten.",
  apply: applyItem,
  texts: (doc) => [doc.description?.archive ?? null, doc.name_archive],
};

/**
 * Item `index` (item id, or data member without the item table; null: none) with its queue of edits (`useDataDoc`), keyed by its data
 * `member`: an edit through any item id sharing the member reaches it, and the writes are laid over every id of that member.
 */
export const useItem = (index: number | null, member: number | null): DataDoc<ItemDoc> => useDataDoc(ITEM, index, member);
