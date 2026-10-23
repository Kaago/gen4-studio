// The toast store: a plain module (no React) that `ToastHost` reads through `useSyncExternalStore`.
// Rules: info vanishes after INFO_MS (hover pauses the clock); an error stays until closed, with or without actions;
// progress stays until `progress(key, null)` and a second one with the same key replaces the first; an error that says what an open
// error already says (same text, same action labels) bumps that one instead of stacking; at most four toasts.

export const INFO_MS = 5000;
const MAX_TOASTS = 4;

export type ToastAction = { label: string; onClick: () => void };
export type ToastKind = "info" | "error" | "progress";
export type Toast = {
  readonly id: string;
  readonly kind: ToastKind;
  readonly text: string;
  readonly actions: readonly ToastAction[];
  /** Progress toasts only: the key that identifies the toast across updates. */
  readonly key?: string;
};
export type ToastOptions = { actions?: readonly ToastAction[] };

type Clock = { timer: ReturnType<typeof setTimeout> | undefined; startedAt: number; remaining: number };

let toasts: readonly Toast[] = [];
let nextId = 1;
const clocks = new Map<string, Clock>();
const listeners = new Set<() => void>();

function emit(next: readonly Toast[]): void {
  toasts = next;
  for (const listener of [...listeners]) listener();
}

function stopClock(id: string): void {
  const clock = clocks.get(id);
  if (clock?.timer !== undefined) clearTimeout(clock.timer);
  clocks.delete(id);
}

function startClock(id: string, ms: number): void {
  const clock: Clock = { timer: undefined, startedAt: Date.now(), remaining: ms };
  clock.timer = setTimeout(() => dismiss(id), ms);
  clocks.set(id, clock);
}

/**
 * Drop order when more than four are open: the oldest info, then the oldest progress, then the oldest error. The toast that was just
 * added (`fresh`) is never the one to go: a full stack of errors makes room for it by dropping its oldest error.
 */
function trim(list: readonly Toast[], fresh: string): readonly Toast[] {
  const kept = [...list];
  while (kept.length > MAX_TOASTS) {
    const victim = (["info", "progress", "error"] as const).map((kind) => kept.findIndex((t) => t.kind === kind && t.id !== fresh)).find((i) => i >= 0) ?? 0;
    const [dropped] = kept.splice(victim, 1);
    if (dropped) stopClock(dropped.id);
  }
  return kept;
}

function add(kind: ToastKind, text: string, options?: ToastOptions, key?: string): string {
  const id = `toast-${nextId++}`;
  const toast: Toast = { id, kind, text, actions: options?.actions ?? [], ...(key === undefined ? {} : { key }) };
  if (kind === "info") startClock(id, INFO_MS);
  emit(trim([...toasts, toast], id));
  return id;
}

function dismiss(id: string): void {
  if (!toasts.some((t) => t.id === id)) return;
  stopClock(id);
  emit(toasts.filter((t) => t.id !== id));
}

export const toast = {
  info: (text: string, options?: ToastOptions): string => add("info", text, options),
  /** An error equal to an open one (text and action labels) takes that one's place at the end, with the new actions, and keeps its id. */
  error(text: string, options?: ToastOptions): string {
    const actions = options?.actions ?? [];
    const same = toasts.find((t) => t.kind === "error" && t.text === text && t.actions.length === actions.length && t.actions.every((a, i) => a.label === actions[i]?.label));
    if (same === undefined) return add("error", text, options);
    emit([...toasts.filter((t) => t !== same), { ...same, actions }]);
    return same.id;
  },
  /** Replaces the toast with the same key; `null` removes it. */
  progress(key: string, text: string | null): void {
    const existing = toasts.find((t) => t.kind === "progress" && t.key === key);
    if (text === null) {
      if (existing) dismiss(existing.id);
      return;
    }
    if (existing) {
      if (existing.text !== text) emit(toasts.map((t) => (t === existing ? { ...existing, text } : t)));
      return;
    }
    add("progress", text, undefined, key);
  },
  dismiss,
};

/** Holds the 5 s clock of an info toast (hover, focus inside). */
export function pauseToast(id: string): void {
  const clock = clocks.get(id);
  if (!clock || clock.timer === undefined) return;
  clearTimeout(clock.timer);
  clock.timer = undefined;
  clock.remaining = Math.max(0, clock.remaining - (Date.now() - clock.startedAt));
}

/** Runs the rest of the clock that `pauseToast` stopped. */
export function resumeToast(id: string): void {
  const clock = clocks.get(id);
  if (!clock || clock.timer !== undefined) return;
  clock.startedAt = Date.now();
  clock.timer = setTimeout(() => dismiss(id), clock.remaining);
}

export function subscribeToasts(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** The current list; the same array until something changes (the `useSyncExternalStore` snapshot). */
export function listToasts(): readonly Toast[] {
  return toasts;
}

/** Clears everything, including running clocks (tests). */
export function resetToasts(): void {
  for (const id of [...clocks.keys()]) stopClock(id);
  toasts = [];
  nextId = 1;
  for (const listener of [...listeners]) listener();
}
