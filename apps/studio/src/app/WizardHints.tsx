import { useSyncExternalStore } from "react";
import { useTranslation } from "react-i18next";
import { Kbd } from "../ui/primitives";
import { wizardStatus } from "../workbench/wizards/wizardStatus";
import styles from "./WizardHints.module.css";

/**
 * The left of the status line while a wizard is open: its keys („↵ Weiter, ⌘[ Zurück, Esc Abbrechen“; on a closed step no way back and
 * „Esc Schließen“; on the result only Escape).
 */
export function WizardHints() {
  const { t } = useTranslation("app");
  const status = useSyncExternalStore(wizardStatus.subscribe, wizardStatus.get);
  if (status?.done) {
    return (
      <span className={styles.hints}>
        <Kbd keys="Esc" /> {t("wizardHints.close")}
      </span>
    );
  }
  if (status?.closed) {
    return (
      <span className={styles.hints}>
        <Kbd keys="↵" /> {t("wizardHints.next")}, <Kbd keys="Esc" /> {t("wizardHints.close")}
      </span>
    );
  }
  return (
    <span className={styles.hints}>
      <Kbd keys="↵" /> {t("wizardHints.next")}, <Kbd keys="⌘[" /> {t("wizardHints.back")}, <Kbd keys="Esc" /> {t("wizardHints.cancel")}
    </span>
  );
}

/** The middle of the status line while a wizard is open: where it stands ("Schritt 2 von 4: Anordnung"). */
export function WizardStep() {
  const { t } = useTranslation("app");
  const status = useSyncExternalStore(wizardStatus.subscribe, wizardStatus.get);
  if (status === null || status.done) return null;
  return <span>{t("wizardHints.step", { step: status.step + 1, count: status.count, title: status.title })}</span>;
}
