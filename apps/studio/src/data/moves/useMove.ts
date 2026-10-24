import { useCallback, useEffect, useRef, useState } from "react";
import { dataEdit, dataGet, errorText } from "../../api";
import type { DataDocDto, DataPatch } from "../../api";
import { dataChangedOf } from "../dataBus";
import { useDataArea } from "../useDataArea";

/** The document of a move: its fields with their widths, its description and why it is locked. */
export type MoveDoc = Extract<DataDocDto, { kind: "move" }>;

interface Loaded {
  index: number;
  doc: MoveDoc | null;
  error: string | null;
}

export interface Move {
  /** The document of the move asked for; null while it is read or when it could not be. */
  doc: MoveDoc | null;
  /** Why it could not be read. */
  error: string | null;
  loading: boolean;
  /** Why the last edit was refused (the backend's message); null after one that went through. */
  editError: string | null;
  /**
   * Writes one patch to move `index` as one undo step named `label`, after the edits before it. Then the list, the view and this document
   * read the move again (`dataChanged`) and the dirty count catches up. Resolves to whether it went through.
   */
  edit(index: number, patch: DataPatch, label: string): Promise<boolean>;
}

/**
 * Move `index` (null: none) through `data_get`, read again when it changed behind it (an edit, undo, redo, reload: `dataChanged`). A
 * read overtaken by a newer one does not deliver; while another move loads, the previous one is not shown as this one.
 */
export function useMove(index: number | null): Move {
  const { project, bus, onEdited } = useDataArea("pokemon");
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [editError, setEditError] = useState<string | null>(null);
  const [errorOf, setErrorOf] = useState(index);
  if (errorOf !== index) {
    // Another move: the refusal of the last one is moot.
    setErrorOf(index);
    setEditError(null);
  }

  const latestRead = useRef(0);
  const read = useCallback(async (i: number) => {
    const mine = ++latestRead.current;
    try {
      const doc = await dataGet("move", i);
      if (latestRead.current === mine) setLoaded({ index: i, doc: doc.kind === "move" ? doc : null, error: doc.kind === "move" ? null : "Die Antwort enthält keine Attacken-Daten." });
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
      if (event.topic === "dataChanged" && event.kind === "move" && (event.index === null || event.index === index)) void read(index);
    });
  }, [bus, index, read]);

  // The callbacks of the latest render, for edits that run after it.
  const latest = useRef({ project, bus, onEdited });
  latest.current = { project, bus, onEdited };
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  const edit = useCallback((target: number, patch: DataPatch, label: string): Promise<boolean> => {
    const owner = latest.current.project;
    const job = chain.current.then(async () => {
      // An edit that outlives its project is dropped.
      if (latest.current.project !== owner) return false;
      try {
        const changed = await dataEdit("move", target, patch, label);
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

  const current = loaded !== null && loaded.index === index ? loaded : null;
  return { doc: current?.doc ?? null, error: current?.error ?? null, loading: index !== null && current === null, editError, edit };
}
