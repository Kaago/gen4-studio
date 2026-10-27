import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, originalStatus } from "../api";
import type { BaseStatus, OriginalStatus } from "../api";

export interface OriginalApi {
  /** The base ROM as resetting needs it; null while it is read and without a project. A failed read counts as `unavailable`. */
  status: OriginalStatus | null;
  /** Reads the status again (an agent assigned a base). */
  refresh: () => Promise<void>;
}

export type UseOriginalOptions = {
  /** The open project's folder; null without one (then nothing is read). */
  project: string | null;
  /** The base status of Technik › Veröffentlichen (`useBase`); a change of the base it shows (assign, register) reads again. */
  base: BaseStatus | null;
};

/** What of the base status says which base ROM is meant and whether it is found; null while it is not known. */
function baseKey(base: BaseStatus | null): string | null {
  if (base === null) return null;
  switch (base.state) {
    case "found":
      return `found:${base.mark.sha1}:${base.path}`;
    case "missing":
      return `missing:${base.mark.sha1}`;
    default:
      return base.state;
  }
}

/**
 * The base ROM of the open project as „Auf Original zurücksetzen“ needs it: read once per project (it reads the base ROM) and again when
 * the base status changes from one known state to another (a base assigned, the ROM found again). The base status arriving the first time
 * (Technik opened) is no change. An answer for the project before is dropped.
 */
export function useOriginal({ project, base }: UseOriginalOptions): OriginalApi {
  const [status, setStatus] = useState<OriginalStatus | null>(null);
  const current = useRef(project);
  const reads = useRef(0);
  // The base status last seen for this project; null until one is known.
  const seen = useRef<string | null>(null);

  const read = useCallback(async () => {
    const forProject = current.current;
    if (forProject === null) return;
    const mine = ++reads.current;
    let next: OriginalStatus;
    try {
      next = await originalStatus();
    } catch (e) {
      next = { state: "unavailable", reason: errorText(e) };
    }
    if (current.current === forProject && reads.current === mine) setStatus(next);
  }, []);

  useEffect(() => {
    current.current = project;
    reads.current += 1;
    seen.current = null;
    setStatus(null);
    if (project !== null) void read();
  }, [project, read]);

  const key = baseKey(base);
  useEffect(() => {
    const before = seen.current;
    seen.current = key;
    if (key !== null && before !== null && key !== before) void read();
  }, [key, read]);

  return { status, refresh: read };
}
