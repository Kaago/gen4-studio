import { useEffect, useState } from "react";
import { errorText, storyGet } from "../api";
import type { ProjectInfo, StoryDto } from "../api";

type Read = { project: ProjectInfo; id: string; story: StoryDto | null; error: string | null };

/**
 * The story `id` (`story_get`), read when one is chosen and again for every new `revision`; while a newer revision of the same story is
 * read the older answer stays (a field keeps the text it wrote until it is read back). Nothing chosen or no project: nothing is read. An
 * answer a newer read overtook never shows, nor another story or project while the new one is read (`busy` meanwhile). `error` is the
 * refusal's sentence.
 */
export function useStory(id: string | null, deps: { project: ProjectInfo | null; revision: number }): { story: StoryDto | null; error: string | null; busy: boolean } {
  const { project, revision } = deps;
  const [read, setRead] = useState<Read | null>(null);
  useEffect(() => {
    if (project === null || id === null) return;
    let live = true;
    storyGet(id).then(
      (story) => live && setRead({ project, id, story, error: null }),
      (e: unknown) => live && setRead({ project, id, story: null, error: errorText(e) }),
    );
    return () => {
      live = false;
    };
  }, [project, id, revision]);
  if (project === null || id === null) return { story: null, error: null, busy: false };
  const current = read !== null && read.project === project && read.id === id ? read : null;
  return { story: current?.story ?? null, error: current?.error ?? null, busy: current === null };
}
