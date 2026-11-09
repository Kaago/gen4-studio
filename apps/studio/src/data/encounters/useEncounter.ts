import { t } from "../../i18n";
import { useDataDoc } from "../useDataDoc";
import type { DataDoc, DocSpec } from "../useDataDoc";
import { applyEncounter } from "./encounterModel";
import type { EncounterDoc } from "./encounterModel";

/**
 * An encounter file is one member: a `dataChanged` of it (an edit, undo, redo; any file after a header edit, a new file or a split) reads it
 * again. It shows the names of the places that use it, which live in the location name bank: it names no archive (`texts` empty), so every
 * `textData` reads it again. Its rule warnings show in the view and the hand; the Problems list gets them from the checks (`encounter.*`).
 */
const ENCOUNTER: DocSpec<EncounterDoc> = {
  area: "pokemon",
  kind: "encounter",
  parts: ["encounter"],
  accept: (doc) => (doc.kind === "encounter" ? doc : null),
  get wrongKind() {
    return t("data:encounters.wrongKind");
  },
  apply: applyEncounter,
  texts: () => [],
};

/** Encounter file `index` (null: none) with its queue of edits (`useDataDoc`). */
export const useEncounter = (index: number | null): DataDoc<EncounterDoc> => useDataDoc(ENCOUNTER, index);
