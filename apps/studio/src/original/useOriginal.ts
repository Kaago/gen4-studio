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
 * (Technik opened) is no change, unless it disagrees with the status read before (found but not ready, or ready but not found: the ROM
 * appeared or went since); then it is read once more. An answer for the project before is dropped.
 */
export function useOriginal({ project, base }: UseOriginalOptions): OriginalApi {
  // The status with the project it was read for: a render for another project gets null, not the status of the one before (the reset
  // below runs in an effect, after the first render for the new project).
  const [held, setHeld] = useState<{ project: string | null; status: OriginalStatus | null }>({ project, status: null });
  const current = useRef(project);
  const reads = useRef(0);
  // The base status last seen for this project; null until one is known.
  const seen = useRef<string | null>(null);
  // Whether the first base status of this project was held against the status read (once per project).
  const compared = useRef(false);

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
    if (current.current === forProject && reads.current === mine) setHeld({ project: forProject, status: next });
  }, []);

  useEffect(() => {
    current.current = project;
    reads.current += 1;
    seen.current = null;
    compared.current = false;
    setHeld({ project, status: null });
    if (project !== null) void read();
  }, [project, read]);

  const key = baseKey(base);
  useEffect(() => {
    const before = seen.current;
    seen.current = key;
    if (key !== null && before !== null && key !== before) void read();
  }, [key, read]);

  const status = held.project === project ? held.status : null;
  const found = base === null ? null : base.state === "found";
  useEffect(() => {
    if (compared.current || found === null || status === null) return;
    compared.current = true;
    if (found !== (status.state === "ready")) void read();
  }, [found, status, read]);

  return { status, refresh: read };
}
