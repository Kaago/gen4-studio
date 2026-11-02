import { useEffect, useState } from "react";
import { errorText, threadGraph } from "../api";
import type { ProjectInfo, ThreadGraphDto } from "../api";

type Read = { project: ProjectInfo; variable: number; story: string | null; graph: ThreadGraphDto | null; error: string | null };

/**
 * The graph of variable `variable` (`thread_graph`), with the steps and the layout of `story` (its id) when given; read when a variable is
 * chosen and again for every new `revision`. Nothing chosen or no project: nothing is read. An answer that a newer read overtook never
 * shows, nor the graph of another variable, story or project while the new one is read (`busy` meanwhile). `error` is the refusal's sentence.
 */
export function useThreadGraph(
  variable: number | null,
  story: string | null,
  deps: { project: ProjectInfo | null; revision: number },
): { graph: ThreadGraphDto | null; error: string | null; busy: boolean } {
  const { project, revision } = deps;
  const [read, setRead] = useState<Read | null>(null);
  useEffect(() => {
    if (project === null || variable === null) return;
    let live = true;
    threadGraph(variable, story ?? undefined).then(
      (graph) => live && setRead({ project, variable, story, graph, error: null }),
      (e: unknown) => live && setRead({ project, variable, story, graph: null, error: errorText(e) }),
    );
    return () => {
      live = false;
    };
  }, [project, variable, story, revision]);
  if (project === null || variable === null) return { graph: null, error: null, busy: false };
  const current = read !== null && read.project === project && read.variable === variable && read.story === story ? read : null;
  return { graph: current?.graph ?? null, error: current?.error ?? null, busy: current === null };
}
