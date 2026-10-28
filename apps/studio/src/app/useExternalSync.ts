import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, externalSync } from "../api";
import type { SyncOutcome, SyncProgress } from "../api";
import { subscribeWindowFocus } from "../platform/windowFocus";
import { useStableCallback } from "./useStable";

/** A focus syncs this long after it came (the last of several quick ones counts). */
export const FOCUS_DELAY_MS = 800;

/** The sync of the open text project with DSPRE, as Technik › Textprojekt and the app's build read it (`useExternalSync`). */
export interface ExternalSyncApi {
  /** What the last sync of this project found; null before the first answered. */
  last: SyncOutcome | null;
  /** The last sync of this project that took something over (adopted or applied), for „Zuletzt übernommen“; null before any. */
  taken: SyncOutcome | null;
  running: boolean;
  /** How far the running sync is; null before it reports. */
  progress: SyncProgress | null;
  /** Why the last sync failed; null when it went through. */
  error: string | null;
  /** Syncs now („Jetzt abgleichen“, before a build): after a running sync when there is one; answers its outcome, null when it failed. */
  run: () => Promise<SyncOutcome | null>;
}

export interface ExternalSyncOptions {
  /** The open project's folder; null without one. */
  project: string | null;
  /** The project is a text project of the current schema: only then is there anything to sync. */
  textProject: boolean;
  /** A build or a save runs: no sync starts by itself meanwhile (the build syncs first on its own). */
  busy: boolean;
  /** Each sync that went through (the app reloads what it took over and says so). */
  onOutcome: (outcome: SyncOutcome) => void;
  /** The sync when the project opened failed (a focus sync or `run` keeps its error in `error` only). */
  onError?: (e: unknown) => void;
}

/** What a sync found, for the project it was found in. */
type Found = { key: string | null; last: SyncOutcome | null; taken: SyncOutcome | null; error: string | null };

/** Whether a sync that waits reports its failure: the one of the opening does, a focus sync and `run` do not. */
type Loud = boolean;

/**
 * The sync of a text project with DSPRE (spec 4.3b „Wann abgeglichen wird“): when the project opens, on window focus after
 * `FOCUS_DELAY_MS` (quick focus changes make one sync), and on `run`. At most one sync runs at a time; a request while one runs waits for
 * it, and all requests meanwhile share the one sync after it. While `busy` (a build) no sync starts by itself; one that came meanwhile
 * runs once when `busy` ends. Another project (or the folder stops being a text project) forgets what was found, and a sync of the old one
 * that answers late is dropped.
 */
export function useExternalSync({ project, textProject, busy, onOutcome, onError }: ExternalSyncOptions): ExternalSyncApi {
  const key = project !== null && textProject ? project : null;
  const [found, setFound] = useState<Found>({ key, last: null, taken: null, error: null });
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<SyncProgress | null>(null);
  const reportOutcome = useStableCallback(onOutcome);
  const reportError = useStableCallback((e: unknown) => onError?.(e));

  const busyRef = useRef(busy);
  busyRef.current = busy;
  // Bumped when the project changes: answers of an older generation are dropped.
  const generation = useRef(0);
  const keyRef = useRef(key);
  // The sync under way, the one waiting for it (shared by every request meanwhile) and whether that one reports its failure.
  const inflight = useRef<Promise<SyncOutcome | null> | null>(null);
  const queued = useRef<Promise<SyncOutcome | null> | null>(null);
  const queuedLoud = useRef<Loud>(false);
  // A request that came while busy: it runs when busy ends (null: none came).
  const deferred = useRef<Loud | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const once = useCallback(
    async (gen: number, loud: Loud): Promise<SyncOutcome | null> => {
      const mine = () => generation.current === gen;
      setProgress(null);
      try {
        const outcome = await externalSync((p) => {
          if (mine()) setProgress(p);
        });
        if (!mine()) return null;
        const took = outcome.adopted.length + outcome.applied.length > 0;
        setFound((before) => ({ key: keyRef.current, last: outcome, taken: took ? outcome : before.taken, error: null }));
        reportOutcome(outcome);
        return outcome;
      } catch (e) {
        if (!mine()) return null;
        setFound((before) => ({ ...before, key: keyRef.current, error: errorText(e) }));
        if (loud) reportError(e);
        return null;
      } finally {
        if (mine()) setProgress(null);
      }
    },
    [reportOutcome, reportError],
  );

  const start = useCallback(
    (gen: number, loud: Loud): Promise<SyncOutcome | null> => {
      if (generation.current !== gen) return Promise.resolve(null);
      setRunning(true);
      const run: Promise<SyncOutcome | null> = once(gen, loud).finally(() => {
        if (inflight.current === run) inflight.current = null;
        // The sync waiting for this one starts next: still running.
        if (queued.current === null && generation.current === gen) setRunning(false);
      });
      inflight.current = run;
      return run;
    },
    [once],
  );

  const request = useCallback(
    (loud: Loud): Promise<SyncOutcome | null> => {
      const gen = generation.current;
      const before = inflight.current;
      if (before === null) return start(gen, loud);
      queuedLoud.current ||= loud;
      if (queued.current === null) {
        const next: Promise<SyncOutcome | null> = before.then(() => {
          // Another project dropped this one meanwhile: its successor waits on its own.
          if (queued.current !== next) return null;
          queued.current = null;
          const reports = queuedLoud.current;
          queuedLoud.current = false;
          return start(gen, reports);
        });
        queued.current = next;
        setRunning(true);
      }
      return queued.current;
    },
    [start],
  );

  /** A sync that starts by itself: not while busy (it runs when busy ends). */
  const requestUnlessBusy = useCallback(
    (loud: Loud) => {
      if (busyRef.current) deferred.current = (deferred.current ?? false) || loud;
      else void request(loud);
    },
    [request],
  );

  useEffect(() => {
    generation.current++;
    keyRef.current = key;
    queued.current = null;
    queuedLoud.current = false;
    deferred.current = null;
    setFound({ key, last: null, taken: null, error: null });
    setRunning(false);
    setProgress(null);
    if (key === null) return;
    requestUnlessBusy(true);
    const stop = subscribeWindowFocus(() => {
      clearTimeout(timer.current);
      timer.current = setTimeout(() => {
        timer.current = undefined;
        requestUnlessBusy(false);
      }, FOCUS_DELAY_MS);
    });
    return () => {
      stop();
      clearTimeout(timer.current);
      timer.current = undefined;
    };
  }, [key, requestUnlessBusy]);

  useEffect(() => {
    if (busy || deferred.current === null) return;
    const loud = deferred.current;
    deferred.current = null;
    void request(loud);
  }, [busy, request]);

  const run = useCallback(() => (keyRef.current === null ? Promise.resolve(null) : request(false)), [request]);

  const shown = found.key === key ? found : { last: null, taken: null, error: null };
  return { last: shown.last, taken: shown.taken, running: key !== null && running, progress: key !== null ? progress : null, error: shown.error, run };
}
