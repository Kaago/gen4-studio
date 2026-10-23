import { useEffect, useRef, useState } from "react";
import { history } from "../api";
import type { HistoryDto } from "../api";
import type { EditBus } from "../bus/editBus";

export const HISTORY_DEBOUNCE_MS = 100;
const NO_HISTORY: HistoryDto = { undo: [], redo: [] };

const same = (a: readonly string[], b: readonly string[]) => a.length === b.length && a.every((label, i) => label === b[i]);

/**
 * The undo and redo labels the hinge shows. Read again, debounced, after every topic of the edit bus except `headerReveal` (which only
 * scrolls the list) and whenever `revision` changes (App bumps it after an undo or redo and after every write it refreshes the dirty
 * count for); never on hover. `enabled` false (no project) shows an empty history and reads nothing. A read that fails keeps what is shown.
 */
export function useHistory(bus: EditBus, revision: number, enabled = true): HistoryDto {
  const [shown, setShown] = useState<HistoryDto>(NO_HISTORY);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Reads that were asked for while another was in flight: only the newest answer is used.
  const asked = useRef(0);

  useEffect(() => {
    if (!enabled) {
      setShown((before) => (before === NO_HISTORY ? before : NO_HISTORY));
      return;
    }
    const read = () => {
      timer.current = null;
      const seq = ++asked.current;
      void Promise.resolve()
        .then(() => history())
        .then((next) => {
          if (seq !== asked.current) return;
          setShown((before) => (same(before.undo, next.undo) && same(before.redo, next.redo) ? before : next));
        })
        .catch(() => {
          /* keeps the last history; the next change reads again */
        });
    };
    const schedule = () => {
      if (timer.current !== null) clearTimeout(timer.current);
      timer.current = setTimeout(read, HISTORY_DEBOUNCE_MS);
    };
    schedule();
    const unsubscribe = bus.subscribe((event) => event.topic !== "headerReveal", schedule);
    return () => {
      unsubscribe();
      if (timer.current !== null) clearTimeout(timer.current);
      timer.current = null;
      // An answer still on its way belongs to a project or revision that is gone.
      asked.current++;
    };
  }, [bus, revision, enabled]);

  return shown;
}
