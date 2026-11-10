import { useTranslation } from "react-i18next";
import type { HeaderPatchDto } from "../../api";
import { t } from "../../i18n";
import { Badge, Checkbox, Notice } from "../../ui/primitives";
import styles from "./WizardStep.module.css";

/** The consent Bauen asks for when the game lacks the Dynamic Headers patch (decision „Patch-Zeile“, word for word). */
export const patchConsentText = (): string => t("workbench:patchConsent.consent");

/**
 * Whether a wizard in Bauen may create: the patch is in and sound, or it is missing and the user agreed to apply it while creating.
 * Not while the state is unknown (null), for a broken patch and for a game without one.
 */
export function patchReady(patch: HeaderPatchDto | null, consent: boolean): boolean {
  if (patch === null || !patch.supported) return false;
  return patch.applied ? !patch.repairable : consent;
}

interface Props {
  /** The patch's state; null while it is read (or when reading it failed, see `error`). */
  patch: HeaderPatchDto | null;
  /** Why the state could not be read. */
  error?: string | null;
  checked: boolean;
  onChange: (checked: boolean) => void;
}

/**
 * The patch line of a summary in Bauen: with the patch missing, the badge „Nicht rückgängig“ and the one-time consent; with a broken
 * patch, the way to repair it; nothing when the patch is in. While the state is read it says so, and why when that failed.
 */
export function PatchConsent({ patch, error = null, checked, onChange }: Props) {
  // Renders the line again when the language changes; the words below are read at render time.
  useTranslation("workbench");
  if (patch === null) return error !== null ? <Notice kind="error">{error}</Notice> : <p className={styles.note}>{t("workbench:patchConsent.checking")}</p>;
  if (!patch.supported) {
    return (
      <div className={styles.actions}>
        <Badge kind="ptHgssOnly" />
        {/* Bauen's words; the patch's own name says „Header“. */}
        <span className={styles.note}>{t("workbench:patchConsent.unsupported")}</span>
      </div>
    );
  }
  if (patch.applied) return patch.repairable ? <Notice kind="error">{t("workbench:patchConsent.broken")}</Notice> : null;
  return (
    <div className={styles.actions} data-patch-consent="">
      <Badge kind="irreversible" />
      <Checkbox label={patchConsentText()} checked={checked} onChange={onChange} />
    </div>
  );
}
