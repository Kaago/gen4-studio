import { useCallback, useEffect, useRef, useState, useSyncExternalStore } from "react";
import { errorText, listFlags } from "../api";
import type { FlagKind, FlagListDto, ProjectInfo } from "../api";

/**
 * The read of each kind per project, revision source and revision, shared by the fields that ask for it: a level script with ten
 * conditions reads the variables once. Only the latest revision of a kind and source is kept; a refused read is dropped, so the next one
 * asks again. Revisions of different sources (the events' bus counter, the scripts' counter) never share a read.
 */
let reads = new WeakMap<ProjectInfo, Map<string, { revision: number; list: Promise<FlagListDto> }>>();
/** Bumped by `invalidateFlagLists`: every list on screen is read again. */
let generation = 0;
const listeners = new Set<() => void>();

/**
 * Forgets every shared read and reads the lists on screen again: after something no revision tells of (an own name, `name_flag`), and
 * between tests.
 */
export function invalidateFlagLists(): void {
  reads = new WeakMap();
  generation += 1;
  for (const listener of listeners) listener();
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => void listeners.delete(listener);
}

const currentGeneration = () => generation;

function read(project: ProjectInfo, kind: FlagKind, source: string, revision: number, fresh: boolean): Promise<FlagListDto> {
  let held = reads.get(project);
  if (held === undefined) reads.set(project, (held = new Map()));
  const kinds = held;
  const slot = `${kind}:${source}`;
  const last = kinds.get(slot);
  if (!fresh && last !== undefined && last.revision === revision) return last.list;
  const entry = { revision, list: listFlags(kind) };
  kinds.set(slot, entry);
  entry.list.catch(() => {
    if (kinds.get(slot) === entry) kinds.delete(slot);
  });
  return entry.list;
}

type Shown = { project: ProjectInfo; kind: FlagKind; list: FlagListDto | null; error: string | null };

export interface FlagListDeps {
  project: ProjectInfo | null;
  revision: number;
  /** What `revision` counts (`events`, `scripts`): reads of different sources are not shared. */
  source?: string;
  /**
   * Read anew every time a project is given (a picker opening: `project` null while closed), never from the shared read, and show nothing
   * older meanwhile: the pool and the lock of „Freies Flag nehmen“ are then always current.
   */
  fresh?: boolean;
}

/**
 * The list of flags or variables (`kind`) of the open project, read when there is a project and again for every new `revision` (an event,
 * a script changed) and after `invalidateFlagLists`; `reload` reads it again without one. While a newer revision is read the older list
 * stays (not with `fresh`); an answer that a newer read overtook never shows, nor a list of another project or kind. `error` is the
 * refusal's sentence.
 */
export function useFlagList(kind: FlagKind, deps: FlagListDeps): { list: FlagListDto | null; error: string | null; reload: () => void } {
  const { project, revision, source = "", fresh = false } = deps;
  const gen = useSyncExternalStore(subscribe, currentGeneration);
  const [shown, setShown] = useState<Shown | null>(null);
  const [reloads, setReloads] = useState(0);
  // A fresh list forgets what it showed as soon as the project is asked for again (the picker opens).
  const [asked, setAsked] = useState(project);
  if (asked !== project) {
    setAsked(project);
    if (fresh) setShown(null);
  }
  // Set by `reload`: the next read bypasses the shared one.
  const bypass = useRef(false);
  useEffect(() => {
    if (project === null) return;
    let live = true;
    const anew = fresh || bypass.current;
    bypass.current = false;
    read(project, kind, source, revision, anew).then(
      (list) => live && setShown({ project, kind, list, error: null }),
      (e: unknown) => live && setShown({ project, kind, list: null, error: errorText(e) }),
    );
    return () => {
      live = false;
    };
  }, [project, kind, source, revision, fresh, reloads, gen]);
  const reload = useCallback(() => {
    bypass.current = true;
    setReloads((n) => n + 1);
  }, []);
  const current = shown !== null && project !== null && shown.project === project && shown.kind === kind ? shown : null;
  return { list: current?.list ?? null, error: current?.error ?? null, reload };
}
