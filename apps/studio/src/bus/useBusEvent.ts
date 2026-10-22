import { useEffect, useRef, useState } from "react";
import { enqueue } from "../reload";
import type { RefreshQueue } from "../reload";
import { matchesFilter } from "./editBus";
import type { EditBus, EditEvent, EditFilter, EditTopic } from "./editBus";

/**
 * Calls `fn` for every event of `bus` that matches `filter`, from mount to unmount. Subscribes
 * once per bus: `filter` and `fn` are read through refs, so a new closure every render neither
 * resubscribes nor calls a stale callback.
 */
export function useBusEvent(bus: EditBus, filter: EditFilter, fn: (event: EditEvent) => void) {
  const latestFilter = useRef(filter);
  latestFilter.current = filter;
  const latestFn = useRef(fn);
  latestFn.current = fn;
  useEffect(
    () =>
      bus.subscribe(
        (event) => matchesFilter(latestFilter.current, event),
        (event) => latestFn.current(event),
      ),
    [bus],
  );
}

/**
 * A number that changes with every matching event: the key an editor lists in its effect
 * dependencies to read its asset again. Local to the consumer, it starts at 0 on every mount.
 */
export function useBusRevision(bus: EditBus, filter: EditFilter): number {
  const [revision, setRevision] = useState(0);
  useBusEvent(bus, filter, () => setRevision((r) => r + 1));
  return revision;
}

/**
 * The events of one topic as the scene's `RefreshQueue`: every event is appended in the state
 * updater, so several events before the next render are all kept, in order (see `enqueue`). The
 * scene's effect drains the queue with `pendingSince` / `drainWhenReady`.
 */
export function useBusQueue<K extends EditTopic, T>(bus: EditBus, topic: K, toItem: (event: Extract<EditEvent, { topic: K }>) => T): RefreshQueue<T> | null {
  const [queue, setQueue] = useState<RefreshQueue<T> | null>(null);
  useBusEvent(bus, topic, (event) => {
    const item = toItem(event as Extract<EditEvent, { topic: K }>);
    setQueue((previous) => enqueue(previous, item));
  });
  return queue;
}
