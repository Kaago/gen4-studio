import { useCallback, useEffect, useRef, useState } from "react";
import { assignBase, baseStatus, checkBase, createPatch as createPatchCmd, errorText, registerBaseRom } from "../api";
import type { BaseCheck, BaseStatus, PatchDto } from "../api";
import { useStableCallback } from "../app/useStable";

type Step = "check" | "assign" | "register" | "patch";

/**
 * The base ROM of the open project as Technik › Veröffentlichen works on it. `status` is null while it is read (and when reading
 * failed: then `error` says why). `busy` names what runs; a failure of any step becomes `error` (the German sentence of the
 * backend) with `errorFrom` naming the step (`read` for reading the status), and the status stays as it was. `lastPatch` is the
 * patch written for this project in this session.
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
  /** Saves, builds and writes the patch. */
  createPatch: () => Promise<void>;
  lastPatch: PatchDto | null;
  /** Reads the status again. */
  refresh: () => Promise<void>;
}

export type UseBaseOptions = {
  /** The open project's folder; null without one (then nothing is read). */
  project: string | null;
  /** Technik shows: the status is read each time this turns true (an agent may have assigned a base meanwhile). */
  active: boolean;
  /** Runs the patch; the app passes the session's build path, so its build and save guards hold while a patch builds. */
  createPatch?: () => Promise<PatchDto>;
};

/**
 * Held by the app above the areas, so a running patch, its result and the status survive leaving Technik. Everything resets when another
 * project opens; an answer that arrives for the project before is dropped.
 */
export function useBase({ project, active, createPatch: runPatch = createPatchCmd }: UseBaseOptions): BaseApi {
  const [status, setStatus] = useState<BaseStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [errorFrom, setErrorFrom] = useState<BaseApi["errorFrom"]>(null);
  const [busy, setBusy] = useState<Step | null>(null);
  const [lastPatch, setLastPatch] = useState<PatchDto | null>(null);
  const patch = useStableCallback(runPatch);
  // The project the answers belong to, and the number of the last status write: a read that started before a newer write is dropped.
  const current = useRef(project);
  const writes = useRef(0);

  useEffect(() => {
    current.current = project;
    writes.current += 1;
    setStatus(null);
    setError(null);
    setErrorFrom(null);
    setBusy(null);
    setLastPatch(null);
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

  useEffect(() => {
    if (project !== null && active) void read();
  }, [project, active, read]);

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
  const createPatch = useCallback(async () => {
    const done = await run("patch", patch);
    if (done !== null) setLastPatch(done);
  }, [run, patch]);

  return { status, error, errorFrom, busy, check, assign, register, createPatch, lastPatch, refresh: read };
}
