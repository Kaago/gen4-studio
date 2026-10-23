import { useCallback, useEffect, useRef, useState } from "react";
import { errorText, getTextArchive } from "../../api";
import type { TextArchiveDto } from "../../api";

/** What the last read of an archive gave: the archive, or why it could not be read. */
interface Loaded {
  index: number;
  archive: TextArchiveDto | null;
  error: string | null;
}

/**
 * Archive `index` including unsaved edits (null: none shown). It is read again when `index` or
 * `revision` changes (undo, redo, reload) and by `reload`. An archive that cannot be decoded is an
 * `error` text, not a throw: the list around it keeps working. While another archive loads, the
 * previous one is not shown as this one; while the same one reloads, it stays on screen.
 */
export function useTextArchive(index: number | null, revision: number) {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  // A read that was overtaken by a newer one (another archive, an undo) must not deliver.
  const latestRead = useRef(0);
  const latestIndex = useRef(index);
  latestIndex.current = index;

  const read = useCallback(async (i: number): Promise<TextArchiveDto | null> => {
    const mine = ++latestRead.current;
    try {
      const archive = await getTextArchive(i);
      if (latestRead.current === mine) setLoaded({ index: i, archive, error: null });
      return archive;
    } catch (e) {
      if (latestRead.current === mine) setLoaded({ index: i, archive: null, error: errorText(e) });
      return null;
    }
  }, []);

  useEffect(() => {
    if (index === null) {
      latestRead.current++;
      setLoaded(null);
      return;
    }
    void read(index);
  }, [index, revision, read]);

  /** Reads the shown archive again; resolves to it (null: none shown or unreadable). */
  const reload = useCallback(() => (latestIndex.current === null ? Promise.resolve(null) : read(latestIndex.current)), [read]);

  const current = loaded !== null && loaded.index === index ? loaded : null;
  return { archive: current?.archive ?? null, error: current?.error ?? null, loading: index !== null && current === null, reload };
}
