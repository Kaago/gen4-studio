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

/** A store whose methods work taken off the object. */
export function createLiveStore(): LiveStore {
  let status = OFF_STATUS;
  const listeners = new Set<() => void>();
  return {
    get: () => status,
    set(next) {
      if (next === status) return;
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
  /** The snapshot's revision: changes with every new reading. */
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
