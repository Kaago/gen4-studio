import { useEffect, useRef, useState } from "react";
import { errorText, refs } from "../api";
import type { DataKind, ProjectInfo, RefDto, TargetDto } from "../api";
import type { DataArea } from "./dataModel";
import { useDataArea } from "./useDataArea";

/** The references found, or why they could not be. */
export interface FoundRefs {
  refs: RefDto[] | null;
  error: string | null;
}

/** One key per target, so the effect runs again for another target, not for another object of the same one. */
const keyOf = (target: TargetDto): string => {
  if (target.kind === "message") return `message:${target.archive}:${target.index}`;
  return `${target.kind}:${"id" in target ? target.id : target.index}`;
};

/**
 * The references to `target` (null: none asked for) in the open project, read again when a member of one of `referring` changed
 * (`dataChanged`): the kinds whose change can add or drop a reference to it. Null while they are read; an answer that a newer read
 * overtook, or one for another target, never shows. `referring` must be stable (a module constant).
 */
export function useRefs(area: DataArea, target: TargetDto | null, referring: readonly DataKind[]): FoundRefs | null {
  const { project, bus } = useDataArea(area);
  const [found, setFound] = useState<(FoundRefs & { key: string; project: ProjectInfo }) | null>(null);
  const latest = useRef(0);
  // The target of the latest render, for the reads its key starts.
  const asked = useRef(target);
  asked.current = target;
  const key = target === null ? null : keyOf(target);
  useEffect(() => {
    const current = asked.current;
    if (key === null || current === null || project === null) return;
    const read = async () => {
      const mine = ++latest.current;
      try {
        const all = await refs(current);
        if (latest.current === mine) setFound({ key, project, refs: all, error: null });
      } catch (e) {
        if (latest.current === mine) setFound({ key, project, refs: null, error: errorText(e) });
      }
    };
    void read();
    return bus.subscribe("dataChanged", (event) => {
      if (event.topic === "dataChanged" && referring.includes(event.kind)) void read();
    });
  }, [key, project, bus, referring]);
  return found !== null && found.key === key && found.project === project ? found : null;
}

/**
 * The references to `target` (null: none asked for) in `project`, outside a data area (the text area's „Verwendet in“ of a message): read
 * whenever the target or the project changes. Null while they are read; an answer that a newer read overtook, or one for another target,
 * never shows.
 */
export function useProjectRefs(project: ProjectInfo | null, target: TargetDto | null): FoundRefs | null {
  const [found, setFound] = useState<(FoundRefs & { key: string; project: ProjectInfo }) | null>(null);
  const latest = useRef(0);
  const asked = useRef(target);
  asked.current = target;
  const key = target === null ? null : keyOf(target);
  useEffect(() => {
    const current = asked.current;
    if (key === null || current === null || project === null) return;
    const mine = ++latest.current;
    refs(current).then(
      (all) => latest.current === mine && setFound({ key, project, refs: all, error: null }),
      (e) => latest.current === mine && setFound({ key, project, refs: null, error: errorText(e) }),
    );
  }, [key, project]);
  return found !== null && found.key === key && found.project === project ? found : null;
}
