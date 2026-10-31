import { useCallback, useEffect, useRef, useState } from "react";
import { assignBase, baseStatus, checkBase, errorText, registerBaseRom, releases as readReleases } from "../api";
import type { BaseCheck, BaseStatus, ReleasesDto } from "../api";

type Step = "check" | "assign" | "register";

/**
 * The base ROM of the open project as Technik › Veröffentlichen works on it, and the project's releases. `status` is null while it is
 * read (and when reading failed: then `error` says why). `busy` names what runs; a failure of any step becomes `error` (the German
 * sentence of the backend) with `errorFrom` naming the step (`read` for reading the status), and the status stays as it was. A release
 * itself runs in the wizard („Veröffentlichen…“) on the session's build path, not here.
 */
export interface BaseApi {
  status: BaseStatus | null;
  error: string | null;
  errorFrom: Step | "read" | null;
  busy: Step | null;
  /** Checks a `.nds` as the base ROM (writes nothing); null when the check failed (`error` says why). */
  check: (nds: string) => Promise<BaseCheck | null>;
  /** Writes the base mark for `nds` and takes the new status. */
  assign: (nds: string) => Promise<void>;
  /** Remembers where the base ROM lies now („Basis-ROM suchen…“). */
  register: (path: string) => Promise<void>;
  /** Reads the status again. */
  refresh: () => Promise<void>;
  /** The releases of the project (null while read or on a failure: `releasesError` says why). Read with the status and after a publish. */
  releases: ReleasesDto | null;
  /** Why the releases could not be read (a broken or newer `.g4/releases.json`: `releases_broken`); it locks „Veröffentlichen…“. */
  releasesError: string | null;
  /** Reads the releases again (after a release). */
  refreshReleases: () => Promise<void>;
}

export type UseBaseOptions = {
  /** The open project's folder; null without one (then nothing is read). */
  project: string | null;
  /** Technik shows: status and releases are read each time this turns true (an agent may have assigned a base or published meanwhile). */
  active: boolean;
};

/**
 * Held by the app above the areas, so the status and the releases survive leaving Technik. Everything resets when another project opens;
 * an answer that arrives for the project before is dropped.
 */
export function useBase({ project, active }: UseBaseOptions): BaseApi {
  const [status, setStatus] = useState<BaseStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [errorFrom, setErrorFrom] = useState<BaseApi["errorFrom"]>(null);
  const [busy, setBusy] = useState<Step | null>(null);
  const [releases, setReleases] = useState<ReleasesDto | null>(null);
  const [releasesError, setReleasesError] = useState<string | null>(null);
  // The project the answers belong to, and the number of the last status write: a read that started before a newer write is dropped.
  const current = useRef(project);
  const writes = useRef(0);
  // The number of the last read of the releases: an older answer is dropped.
  const releaseReads = useRef(0);

  useEffect(() => {
    current.current = project;
    writes.current += 1;
    releaseReads.current += 1;
    setStatus(null);
    setError(null);
    setErrorFrom(null);
    setBusy(null);
    setReleases(null);
    setReleasesError(null);
  }, [project]);

  const read = useCallback(async () => {
    const forProject = current.current;
    const mine = ++writes.current;
    try {
      const next = await baseStatus();
      if (current.current !== forProject || writes.current !== mine) return;
      setStatus(next);
      setError(null);
      setErrorFrom(null);
    } catch (e) {
      if (current.current !== forProject || writes.current !== mine) return;
      setError(errorText(e));
      setErrorFrom("read");
    }
  }, []);

  const refreshReleases = useCallback(async () => {
    const forProject = current.current;
    const mine = ++releaseReads.current;
    try {
      const next = await readReleases();
      if (current.current !== forProject || releaseReads.current !== mine) return;
      setReleases(next);
      setReleasesError(null);
    } catch (e) {
      if (current.current !== forProject || releaseReads.current !== mine) return;
      setReleases(null);
      setReleasesError(errorText(e));
    }
  }, []);

  useEffect(() => {
    if (project === null || !active) return;
    void read();
    void refreshReleases();
  }, [project, active, read, refreshReleases]);

  /** Runs `step` as `kind`: busy while it runs, the error cleared before and set on a failure; null when it failed or the project changed. */
  const run = useCallback(async <T,>(kind: Step, step: () => Promise<T>): Promise<T | null> => {
    const forProject = current.current;
    const mine = () => current.current === forProject;
    setBusy(kind);
    setError(null);
    setErrorFrom(null);
    try {
      const result = await step();
      return mine() ? result : null;
    } catch (e) {
      if (mine()) {
        setError(errorText(e));
        setErrorFrom(kind);
      }
      return null;
    } finally {
      if (mine()) setBusy(null);
    }
  }, []);

  const take = useCallback((next: BaseStatus | null) => {
    if (next === null) return;
    writes.current += 1;
    setStatus(next);
  }, []);

  const check = useCallback((nds: string) => run("check", () => checkBase(nds)), [run]);
  const assign = useCallback(async (nds: string) => take(await run("assign", () => assignBase(nds))), [run, take]);
  const register = useCallback(async (path: string) => take(await run("register", () => registerBaseRom(path))), [run, take]);

  return { status, error, errorFrom, busy, check, assign, register, refresh: read, releases, releasesError, refreshReleases };
}
