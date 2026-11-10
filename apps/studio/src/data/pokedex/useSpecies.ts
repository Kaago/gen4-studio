import { useCallback } from "react";
import type { DataDocDto, DataPatch } from "../../api";
import { t } from "../../i18n";
import { useDataDoc } from "../useDataDoc";
import type { DocEdit, DocSpec } from "../useDataDoc";

/** The document of a species: personal data (and as last saved), learnset, evolutions, TM/HM bits, and why a part is locked. */
export type SpeciesDoc = Extract<DataDocDto, { kind: "species" }>;

/** A patch and its undo label. */
export type SpeciesEdit = DocEdit;

export interface Species {
  /** The document of the species asked for; null while it is read or when it could not be. */
  doc: SpeciesDoc | null;
  /** Why it could not be read. */
  error: string | null;
  loading: boolean;
  /** Why the last edit was refused (the backend's message); null after one that went through. */
  editError: string | null;
  /**
   * Writes one patch to species `index` as one undo step named `label`, after the edits before it. Then the lists, the view and this
   * document read the changed parts again (`dataChanged`) and the dirty count catches up. Resolves to whether it went through.
   */
  edit(index: number, patch: DataPatch, label: string): Promise<boolean>;
  /**
   * Like `edit`, for a patch that depends on what the species holds (a learnset or an evolution entry is written whole): `make` builds it
   * when its turn comes, from the species as the edits before it left it (read anew then), not from the document on screen, which may
   * not have caught up yet. `make` returning null writes nothing.
   */
  update(index: number, make: (doc: SpeciesDoc) => SpeciesEdit | null): Promise<boolean>;
}

/**
 * A species is made of its personal data, learnset and evolutions: a change to one of them is a change to the document. Its egg moves
 * live in one table for every species (`EggMoves 0`): a change to it announces `species` for any member (`dataBus`, index null), which
 * reads the shown species again whichever one was edited (undo of an edit made on another species). No `apply`: the
 * Pokédex shows the document as last read (its learnset and evolution drafts follow `read`), so `make` never runs during a render.
 */
const SPECIES: DocSpec<SpeciesDoc> = {
  area: "pokemon",
  kind: "species",
  parts: ["species", "learnset", "evolution"],
  accept: (doc) => (doc.kind === "species" ? doc : null),
  get wrongKind() {
    return t("data:pokedex.wrongKind");
  },
};

/**
 * Species `index` of the Pokédex (null: none) through the shared document queue (`useDataDoc`): read again when one of its parts changed
 * behind it, every edit built and written after the edits before it.
 */
export function useSpecies(index: number | null): Species {
  const { doc, error, loading, editError, update } = useDataDoc(SPECIES, index);
  const edit = useCallback((target: number, patch: DataPatch, label: string) => update(target, () => ({ patch, label })), [update]);
  return { doc, error, loading, editError, edit, update };
}
