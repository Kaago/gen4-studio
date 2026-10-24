import type { DataChangedEvent } from "../data/dataBus";

/**
 * What happened to the data behind the panels and the scene. The editors and panels re-read their
 * asset on the `...Data` events; the scene refreshes on the `...Refetch` and `modelsReload` events.
 */
export type EditEvent =
  /** The header table changed behind the header editor's back (undo, redo, reload). */
  | { topic: "headerTable" }
  /** Matrix data may have changed behind the matrix panel and the cell editor (edit, undo, redo, reload). */
  | { topic: "matrixData" }
  /** Area data may have changed behind the area editor (undo, redo, reload). */
  | { topic: "areaData" }
  /** Buildings may have changed behind the building editor (a drag, add, delete, undo, redo, reload). */
  | { topic: "buildingData" }
  /** Events may have changed behind the event editor (a drag, add, delete, undo, redo, reload). */
  | { topic: "eventData" }
  /** The scene refetches this map (overlay colours and buildings of all its cells). */
  | { topic: "mapRefetch"; map: number }
  /** The scene reads this map's buildings again; `retryModel` is a model whose earlier failure is forgotten. */
  | { topic: "buildingsRefetch"; map: number; retryModel: number | null }
  /** The scene reads this event file's events again. */
  | { topic: "eventsRefetch"; file: number }
  /** The scene reloads its models (the tilesets of the shown area data changed). */
  | { topic: "modelsReload" }
  /** A text archive may have changed behind the text panel (undo, redo, reload); `archive` null: any of them (a reload after a stale file). */
  | { topic: "textData"; archive: number | null }
  /**
   * Project files the language server should read again (LSP `workspace/didChangeWatchedFiles`): a saved text archive
   * (its message ids), the project config after an adoption. `type` is the LSP file change type: 1 created, 2 changed.
   * Nobody listens while the server is not running.
   */
  | { topic: "lspFilesChanged"; changes: { path: string; type: 1 | 2 }[] }
  /** The header list scrolls to the selection (a link led to a header). */
  | { topic: "headerReveal" }
  /** A data member (species, move, item, …) may have changed behind the data areas (see `data/dataBus.ts`). */
  | DataChangedEvent;

export type EditTopic = EditEvent["topic"];

/** Which events a subscriber wants: one topic, several, or any predicate. */
export type EditFilter = EditTopic | readonly EditTopic[] | ((event: EditEvent) => boolean);

export function matchesFilter(filter: EditFilter, event: EditEvent): boolean {
  if (typeof filter === "string") return filter === event.topic;
  if (typeof filter === "function") return filter(event);
  return filter.includes(event.topic);
}

export interface EditBus {
  /** Hands the event to every matching subscriber, in subscription order, before it returns (see `createEditBus`). */
  emit(event: EditEvent): void;
  /** Calls `fn` with every later matching event; the returned function ends the subscription. */
  subscribe(filter: EditFilter, fn: (event: EditEvent) => void): () => void;
}

interface Subscription {
  filter: EditFilter;
  fn: (event: EditEvent) => void;
  active: boolean;
}

/**
 * A pure, synchronous emitter, with no React in it. Every emitted event reaches every matching
 * subscriber (nothing is collapsed or dropped, two events for different maps in one batch both
 * arrive) and in the order they were emitted: an event emitted by a subscriber while another one
 * is being delivered waits until that delivery is done. A subscriber that unsubscribes gets
 * nothing more, not even the rest of the current delivery; one that subscribes during a delivery
 * starts with the next event. A subscriber that throws is reported to `onError` and does not
 * keep the others from their events; neither does an `onError` that throws itself.
 */
export function createEditBus(onError: (error: unknown) => void = (e) => console.error(e)): EditBus {
  let subscriptions: Subscription[] = [];
  const queued: EditEvent[] = [];
  let delivering = false;

  function deliver(event: EditEvent) {
    // A copy: subscribing or unsubscribing during the delivery must not disturb the loop.
    for (const s of [...subscriptions]) {
      if (!s.active || !matchesFilter(s.filter, event)) continue;
      try {
        s.fn(event);
      } catch (e) {
        // A throwing error handler must not abort the delivery: the events queued behind this one would be stranded.
        try {
          onError(e);
        } catch {
          /* nowhere left to report it */
        }
      }
    }
  }

  return {
    emit(event) {
      queued.push(event);
      if (delivering) return;
      delivering = true;
      try {
        for (let next = queued.shift(); next; next = queued.shift()) deliver(next);
      } finally {
        delivering = false;
      }
    },
    subscribe(filter, fn) {
      const subscription: Subscription = { filter, fn, active: true };
      subscriptions = [...subscriptions, subscription];
      return () => {
        subscription.active = false;
        subscriptions = subscriptions.filter((s) => s !== subscription);
      };
    },
  };
}
