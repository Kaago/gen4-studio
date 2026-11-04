import { useCallback, useEffect, useState } from "react";
import { errorText, storyList } from "../api";
import type { ProjectInfo, StoryListDto } from "../api";

type Shown = { project: ProjectInfo; revision: number; list: StoryListDto | null; error: string | null };

/**
 * The stories of the open project (`story_list`), read when there is a project, again for every new `revision` (a story created or
 * changed, an undo, a save) and on `reload`. While a newer revision is read the older list stays; an answer that a newer read overtook
 * never shows, nor a list of another project. `error` is the refusal's sentence; `revision` the one the shown answer was read at (behind
 * `deps.revision` while a newer read runs), null without one.
 */
export function useStories(deps: { project: ProjectInfo | null; revision: number }): {
  list: StoryListDto | null;
  error: string | null;
  revision: number | null;
  reload: () => void;
} {
  const { project, revision } = deps;
  const [shown, setShown] = useState<Shown | null>(null);
  const [reloads, setReloads] = useState(0);
  useEffect(() => {
    if (project === null) return;
    let live = true;
    storyList().then(
      (list) => live && setShown({ project, revision, list, error: null }),
      (e: unknown) => live && setShown({ project, revision, list: null, error: errorText(e) }),
    );
    return () => {
      live = false;
    };
  }, [project, revision, reloads]);
  const reload = useCallback(() => setReloads((n) => n + 1), []);
  const current = shown !== null && project !== null && shown.project === project ? shown : null;
  return { list: current?.list ?? null, error: current?.error ?? null, revision: current?.revision ?? null, reload };
}
