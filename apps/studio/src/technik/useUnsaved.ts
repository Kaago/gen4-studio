import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, unsavedChanges } from "../api";
import type { ChangedEntry } from "../api";

export type UseUnsavedOptions = {
  /** The open project's folder; null without one (nothing is read). */
  project: string | null;
  /** Technik › Änderungen shows „Gegen gespeichert“: only then is read. */
  shown: boolean;
  /** Moves with every write of the app and every save: the unsaved edits may differ now. */
  revision: number;
};

/** What was read, for the project it was read in. */
type Found = { project: string | null; entries: ChangedEntry[] | null; error: string | null };

/**
 * The list „Gegen gespeichert“ of Technik › Änderungen: the store's unsaved edits as entities. Read when it shows and again when `revision`
 * moves while it shows (a move while hidden reads once it shows again). A failed read keeps the list before with the reason; `reload` reads
 * again at once, and so does showing it again. Another project forgets the list; an answer that comes after a newer read started, or for
 * the project before, is dropped.
 */
export function useUnsaved({ project, shown, revision }: UseUnsavedOptions): { entries: ChangedEntry[] | null; error: string | null; reload: () => void } {
  const [found, setFound] = useState<Found>({ project, entries: null, error: null });
  const serial = useRef(0);
  // The revision the shown list was read at, per project: showing again without a move reads nothing. A failed read leaves none.
  const read = useRef<{ project: string | null; revision: number } | null>(null);
  const latest = useRef({ project, revision });
  latest.current = { project, revision };

  const run = useCallback(() => {
    const { project: at, revision: rev } = latest.current;
    if (at === null) return;
    read.current = { project: at, revision: rev };
    const mine = ++serial.current;
    unsavedChanges().then(
      (entries) => mine === serial.current && setFound({ project: at, entries, error: null }),
      (e: unknown) => {
        if (mine !== serial.current) return;
        read.current = null;
        setFound((before) => ({ project: at, entries: before.project === at ? before.entries : null, error: errorText(e) }));
      },
    );
  }, []);

  useEffect(() => {
    if (project === null || !shown) return;
    if (read.current !== null && read.current.project === project && read.current.revision === revision) return;
    run();
  }, [project, shown, revision, run]);

  // Another project: drop what is in flight for the one before.
  useEffect(
    () => () => {
      serial.current++;
      read.current = null;
    },
    [project],
  );

  return found.project === project ? { entries: found.entries, error: found.error, reload: run } : { entries: null, error: null, reload: run };
}
