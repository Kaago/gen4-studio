import { useCallback, useEffect, useReducer, useRef, useState } from "react";
import { dataEdit, dataGet, errorText } from "../../api";
import type { DataDocDto } from "../../api";
import { dataChangedOf } from "../dataBus";
import { useDataArea } from "../useDataArea";
import { itemValues } from "./itemModel";
import type { ItemField, ItemValues } from "./itemModel";

/** The document of an item: its fields with their widths, its description, the item ids sharing its data, why it is locked. */
export type ItemDoc = Extract<DataDocDto, { kind: "item" }>;

/**
 * One edit of one field, worked out when it runs: `next` gets the field's newest value (after every edit before it, even before the
 * document is read again) and gives the value to write; `label` names the step from the old and the new value.
 */
export interface ItemEdit {
  field: ItemField;
  next(old: number): number;
  label(old: number, value: number): string;
}

export interface Item {
  /** The document of the item asked for; null while it is read or when it could not be. */
  doc: ItemDoc | null;
  /**
   * The item's values as the editor shows them: the document's, with the edits written since it was read and the edits still waiting laid
   * over them (two quick switches both show at once). Null while there is no document.
   */
  values: ItemValues | null;
  /** Why it could not be read. */
  error: string | null;
  loading: boolean;
  /** Why the last edit was refused (the backend's message); null after one that went through. */
  editError: string | null;
  /**
   * Writes one field of item `index` as one undo step, after the edits before it, from the newest value of that field. Then the list, the
   * view and this document read the item again (`dataChanged`) and the dirty count catches up. Resolves to whether it went through.
   */
  edit(index: number, edit: ItemEdit): Promise<boolean>;
}

interface Loaded {
  index: number;
  doc: ItemDoc | null;
  error: string | null;
}

/** A write that went through: the item, the field and the value, numbered in the order they went through. */
interface Write {
  seq: number;
  index: number;
  field: ItemField;
  value: number;
}

/** An edit waiting in the queue (for the values shown until it ran). */
interface Waiting {
  index: number;
  edit: ItemEdit;
}

const lay = (values: ItemValues, field: ItemField, next: (old: number) => number): ItemValues => {
  const old = values[field];
  return old === undefined ? values : { ...values, [field]: next(old) };
};

/**
 * Item `index` (item id, or data member without the item table; null: none) through `data_get`, read again when its data member
 * `member` changed behind it (an edit through any item id sharing it, undo, redo, reload: `dataChanged`). A read overtaken by a newer one
 * does not deliver; while another item loads, the previous one is not shown as this one.
 *
 * Edits never build on a stale document: each one runs after the ones before it and takes the field's value from the newest state this
 * hook knows, which is the last document read plus the writes that went through after that read was asked for.
 */
export function useItem(index: number | null, member: number | null): Item {
  const { project, bus, onEdited } = useDataArea("gegenstaende");
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [editError, setEditError] = useState<string | null>(null);
  const [, redraw] = useReducer((n: number) => n + 1, 0);
  const [errorOf, setErrorOf] = useState(index);
  if (errorOf !== index) {
    // Another item: the refusal of the last one is moot.
    setErrorOf(index);
    setEditError(null);
  }

  // The newest values per item as far as this hook knows them, the writes since, and the edits not yet run.
  const known = useRef(new Map<number, ItemValues>());
  const writes = useRef<Write[]>([]);
  const written = useRef(0);
  const waiting = useRef<Waiting[]>([]);

  const latestRead = useRef(0);
  const read = useCallback(async (i: number) => {
    const mine = ++latestRead.current;
    // The writes that went through before this read was asked for are in its answer; the later ones are laid over it.
    const since = written.current;
    try {
      const doc = await dataGet("item", i);
      if (latestRead.current !== mine) return;
      if (doc.kind !== "item") {
        setLoaded({ index: i, doc: null, error: "Die Antwort enthält keine Gegenstands-Daten." });
        return;
      }
      let values = itemValues(doc.fields);
      for (const w of writes.current) if (w.seq > since && w.index === i) values = { ...values, [w.field]: w.value };
      // Every later read starts after `since`: the writes before it are in every answer from now on.
      writes.current = writes.current.filter((w) => w.seq > since);
      known.current.set(i, values);
      setLoaded({ index: i, doc, error: null });
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
      if (event.topic === "dataChanged" && event.kind === "item" && (event.index === null || event.index === member)) void read(index);
    });
  }, [bus, index, member, read]);

  // The callbacks of the latest render, for edits that run after it.
  const latest = useRef({ project, bus, onEdited });
  latest.current = { project, bus, onEdited };
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  const edit = useCallback((target: number, change: ItemEdit): Promise<boolean> => {
    const owner = latest.current.project;
    const queued: Waiting = { index: target, edit: change };
    waiting.current = [...waiting.current, queued];
    redraw();
    const job = chain.current.then(async () => {
      try {
        // An edit that outlives its project is dropped.
        const base = known.current.get(target);
        if (latest.current.project !== owner || base === undefined) return false;
        const old = base[change.field];
        if (old === undefined) return false;
        const value = change.next(old);
        if (value === old) return true;
        try {
          const changed = await dataEdit("item", target, { Item: { field: change.field, value } }, change.label(old, value));
          writes.current = [...writes.current, { seq: ++written.current, index: target, field: change.field, value }];
          known.current.set(target, { ...(known.current.get(target) ?? base), [change.field]: value });
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
      } finally {
        waiting.current = waiting.current.filter((w) => w !== queued);
        redraw();
      }
    });
    chain.current = job;
    return job;
  }, []);

  const current = loaded !== null && loaded.index === index ? loaded : null;
  const doc = current?.doc ?? null;
  let values: ItemValues | null = null;
  const newest = index === null ? undefined : known.current.get(index);
  if (doc !== null && newest !== undefined) {
    values = newest;
    for (const w of waiting.current) if (w.index === index) values = lay(values, w.edit.field, w.edit.next);
  }
  return { doc, values, error: current?.error ?? null, loading: index !== null && current === null, editError, edit };
}
