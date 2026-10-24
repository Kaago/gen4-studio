import type { DataDocDto, DataPatch } from "../../api";
import { useDataDoc } from "../useDataDoc";
import type { DataDoc, DocSpec } from "../useDataDoc";

/** The document of a move: its fields with their widths, its description, its name archive and why it is locked. */
export type MoveDoc = Extract<DataDocDto, { kind: "move" }>;

/** `doc` with a `Move` patch written into its field. */
export function applyMove(doc: MoveDoc, patch: DataPatch): MoveDoc {
  if (!("Move" in patch)) return doc;
  const { field, value } = patch.Move;
  return { ...doc, fields: doc.fields.map((f) => (f.name === field ? { ...f, value } : f)) };
}

const MOVE: DocSpec<MoveDoc> = {
  area: "pokemon",
  kind: "move",
  parts: ["move"],
  accept: (doc) => (doc.kind === "move" ? doc : null),
  wrongKind: "Die Antwort enthält keine Attacken-Daten.",
  apply: applyMove,
  texts: (doc) => [doc.description?.archive ?? null, doc.name_archive],
};

/** Move `index` (null: none) of the Pokémon area, with its queue of edits (`useDataDoc`). */
export const useMove = (index: number | null): DataDoc<MoveDoc> => useDataDoc(MOVE, index);
