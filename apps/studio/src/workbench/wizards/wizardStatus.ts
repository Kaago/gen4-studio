/** Where the open wizard stands: the step shown (from 0), how many there are and its title; `done` once its result shows. */
export interface WizardStatus {
  step: number;
  count: number;
  title: string;
  done: boolean;
}

/** The open wizard's frame writes it, the status line reads it (null: no wizard is open). */
export interface WizardStatusStore {
  subscribe(onChange: () => void): () => void;
  get(): WizardStatus | null;
  set(status: WizardStatus | null): void;
}

const same = (a: WizardStatus | null, b: WizardStatus | null) =>
  a === b || (a !== null && b !== null && a.step === b.step && a.count === b.count && a.title === b.title && a.done === b.done);

/** A store whose methods work taken off the object (as `useSyncExternalStore` calls them). */
export function createWizardStatusStore(): WizardStatusStore {
  let status: WizardStatus | null = null;
  const listeners = new Set<() => void>();
  return {
    subscribe(onChange) {
      listeners.add(onChange);
      return () => void listeners.delete(onChange);
    },
    get: () => status,
    set(next) {
      if (same(next, status)) return;
      status = next;
      for (const listener of [...listeners]) listener();
    },
  };
}

/** The app's one store: one wizard is open at a time. */
export const wizardStatus = createWizardStatusStore();
