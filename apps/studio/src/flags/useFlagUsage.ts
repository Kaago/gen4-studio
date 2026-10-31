import { useEffect, useState } from "react";
import { errorText, flagUsage } from "../api";
import type { FlagKind, FlagUsageDto, ProjectInfo } from "../api";
import { useFlagListGeneration } from "./useFlagList";

type Read = { project: ProjectInfo; kind: FlagKind; number: number; usage: FlagUsageDto | null; error: string | null };

/**
 * Where number `number` of `kind` is set and checked (`flag_usage`), read when it is chosen, again for every new `revision` and after
 * `invalidateFlagLists` (an own name, a description). Nothing chosen or no project: nothing is read. An answer that a newer read overtook
 * never shows, nor the usage of another number or project while the new one is read (`busy` meanwhile). `error` is the refusal's sentence.
 */
export function useFlagUsage(kind: FlagKind, number: number | null, deps: { project: ProjectInfo | null; revision: number }): { usage: FlagUsageDto | null; error: string | null; busy: boolean } {
  const { project, revision } = deps;
  const gen = useFlagListGeneration();
  const [read, setRead] = useState<Read | null>(null);
  useEffect(() => {
    if (project === null || number === null) return;
    let live = true;
    flagUsage(kind, number).then(
      (usage) => live && setRead({ project, kind, number, usage, error: null }),
      (e: unknown) => live && setRead({ project, kind, number, usage: null, error: errorText(e) }),
    );
    return () => {
      live = false;
    };
  }, [project, kind, number, revision, gen]);
  if (project === null || number === null) return { usage: null, error: null, busy: false };
  const current = read !== null && read.project === project && read.kind === kind && read.number === number ? read : null;
  return { usage: current?.usage ?? null, error: current?.error ?? null, busy: current === null };
}
