import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, listFlags } from "../api";
import type { FlagKind, FlagListDto, ProjectInfo } from "../api";

/**
 * The read of each kind per project and revision, shared by every field and picker that asks for it: a level script with ten conditions
 * reads the variables once, and a picker opened next to its field reuses the field's list. Only the latest revision of a kind is kept; a
 * refused read is dropped, so the next one asks again.
 */
const reads = new WeakMap<ProjectInfo, Map<FlagKind, { revision: number; list: Promise<FlagListDto> }>>();

function read(project: ProjectInfo, kind: FlagKind, revision: number, fresh: boolean): Promise<FlagListDto> {
  let kinds = reads.get(project);
  if (kinds === undefined) reads.set(project, (kinds = new Map()));
  const held = kinds.get(kind);
  if (!fresh && held !== undefined && held.revision === revision) return held.list;
  const entry = { revision, list: listFlags(kind) };
  kinds.set(kind, entry);
  entry.list.catch(() => {
    if (kinds.get(kind) === entry) kinds.delete(kind);
  });
  return entry.list;
}

type Shown = { project: ProjectInfo; kind: FlagKind; list: FlagListDto | null; error: string | null };

/**
 * The list of flags or variables (`kind`) of the open project, read when there is a project and again for every new `revision` (an event,
 * a script or a name changed); `reload` reads it again without one. While a newer revision is read the older list stays; an answer that a
 * newer read overtook never shows, nor a list of another project or kind. `error` is the refusal's sentence.
 */
export function useFlagList(kind: FlagKind, deps: { project: ProjectInfo | null; revision: number }): { list: FlagListDto | null; error: string | null; reload: () => void } {
  const { project, revision } = deps;
  const [shown, setShown] = useState<Shown | null>(null);
  const [reloads, setReloads] = useState(0);
  // Set by `reload`: the next read bypasses the shared one.
  const fresh = useRef(false);
  useEffect(() => {
    if (project === null) return;
    let live = true;
    const bypass = fresh.current;
    fresh.current = false;
    read(project, kind, revision, bypass).then(
      (list) => live && setShown({ project, kind, list, error: null }),
      (e: unknown) => live && setShown({ project, kind, list: null, error: errorText(e) }),
    );
    return () => {
      live = false;
    };
  }, [project, kind, revision, reloads]);
  const reload = useCallback(() => {
    fresh.current = true;
    setReloads((n) => n + 1);
  }, []);
  const current = shown !== null && project !== null && shown.project === project && shown.kind === kind ? shown : null;
  return { list: current?.list ?? null, error: current?.error ?? null, reload };
}
