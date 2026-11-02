import { useSyncExternalStore } from "react";
import { OFF_STATUS } from "../api.live";
import type { LiveStatusDto } from "../api.live";

/** The latest play-mode status: what `subscribeLive` delivered or a command answered (`useLive` writes it). */
export interface LiveStore {
  /** The same object until a new status was set (as `useSyncExternalStore` wants it). */
  get(): LiveStatusDto;
  set(s: LiveStatusDto): void;
  subscribe(l: () => void): () => void;
}

/** Whether two JSON values (a status and its parts) are equal in every field. */
function sameJson(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null) return false;
  if (Array.isArray(a) !== Array.isArray(b)) return false;
  if (Array.isArray(a) && Array.isArray(b)) return a.length === b.length && a.every((v, i) => sameJson(v, b[i]));
  const ka = Object.keys(a);
  const kb = Object.keys(b);
  const ob = b as Record<string, unknown>;
  return ka.length === kb.length && ka.every((k) => k in ob && sameJson((a as Record<string, unknown>)[k], ob[k]));
}

/** Whether `a` shows what `b` shows: every field but the snapshot's revision and timing, which move with every refresh (as `same_view` in `live_hub.rs`). */
function sameView(a: LiveStatusDto, b: LiveStatusDto): boolean {
  if (a === b) return true;
  const strip = (s: LiveStatusDto): LiveStatusDto => (s.snapshot === null ? s : { ...s, snapshot: { ...s.snapshot, revision: 0, taken_ms: 0 } });
  return sameJson(strip(a), strip(b));
}

/**
 * A store whose methods work taken off the object. A status that shows the same as the current one changes nothing (`sameView`: the
 * browser mode's poll answers a fresh object every 500 or 1 000 ms, and each refresh moves the snapshot's revision; the app would render
 * again each time).
 */
export function createLiveStore(): LiveStore {
  let status = OFF_STATUS;
  const listeners = new Set<() => void>();
  return {
    get: () => status,
    set(next) {
      if (sameView(next, status)) return;
      status = next;
      for (const listener of [...listeners]) listener();
    },
    subscribe(listener) {
      listeners.add(listener);
      return () => void listeners.delete(listener);
    },
  };
}

/** The app's one store. */
export const liveStore = createLiveStore();

/** The latest status; renders again when it changes. */
export function useLiveStatus(): LiveStatusDto {
  return useSyncExternalStore(liveStore.subscribe, liveStore.get);
}

/** The live values for the list and the detail: `null` outside the snapshot (unknown, not 0). */
export interface LiveValues {
  /** The snapshot's revision when the shown status last changed (a refresh with the same values keeps the earlier status, `sameView`). */
  revision: number;
  /** Whether flag `n` is set. */
  flag(n: number): boolean | null;
  /** The value of variable `v` (from 0x4000). */
  var(v: number): number | null;
}

const VAR_BASE = 0x4000;

/** The values of the snapshot; `null` unless the state is `live` or `halted` with a snapshot. */
export function liveValuesOf(s: LiveStatusDto): LiveValues | null {
  const snapshot = s.snapshot;
  if ((s.state !== "live" && s.state !== "halted") || snapshot === null) return null;
  return {
    revision: snapshot.revision,
    flag(n) {
      if (!Number.isInteger(n) || n < 0) return null;
      const byte = snapshot.flags[Math.floor(n / 8)];
      return byte === undefined ? null : (byte & (1 << n % 8)) !== 0;
    },
    var(v) {
      if (!Number.isInteger(v) || v < VAR_BASE) return null;
      return snapshot.vars[v - VAR_BASE] ?? null;
    },
  };
}
