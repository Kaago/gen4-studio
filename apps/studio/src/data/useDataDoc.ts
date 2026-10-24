import { useCallback, useEffect, useMemo, useReducer, useRef, useState } from "react";
import { dataEdit, dataGet, errorText } from "../api";
import type { DataDocDto, DataKind, DataPatch, ProjectInfo } from "../api";
import { dataChangedOf } from "./dataBus";
import type { DataArea } from "./dataModel";
import { useDataArea } from "./useDataArea";

/** A patch and its undo label, as one edit writes them. */
export interface DocEdit {
  patch: DataPatch;
  label: string;
}

/** What a document of one kind needs besides the generic part. */
export interface DocSpec<D> {
  /** The data area the editor lives in (its project, bus and `onEdited`). */
  area: DataArea;
  /** The kind `data_get` and `data_edit` take. */
  kind: DataKind;
  /** The kinds whose `dataChanged` mean this document changed (a species is made of three). */
  parts: readonly DataKind[];
  /** The document of the right kind, or null when the answer holds another. */
  accept(doc: DataDocDto): D | null;
  /** What the editor says when the answer holds another kind. */
  wrongKind: string;
  /**
   * `doc` with `patch` written, as the backend would hold it, for what the editor shows until the document is read again. Without it the
   * editor shows the last document read (a whole-part write the editor cannot model), and `make` never runs during a render.
   */
  apply?(doc: D, patch: DataPatch): D;
}

export interface DataDoc<D> {
  /**
   * The document as the editor shows it: the last one read, with the writes that went through after that read was asked for and the
   * edits still waiting laid over it (`apply`), so two quick changes both show at once. Null while it is read or when it could not be.
   * The same object from render to render until a read, a write or a queued edit changes it.
   */
  doc: D | null;
  /** The document as last read, without anything laid over it (a new object with every read: what follows the backend). */
  read: D | null;
  /** Why it could not be read. */
  error: string | null;
  loading: boolean;
  /** Why the last edit was refused (the backend's message); null after one that went through. */
  editError: string | null;
  /**
   * Writes one edit to member `index` as one undo step, after the edits before it. `make` builds the patch and its label when its turn
   * comes, from the member as the backend holds it then (read anew: every edit before it is in), never from the document on screen;
   * null writes nothing (a locked member, a value that is already there). Then the lists, the views and this document read the member
   * again (`dataChanged`) and the dirty count catches up. Resolves to whether it went through (nothing to write counts as through).
   *
   * With `apply`, `make` also runs during renders, to lay the waiting edit over the shown document: it must be pure and must not throw,
   * and it must name the result (set this bit, this value), not a step (flip, add one).
   */
  update(index: number, make: (doc: D) => DocEdit | null): Promise<boolean>;
}

interface Loaded<D> {
  index: number;
  doc: D | null;
  error: string | null;
  /** The writes numbered up to this one were in when the read was asked for. */
  since: number;
}

/**
 * A write that went through, numbered in order, by the data member it changed (items: several ids share one) and the project it
 * belongs to.
 */
interface Write {
  seq: number;
  member: number;
  project: ProjectInfo | null;
  patch: DataPatch;
}

/** An edit in the queue that has not written yet. */
interface Waiting<D> {
  member: number;
  project: ProjectInfo | null;
  make: (doc: D) => DocEdit | null;
}

/**
 * The document of member `index` (null: none) of `spec.kind` through `data_get`, read again when it changed behind the editor (an edit,
 * undo, redo, reload: `dataChanged` of one of `spec.parts` for `member`, which defaults to `index`; items pass their data member). A
 * read overtaken by a newer one does not deliver; while another member loads, the previous one is not shown as this one.
 *
 * Edits run one after another. Each builds its patch and label when it runs, from a fresh `data_get` of its member, so it builds on every
 * edit before it even while the shown document has not caught up; a refused edit leaves nothing behind for the next one to build on.
 * Writes and waiting edits are laid over the shown document by data member and only within their project.
 */
export function useDataDoc<D>(spec: DocSpec<D>, index: number | null, member: number | null = index): DataDoc<D> {
  const { project, bus, onEdited } = useDataArea(spec.area);
  const [loaded, setLoaded] = useState<Loaded<D> | null>(null);
  const [editError, setEditError] = useState<string | null>(null);
  const [version, redraw] = useReducer((n: number) => n + 1, 0);
  const [errorOf, setErrorOf] = useState(index);
  if (errorOf !== index) {
    // Another member: the refusal of the last one is moot.
    setErrorOf(index);
    setEditError(null);
  }

  // The spec of the latest render (callers may build it inline).
  const specRef = useRef(spec);
  specRef.current = spec;
  const writes = useRef<Write[]>([]);
  const written = useRef(0);
  const waiting = useRef<Waiting<D>[]>([]);

  const latestRead = useRef(0);
  const read = useCallback(async (i: number) => {
    const mine = ++latestRead.current;
    // The writes that went through before this read was asked for are in its answer; the later ones are laid over it.
    const since = written.current;
    const { kind, accept, wrongKind } = specRef.current;
    try {
      const answer = await dataGet(kind, i);
      if (latestRead.current !== mine) return;
      const doc = accept(answer);
      // Every later read starts after `since`: the writes before it are in every answer from now on.
      writes.current = writes.current.filter((w) => w.seq > since);
      setLoaded({ index: i, doc, error: doc === null ? wrongKind : null, since });
    } catch (e) {
      if (latestRead.current === mine) setLoaded({ index: i, doc: null, error: errorText(e), since });
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
      if (event.topic === "dataChanged" && specRef.current.parts.includes(event.kind) && (event.index === null || event.index === member)) void read(index);
    });
  }, [bus, index, member, read]);

  // The callbacks and the member of the latest render, for edits that run after it.
  const latest = useRef({ project, bus, onEdited, index, member });
  latest.current = { project, bus, onEdited, index, member };
  const chain = useRef<Promise<unknown>>(Promise.resolve());
  const update = useCallback((target: number, make: (doc: D) => DocEdit | null): Promise<boolean> => {
    const owner = latest.current.project;
    // The data member the edit writes: the shown one's for the shown index, else the index itself.
    const targetMember = target === latest.current.index && latest.current.member !== null ? latest.current.member : target;
    const queued: Waiting<D> = { member: targetMember, project: owner, make };
    waiting.current = [...waiting.current, queued];
    redraw();
    const job = chain.current.then(async () => {
      try {
        // An edit that outlives its project is dropped, also when the project changed while it read the member.
        if (latest.current.project !== owner) return false;
        const { kind, accept, wrongKind } = specRef.current;
        const now = accept(await dataGet(kind, target));
        if (latest.current.project !== owner) return false;
        if (now === null) throw new Error(wrongKind);
        const made = make(now);
        if (made === null) return true;
        const changed = await dataEdit(kind, target, made.patch, made.label);
        writes.current = [...writes.current, { seq: ++written.current, member: targetMember, project: owner, patch: made.patch }];
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
      } finally {
        waiting.current = waiting.current.filter((w) => w !== queued);
        redraw();
      }
    });
    chain.current = job;
    return job;
  }, []);

  const current = loaded !== null && loaded.index === index ? loaded : null;
  const lastRead = current?.doc ?? null;
  const since = current?.since ?? 0;
  const apply = spec.apply;
  // The document read, with the writes after its read and the waiting edits of its data member and project laid over it; recomputed
  // only when one of them changed (`version` counts the queue's changes), so the shown document keeps its identity in between.
  const shown = useMemo(() => {
    if (lastRead === null || !apply || member === null) return lastRead;
    let doc: D = lastRead;
    for (const w of writes.current) if (w.seq > since && w.member === member && w.project === project) doc = apply(doc, w.patch);
    for (const w of waiting.current) {
      if (w.member !== member || w.project !== project) continue;
      const made = w.make(doc);
      if (made !== null) doc = apply(doc, made.patch);
    }
    return doc;
    // `version` stands for the refs `writes` and `waiting`.
  }, [lastRead, since, apply, member, project, version]);
  return { doc: shown, read: lastRead, error: current?.error ?? null, loading: index !== null && current === null, editError, update };
}
