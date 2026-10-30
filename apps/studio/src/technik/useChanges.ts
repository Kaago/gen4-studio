import { useCallback, useEffect, useRef, useState } from "react";
import { changedFromOriginal, errorText } from "../api";
import type { ChangedProgress, ChangedReport } from "../api";
import { FOCUS_DELAY_MS } from "../app/useExternalSync";
import { subscribeWindowFocus } from "../platform/windowFocus";

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
  /** The base comparison is the one on screen (`shown` and „Gegen Basis“ chosen): only then does a window focus compare. Default `shown`. */
  listed?: boolean;
};

/**
 * Held by the app above the areas, so the report survives leaving Technik. Compares when the surface first shows for a project and its base,
 * and again when it shows after a save or a reset (`revision` moved since the last compare), at once when that happens while it shows. On
 * window focus after `FOCUS_DELAY_MS` as the Git status (`useGitStatus`) reads again, so a write past the app (a bridge call, another tool)
 * shows too: at once while the base comparison is on screen (`listed`), else when it is next. Such a compare is quiet: not `busy`, no
 * progress, the report before stays until the new one comes, and `error` changes only with a real failure (or a report that went through).
 * Never
 * two compares side by side: a reason to compare that comes while one runs makes one more run after it. Everything starts over when another
 * project opens or another base is found (already in the render that sees it, so no frame shows the report before); an answer for the
 * project and base before is dropped.
 */
export function useChanges({ project, shown, base, revision, listed = shown }: UseChangesOptions): ChangesApi {
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
  const onScreen = useRef(listed);
  onScreen.current = listed;
  const focusTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  // Whether the next run shows that it compares (any reason but a focus), and whether a focus came while the base list was not on screen.
  const loud = useRef(false);
  const focused = useRef(false);

  useEffect(() => {
    current.current = key;
    running.current = null;
    again.current = false;
    compared.current = null;
    loud.current = false;
    focused.current = false;
  }, [key]);

  const run = useCallback((quiet: boolean): Promise<void> => {
    const forKey = current.current;
    if (forKey === null) return Promise.resolve();
    if (!quiet) loud.current = true;
    if (running.current !== null) {
      again.current = true;
      // „Neu vergleichen“ during a quiet run: say at once that one comes.
      if (!quiet) setBusy(true);
      return running.current;
    }
    const mine = () => current.current === forKey;
    const job = (async () => {
      do {
        again.current = false;
        focused.current = false;
        compared.current = latest.current;
        const say = loud.current;
        loud.current = false;
        if (say) {
          setBusy(true);
          setProgress(null);
          setError(null);
        }
        try {
          const next = await changedFromOriginal((p) => say && mine() && setProgress(p));
          if (!mine()) return;
          setReport(next);
          setError(null);
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
  const compare = useCallback(() => run(false), [run]);

  useEffect(() => {
    if (key === null || !shown) return;
    if (compared.current !== revision) void run(false);
    else if (listed && focused.current) void run(true);
  }, [key, shown, listed, revision, run]);

  // The window got the focus back (quick changes make one reason): the files may differ without a revision having moved.
  useEffect(() => {
    if (key === null) return;
    const stop = subscribeWindowFocus(() => {
      clearTimeout(focusTimer.current);
      focusTimer.current = setTimeout(() => {
        focusTimer.current = undefined;
        if (onScreen.current) void run(true);
        else focused.current = true;
      }, FOCUS_DELAY_MS);
    });
    return () => {
      stop();
      clearTimeout(focusTimer.current);
      focusTimer.current = undefined;
    };
  }, [key, run]);

  // React renders again at once after the reset above; this pass already shows nothing of the state before.
  if (stale) return { report: null, progress: null, error: null, busy: false, compare };
  return { report, progress, error, busy, compare };
}
