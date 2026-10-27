import { useCallback, useEffect, useState } from "react";
import { assignBase, baseStatus, checkBase, createPatch as createPatchCmd, errorText, registerBaseRom } from "../api";
import type { BaseCheck, BaseStatus, PatchDto } from "../api";

/**
 * The base ROM of the open project as Technik › Veröffentlichen works on it. `status` is null while it is read (and when reading
 * failed: then `error` says why). `busy` names what runs; a failure of any step becomes `error` (the German sentence of the
 * backend), and the status stays as it was. `lastPatch` is the patch written in this session, kept until the area goes.
 */
export interface BaseApi {
  status: BaseStatus | null;
  error: string | null;
  busy: "check" | "assign" | "register" | "patch" | null;
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

/** Reads the status once when it mounts; `TechnikArea` mounts it once per project, when Technik shows. */
export function useBase(): BaseApi {
  const [status, setStatus] = useState<BaseStatus | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<BaseApi["busy"]>(null);
  const [lastPatch, setLastPatch] = useState<PatchDto | null>(null);

  useEffect(() => {
    let live = true;
    baseStatus().then(
      (next) => live && setStatus(next),
      (e) => live && setError(errorText(e)),
    );
    return () => {
      live = false;
    };
  }, []);

  /** Runs `step` as `kind`: busy while it runs, the error cleared before and set on a failure. */
  const run = useCallback(async <T,>(kind: NonNullable<BaseApi["busy"]>, step: () => Promise<T>): Promise<T | null> => {
    setBusy(kind);
    setError(null);
    try {
      return await step();
    } catch (e) {
      setError(errorText(e));
      return null;
    } finally {
      setBusy(null);
    }
  }, []);

  const check = useCallback((nds: string) => run("check", () => checkBase(nds)), [run]);

  const assign = useCallback(
    async (nds: string) => {
      const next = await run("assign", () => assignBase(nds));
      if (next !== null) setStatus(next);
    },
    [run],
  );

  const register = useCallback(
    async (path: string) => {
      const next = await run("register", () => registerBaseRom(path));
      if (next !== null) setStatus(next);
    },
    [run],
  );

  const createPatch = useCallback(async () => {
    const patch = await run("patch", createPatchCmd);
    if (patch !== null) setLastPatch(patch);
  }, [run]);

  const refresh = useCallback(async () => {
    try {
      setStatus(await baseStatus());
      setError(null);
    } catch (e) {
      setError(errorText(e));
    }
  }, []);

  return { status, error, busy, check, assign, register, createPatch, lastPatch, refresh };
}
