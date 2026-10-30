import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, gitStatus } from "../api";
import type { GitStatus } from "../api";
import { FOCUS_DELAY_MS } from "../app/useExternalSync";
import { subscribeWindowFocus } from "../platform/windowFocus";

/** The Git state of the open project, as the status line, the palette and Technik › Änderungen read it (`useGitStatus`). */
export interface GitStatusApi {
  /** The last status read for this project; null before the first answered and without a project. */
  status: GitStatus | null;
  /** Why the last read failed; null when it went through. */
  error: string | null;
  /** Reads again (after „Git einrichten“): after a running read when there is one; resolves when the read that covers this call is done. */
  refresh: () => Promise<void>;
}

export interface GitStatusOptions {
  /** The open project's folder; null without one. */
  project: string | null;
  /**
   * Moves whenever the saved project may differ (a save, a build, a reset, ⌘Z/⌘⇧Z of a file step, a sync that took something over, an
   * agent's write): the status is read again. The same counter Technik › Änderungen compares on.
   */
  revision: number;
  /** The session of the project (its info object): another one of the same folder is read anew, as after opening. */
  opened?: unknown;
}

/** What was read, for the project it was read in. */
type Found = { key: string | null; status: GitStatus | null; error: string | null };

/**
 * The Git state of the open project (plan 4.4 „Wann der Status gelesen wird“): when it opens, when `revision` moves, on `refresh` and on
 * window focus after `FOCUS_DELAY_MS` (quick focus changes make one read; a commit made in the user's Git tool shows on returning). Never
 * two reads at once: a reason that comes while one runs waits for it, and all reasons meanwhile share the one read after it. Another project
 * forgets what was read, and a read of the old one that answers late is dropped.
 */
export function useGitStatus({ project, revision, opened }: GitStatusOptions): GitStatusApi {
  const [found, setFound] = useState<Found>({ key: project, status: null, error: null });
  // Bumped when the project (or its session) changes: answers of an older generation are dropped.
  const generation = useRef(0);
  const keyRef = useRef(project);
  // The read under way, and the one waiting for it (shared by every reason meanwhile).
  const inflight = useRef<Promise<void> | null>(null);
  const queued = useRef<Promise<void> | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  // The revision the last read was asked for: opening reads once, not once more for the revision it came with.
  const seenRevision = useRef(revision);

  const once = useCallback(async (gen: number): Promise<void> => {
    try {
      const status = await gitStatus();
      if (generation.current !== gen) return;
      setFound({ key: keyRef.current, status, error: null });
    } catch (e) {
      if (generation.current !== gen) return;
      setFound((before) => ({ key: keyRef.current, status: before.key === keyRef.current ? before.status : null, error: errorText(e) }));
    }
  }, []);

  const start = useCallback(
    (gen: number): Promise<void> => {
      const run: Promise<void> = once(gen).finally(() => {
        if (inflight.current === run) inflight.current = null;
      });
      inflight.current = run;
      return run;
    },
    [once],
  );

  const request = useCallback((): Promise<void> => {
    if (keyRef.current === null) return Promise.resolve();
    const gen = generation.current;
    const before = inflight.current;
    if (before === null) return start(gen);
    if (queued.current === null) {
      const next: Promise<void> = before.then(() => {
        // Another project dropped this one meanwhile: its own read waits on its own.
        if (queued.current !== next) return;
        queued.current = null;
        return start(gen);
      });
      queued.current = next;
    }
    return queued.current;
  }, [start]);

  useEffect(() => {
    generation.current++;
    keyRef.current = project;
    queued.current = null;
    seenRevision.current = revision;
    setFound({ key: project, status: null, error: null });
    if (project === null) return;
    void request();
    const stop = subscribeWindowFocus(() => {
      clearTimeout(timer.current);
      timer.current = setTimeout(() => {
        timer.current = undefined;
        void request();
      }, FOCUS_DELAY_MS);
    });
    return () => {
      // A read of this project (or session) that answers after this is dropped, also after an unmount, and the one waiting for it never runs.
      generation.current++;
      queued.current = null;
      stop();
      clearTimeout(timer.current);
      timer.current = undefined;
    };
    // `revision` is read, not followed: the effect below follows it.
  }, [project, opened, request]);

  useEffect(() => {
    if (seenRevision.current === revision) return;
    seenRevision.current = revision;
    void request();
  }, [revision, request]);

  const shown = found.key === project ? found : { status: null, error: null };
  return { status: shown.status, error: shown.error, refresh: request };
}
