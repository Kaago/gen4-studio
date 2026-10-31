import { useSyncExternalStore } from "react";
import { Kbd } from "../ui/primitives";
import { wizardStatus } from "../workbench/wizards/wizardStatus";
import styles from "./WizardHints.module.css";

/**
 * The left of the status line while a wizard is open: its keys ("↵ Weiter, ⌘[ Zurück, Esc Abbrechen"; on a closed step no way back and
 * "Esc Schließen"; on the result only Escape).
 */
export function WizardHints() {
  const status = useSyncExternalStore(wizardStatus.subscribe, wizardStatus.get);
  if (status?.done) {
    return (
      <span className={styles.hints}>
        <Kbd keys="Esc" /> Schließen
      </span>
    );
  }
  if (status?.closed) {
    return (
      <span className={styles.hints}>
        <Kbd keys="↵" /> Weiter, <Kbd keys="Esc" /> Schließen
      </span>
    );
  }
  return (
    <span className={styles.hints}>
      <Kbd keys="↵" /> Weiter, <Kbd keys="⌘[" /> Zurück, <Kbd keys="Esc" /> Abbrechen
    </span>
  );
}

/** The middle of the status line while a wizard is open: where it stands ("Schritt 2 von 4: Anordnung"). */
export function WizardStep() {
  const status = useSyncExternalStore(wizardStatus.subscribe, wizardStatus.get);
  if (status === null || status.done) return null;
  return (
    <span>
      Schritt {status.step + 1} von {status.count}: {status.title}
    </span>
  );
}
