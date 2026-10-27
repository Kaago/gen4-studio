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
  /**
   * The SHA-1 of the found base ROM (`OriginalStatus` ready, read for this project); null without one, then nothing is compared. Another
   * base is compared against at once, and the report of the old one is dropped.
   */
  base: string | null;
  /** Bumped by the app after every save and every reset: the saved state may differ now. */
  revision: number;
};

/**
 * Held by the app above the areas, so the report survives leaving Technik. Compares when the surface first shows for a project and its base,
 * and again when it shows after a save or a reset (`revision` moved since the last compare), at once when that happens while it shows. Never
 * two compares side by side: a reason to compare that comes while one runs makes one more run after it. Everything starts over when another
 * project opens or another base is found (already in the render that sees it, so no frame shows the report before); an answer for the
 * project and base before is dropped.
 */
export function useChanges({ project, shown, base, revision }: UseChangesOptions): ChangesApi {
  const [report, setReport] = useState<ChangedReport | null>(null);
  const [progress, setProgress] = useState<ChangedProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // What the state belongs to: another project or base resets it while rendering (as `useScripts` does), the refs below in the effect.
  const key = project === null || base === null ? null : `${project}\u0000${base}`;
  const [owner, setOwner] = useState(key);
  const stale = owner !== key;
  if (stale) {
    setOwner(key);
    setReport(null);
    setProgress(null);
    setError(null);
    setBusy(false);
  }
  const current = useRef(key);
  // The compare in flight for the current project and base, whether another should follow it, and the revision the last compare started at.
  const running = useRef<Promise<void> | null>(null);
  const again = useRef(false);
  const compared = useRef<number | null>(null);
  const latest = useRef(revision);
  latest.current = revision;

  useEffect(() => {
    current.current = key;
    running.current = null;
    again.current = false;
    compared.current = null;
  }, [key]);

  const compare = useCallback((): Promise<void> => {
    const forKey = current.current;
    if (forKey === null) return Promise.resolve();
    if (running.current !== null) {
      again.current = true;
      return running.current;
    }
    const mine = () => current.current === forKey;
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
    if (key !== null && shown && compared.current !== revision) void compare();
  }, [key, shown, revision, compare]);

  // React renders again at once after the reset above; this pass already shows nothing of the state before.
  if (stale) return { report: null, progress: null, error: null, busy: false, compare };
  return { report, progress, error, busy, compare };
}
