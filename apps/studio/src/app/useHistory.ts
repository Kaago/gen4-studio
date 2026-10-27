import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import { history } from "../api";
import type { HistoryDto } from "../api";
import type { EditBus } from "../bus/editBus";

export const HISTORY_DEBOUNCE_MS = 100;
const NO_HISTORY: HistoryDto = { undo: [], redo: [] };

const same = (a: readonly (string | null)[], b: readonly (string | null)[]) => a.length === b.length && a.every((label, i) => label === b[i]);
/** The same steps by the same hands: an agent's step with the user's label is another history. */
const sameHistory = (a: HistoryDto, b: HistoryDto) =>
  same(a.undo, b.undo) && same(a.redo, b.redo) && same(a.undo_agents ?? [], b.undo_agents ?? []) && same(a.redo_agents ?? [], b.redo_agents ?? []);

/**
 * The undo and redo labels (and who made each step) as last read, for every hinge to show: one reader (`useHistoryReader`) writes
 * them, any number of hinges read them (`useSyncExternalStore`). Writing the same labels and agents again keeps the object, so nobody
 * renders for nothing.
 */
export class HistoryFeed {
  private shown: HistoryDto = NO_HISTORY;
  private readonly listeners = new Set<() => void>();

  get = (): HistoryDto => this.shown;

  set(next: HistoryDto): void {
    if (sameHistory(this.shown, next)) return;
    this.shown = next.undo.length === 0 && next.redo.length === 0 ? NO_HISTORY : next;
    for (const listener of [...this.listeners]) listener();
  }

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };
}

/**
 * Reads the history into `feed`, debounced, after every topic of the edit bus except `headerReveal` and `sceneGroup` (which only move the list) and
 * whenever `revision` changes (App bumps it after an undo or redo and after every write it refreshes the dirty count for); never on
 * hover. `enabled` false (no project) empties it and reads nothing. A read that fails keeps what is shown. Holds no state of its own:
 * the component that calls it does not render for a new history, only the readers of `feed` do.
 */
export function useHistoryReader(feed: HistoryFeed, bus: EditBus, revision: number, enabled = true): void {
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);
  // Reads that were asked for while another was in flight: only the newest answer is used.
  const asked = useRef(0);

  useEffect(() => {
    if (!enabled) {
      feed.set(NO_HISTORY);
      return;
    }
    const read = () => {
      timer.current = null;
      const seq = ++asked.current;
      void Promise.resolve()
        .then(() => history())
        .then((next) => {
          if (seq === asked.current) feed.set(next);
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
    const unsubscribe = bus.subscribe((event) => event.topic !== "headerReveal" && event.topic !== "sceneGroup" && event.topic !== "savedFiles", schedule);
    return () => {
      unsubscribe();
      if (timer.current !== null) clearTimeout(timer.current);
      timer.current = null;
      // An answer still on its way belongs to a project or revision that is gone.
      asked.current++;
    };
  }, [feed, bus, revision, enabled]);
}

/** The undo and redo labels with a reader of its own (see `useHistoryReader`): for a single consumer. */
export function useHistory(bus: EditBus, revision: number, enabled = true): HistoryDto {
  const [feed] = useState(() => new HistoryFeed());
  useHistoryReader(feed, bus, revision, enabled);
  return useSyncExternalStore(feed.subscribe, feed.get);
}
