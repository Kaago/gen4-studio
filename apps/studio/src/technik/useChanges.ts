import { useCallback, useEffect, useRef, useState } from "react";
import { changedFromOriginal, errorText } from "../api";
import type { ChangedProgress, ChangedReport } from "../api";

/**
 * The list „geändert gegenüber Original“ of Technik › Änderungen. `report` is the last compare that went through (it stays while the next
 * one runs and after a failed one); `progress` follows the hashing of the running compare; `error` is the German sentence of the last
 * failure.
 */
export interface ChangesApi {
  report: ChangedReport | null;
  progress: ChangedProgress | null;
  error: string | null;
  busy: boolean;
  /** Compares again („Neu vergleichen“). */
  compare: () => Promise<void>;
}

export type UseChangesOptions = {
  /** The open project's folder; null without one (then nothing is compared). */
  project: string | null;
  /** Technik › Änderungen is on screen: only then is compared (the first compare of a project may hash every member). */
  shown: boolean;
  /** The base ROM is found (`OriginalStatus` ready); without it there is nothing to compare against. */
  ready: boolean;
  /** Bumped by the app after every save and every reset: the saved state may differ now. */
  revision: number;
};

/**
 * Held by the app above the areas, so the report survives leaving Technik. Compares when the surface first shows for a project and again
 * when it shows after a save or a reset (`revision` moved since the last compare), at once when that happens while it shows. Never two
 * compares side by side: a reason to compare that comes while one runs makes one more run after it. Everything starts over when another
 * project opens; an answer for the project before is dropped.
 */
export function useChanges({ project, shown, ready, revision }: UseChangesOptions): ChangesApi {
  const [report, setReport] = useState<ChangedReport | null>(null);
  const [progress, setProgress] = useState<ChangedProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const current = useRef(project);
  // The compare in flight for the current project, whether another should follow it, and the revision the last compare started at.
  const running = useRef<Promise<void> | null>(null);
  const again = useRef(false);
  const compared = useRef<number | null>(null);
  const latest = useRef(revision);
  latest.current = revision;

  useEffect(() => {
    current.current = project;
    running.current = null;
    again.current = false;
    compared.current = null;
    setReport(null);
    setProgress(null);
    setError(null);
    setBusy(false);
  }, [project]);

  const compare = useCallback((): Promise<void> => {
    const forProject = current.current;
    if (forProject === null) return Promise.resolve();
    if (running.current !== null) {
      again.current = true;
      return running.current;
    }
    const mine = () => current.current === forProject;
    const job = (async () => {
      setBusy(true);
      do {
        again.current = false;
        compared.current = latest.current;
        setProgress(null);
        setError(null);
        try {
          const next = await changedFromOriginal((p) => mine() && setProgress(p));
          if (!mine()) return;
          setReport(next);
        } catch (e) {
          if (!mine()) return;
          setError(errorText(e));
        }
      } while (again.current && mine());
      if (!mine()) return;
      setProgress(null);
      setBusy(false);
      running.current = null;
    })();
    running.current = job;
    return job;
  }, []);

  useEffect(() => {
    if (project !== null && shown && ready && compared.current !== revision) void compare();
  }, [project, shown, ready, revision, compare]);

  return { report, progress, error, busy, compare };
}
