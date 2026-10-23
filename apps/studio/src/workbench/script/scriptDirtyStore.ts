/** How many script documents are unsaved; the script panel writes it, the dock tab "Script" reads it. */
export interface ScriptDirtyStore {
  subscribe(onChange: () => void): () => void;
  get(): number;
  set(count: number): void;
}

/** A store whose methods work taken off the object (as `useSyncExternalStore` calls them). */
export function createScriptDirtyStore(): ScriptDirtyStore {
  let count = 0;
  const listeners = new Set<() => void>();
  return {
    subscribe(onChange) {
      listeners.add(onChange);
      return () => void listeners.delete(onChange);
    },
    get: () => count,
    set(next) {
      if (next === count) return;
      count = next;
      for (const listener of [...listeners]) listener();
    },
  };
}

/** The app's one store: there is one script panel. */
export const scriptDirtyStore = createScriptDirtyStore();
