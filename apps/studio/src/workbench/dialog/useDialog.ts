import { useCallback, useEffect, useRef, useState } from "react";
import { dialogOf, errorText } from "../../api";
import type { Dialog } from "../../api";
import type { DialogTarget } from "./dialogModel";

/** The dialog of `target` as read, or why it could not be; `reload` reads it again. */
export interface FoundDialog {
  /** Null while it is read (the first time for a figure), when it could not be read, and without a target. */
  dialog: Dialog | null;
  error: string | null;
  reload: () => void;
}

const keyOf = (t: DialogTarget) => `${t.header}:${t.eventFile}:${t.kind}:${t.index}`;

/**
 * The dialog of `target` (`dialogOf`), read when the figure changes, when `revision` changes (undo, redo, a text written elsewhere) and on
 * `reload`. While a read for the same figure runs, the last answer stays; an answer that a newer read overtook never shows, and neither
 * does one for another figure.
 */
export function useDialog(target: DialogTarget | null, revision: number): FoundDialog {
  const [found, setFound] = useState<{ key: string; dialog: Dialog | null; error: string | null } | null>(null);
  const [tick, setTick] = useState(0);
  const latest = useRef(0);
  // The target of the latest render, for the read its key starts.
  const asked = useRef(target);
  asked.current = target;
  const key = target === null ? null : keyOf(target);
  useEffect(() => {
    const t = asked.current;
    if (key === null || t === null) return;
    const mine = ++latest.current;
    dialogOf(t.header, t.eventFile, t.kind, t.index).then(
      (dialog) => latest.current === mine && setFound({ key, dialog, error: null }),
      (e) => latest.current === mine && setFound({ key, dialog: null, error: errorText(e) }),
    );
  }, [key, revision, tick]);
  // Unmounted (or another figure): answers still on their way are dropped.
  useEffect(() => () => void ++latest.current, []);
  const reload = useCallback(() => setTick((n) => n + 1), []);
  const shown = found !== null && found.key === key ? found : null;
  return { dialog: shown?.dialog ?? null, error: shown?.error ?? null, reload };
}
