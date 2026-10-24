import { useCallback, useEffect, useRef, useState } from "react";
import { dataEdit, dataGet, errorText } from "../../api";
import type { DataDocDto, DataKind, DataPatch } from "../../api";
import { dataChangedOf } from "../dataBus";
import { useDataArea } from "../useDataArea";

/** The document of a species: personal data (and as last saved), learnset, evolutions, TM/HM bits, and why a part is locked. */
export type SpeciesDoc = Extract<DataDocDto, { kind: "species" }>;

/** The kinds a species document is made of: a change to one of them is a change to the document. */
const PARTS: readonly DataKind[] = ["species", "learnset", "evolution"];

interface Loaded {
  index: number;
  doc: SpeciesDoc | null;
  error: string | null;
}

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

/** A patch and its undo label. */
export interface SpeciesEdit {
  patch: DataPatch;
  label: string;
}

/**
 * Species `index` of the Pokédex (null: none) through `data_get`, read again when one of its parts changed behind it (an edit, undo,
 * redo, reload: `dataChanged`). A read overtaken by a newer one does not deliver; while another species loads, the previous one is not
 * shown as this one.
 */
export function useSpecies(index: number | null): Species {
  const { project, bus, onEdited } = useDataArea("pokemon");
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [editError, setEditError] = useState<string | null>(null);
  const [errorOf, setErrorOf] = useState(index);
  if (errorOf !== index) {
    // Another species: the refusal of the last one is moot.
    setErrorOf(index);
    setEditError(null);
  }

  const latestRead = useRef(0);
  const read = useCallback(async (i: number) => {
    const mine = ++latestRead.current;
    try {
      const doc = await dataGet("species", i);
      if (latestRead.current === mine) setLoaded({ index: i, doc: doc.kind === "species" ? doc : null, error: doc.kind === "species" ? null : "Die Antwort enthält keine Pokémon-Daten." });
    } catch (e) {
      if (latestRead.current === mine) setLoaded({ index: i, doc: null, error: errorText(e) });
    }
  }, []);

  useEffect(() => {
    if (index === null || project === null) {
      latestRead.current++;
      setLoaded(null);
      return;
    }
    void read(index);
  }, [index, project, read]);

  useEffect(() => {
    if (index === null) return;
    return bus.subscribe("dataChanged", (event) => {
      if (event.topic === "dataChanged" && PARTS.includes(event.kind) && (event.index === null || event.index === index)) void read(index);
    });
  }, [bus, index, read]);

  // The callbacks of the latest render, for edits that run after it.
  const latest = useRef({ project, bus, onEdited });
  latest.current = { project, bus, onEdited };
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  // `make` gets the species as the backend holds it now (`fresh`), or nothing for a patch that stands on its own.
  const write = useCallback((target: number, fresh: boolean, make: (doc: SpeciesDoc | null) => SpeciesEdit | null): Promise<boolean> => {
    const owner = latest.current.project;
    const job = chain.current.then(async () => {
      // An edit that outlives its project is dropped.
      if (latest.current.project !== owner) return false;
      try {
        let now: SpeciesDoc | null = null;
        if (fresh) {
          const doc = await dataGet("species", target);
          if (doc.kind !== "species") throw new Error("Die Antwort enthält keine Pokémon-Daten.");
          now = doc;
        }
        const made = make(now);
        if (made === null) return false;
        const { patch, label } = made;
        const changed = await dataEdit("species", target, patch, label);
        setEditError(null);
        if (changed.length > 0) {
          for (const asset of changed) {
            const event = dataChangedOf(asset);
            if (event) latest.current.bus.emit(event);
          }
          latest.current.onEdited();
        }
        return true;
      } catch (e) {
        setEditError(errorText(e));
        return false;
      }
    });
    chain.current = job;
    return job;
  }, []);
  const edit = useCallback((target: number, patch: DataPatch, label: string) => write(target, false, () => ({ patch, label })), [write]);
  const update = useCallback((target: number, make: (doc: SpeciesDoc) => SpeciesEdit | null) => write(target, true, (doc) => (doc === null ? null : make(doc))), [write]);

  const current = loaded !== null && loaded.index === index ? loaded : null;
  return { doc: current?.doc ?? null, error: current?.error ?? null, loading: index !== null && current === null, editError, edit, update };
}
