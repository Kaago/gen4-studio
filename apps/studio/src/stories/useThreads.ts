import { useCallback, useEffect, useState } from "react";
import { errorText, listThreads } from "../api";
import type { ProjectInfo, ThreadListDto } from "../api";

type Shown = { project: ProjectInfo; list: ThreadListDto | null; error: string | null };

/**
 * The threads of the open project (`list_threads`), read when there is a project, again for every new `revision` (an edit, a save, an own
 * name) and on `reload`. While a newer revision is read the older list stays; an answer that a newer read overtook never shows, nor a list
 * of another project. `error` is the refusal's sentence.
 */
export function useThreads(deps: { project: ProjectInfo | null; revision: number }): { list: ThreadListDto | null; error: string | null; reload: () => void } {
  const { project, revision } = deps;
  const [shown, setShown] = useState<Shown | null>(null);
  const [reloads, setReloads] = useState(0);
  useEffect(() => {
    if (project === null) return;
    let live = true;
    listThreads().then(
      (list) => live && setShown({ project, list, error: null }),
      (e: unknown) => live && setShown({ project, list: null, error: errorText(e) }),
    );
    return () => {
      live = false;
    };
  }, [project, revision, reloads]);
  const reload = useCallback(() => setReloads((n) => n + 1), []);
  const current = shown !== null && project !== null && shown.project === project ? shown : null;
  return { list: current?.list ?? null, error: current?.error ?? null, reload };
}
