import type { WorldEditRequest, WorldReport } from "../../api.world";

/** Strokes on their way to the kernel, one after another: each `world_edit` goes once the one before it is answered. */
export interface StrokeQueue {
  /** Queues a stroke; resolves with its report, rejects with its refusal (the next stroke goes anyway). */
  push(req: WorldEditRequest): Promise<WorldReport>;
  /** Strokes pushed and not yet answered. */
  pending(): number;
}

/** A queue that sends with `send` and tells `onPending` the number of unanswered strokes on every change. */
export function createStrokeQueue(
  send: (req: WorldEditRequest) => Promise<WorldReport>,
  onPending: (n: number) => void,
): StrokeQueue {
  let tail: Promise<unknown> = Promise.resolve();
  let count = 0;
  const changed = (by: number) => {
    count += by;
    onPending(count);
  };
  return {
    push(req) {
      changed(1);
      const result = tail.then(() => send(req));
      const done = result.finally(() => changed(-1));
      // The next stroke waits for this one, answered or refused.
      tail = done.catch(() => undefined);
      return done;
    },
    pending: () => count,
  };
}
