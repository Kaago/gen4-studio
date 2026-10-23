import { useCallback, useSyncExternalStore } from "react";

/**
 * Which inspector sections are open, kept outside the editors: a hover replaces the pinned record's editor with another one and
 * back, which mounts a fresh editor each time, and a section the user folded (or "Technisch" they opened) must not spring back.
 * Keys are `<editor>:<section>` (one key for "Technisch", which every editor shows). A key that was never toggled has its default.
 */
const state = new Map<string, boolean>();
const listeners = new Set<() => void>();

function subscribe(listener: () => void): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

function set(key: string, open: boolean): void {
  state.set(key, open);
  for (const listener of [...listeners]) listener();
}

/** Whether the section `key` is open and a function that flips it. */
export function useSectionOpen(key: string, defaultOpen: boolean): [boolean, () => void] {
  const open = useSyncExternalStore(subscribe, () => state.get(key) ?? defaultOpen);
  const toggle = useCallback(() => set(key, !(state.get(key) ?? defaultOpen)), [key, defaultOpen]);
  return [open, toggle];
}

/** For a list of sections whose number is not fixed: `isOpen(key)` and `toggle(key)`, all sections default to open. */
export function useSectionFolds(): { isOpen: (key: string) => boolean; toggle: (key: string) => void } {
  // The snapshot is the set of folded keys as a string, so a change of any key renders the caller once.
  useSyncExternalStore(subscribe, () => [...state].filter(([, open]) => !open).map(([key]) => key).join("\n"));
  const isOpen = (key: string) => state.get(key) ?? true;
  const toggle = (key: string) => set(key, !(state.get(key) ?? true));
  return { isOpen, toggle };
}

/** Forgets every choice (tests). */
export function resetSectionFolds(): void {
  state.clear();
  for (const listener of [...listeners]) listener();
}
