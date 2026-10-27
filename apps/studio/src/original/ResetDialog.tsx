import { useEffect, useRef, useState } from "react";
import { planReset, resetErrorText, resetToOriginal, targetLabel } from "../api";
import type { HeaderFamily, ResetPlan, ResetResult, ResetTarget } from "../api";
import { BeforeAfter, Checkbox, Dialog, Notice } from "../ui/primitives";
import type { DialogAction } from "../ui/primitives";
import { headerFieldSpecs } from "../workbench/inspector/fields";
import styles from "./ResetDialog.module.css";

export type ResetDialogProps = {
  target: ResetTarget;
  /** The base ROM's name („Platin DE (Testprojekt)“) for the description; null while the status is not ready (no description then). */
  baseLabel: string | null;
  /** The project's header family, for the German names of the header fields; null: the serde names. */
  family: HeaderFamily | null;
  /** The script editor holds unsaved changes of this script file: they are lost. */
  scriptDirty?: boolean;
  /** An asset of the target has unsaved edits: they are replaced (⌘Z brings them back). */
  dirty?: boolean;
  /** Where focus goes once the dialog has closed (`Dialog.finalFocus`); undefined: back to the element that had it. */
  finalFocus?: () => HTMLElement | null;
  /** „Abbrechen“, „Schließen“, Escape, and after the reset. */
  onClose: () => void;
  /** The reset went in: its result and the entity's name („Karte 0003“). */
  onReset: (result: ResetResult, label: string) => void | Promise<void>;
};

/** The German name of header field `field` (its serde name where the family has none). */
function fieldLabel(family: HeaderFamily | null, field: string): string {
  return (family && headerFieldSpecs(family).find((spec) => spec.key === field)?.label) || field;
}

/**
 * „{label} auf Original zurücksetzen?“: asks the kernel for the plan, shows it before and after per change under its label, long lines
 * wrapped (a header instead: one checkbox per differing field, all on), names what else happens (written at once, unsaved changes
 * replaced or lost), and resets on „Zurücksetzen“. A refused plan or reset shows the reason; after a failed reset the dialog stays open
 * and can try again.
 */
export function ResetDialog({ target, baseLabel, family, scriptDirty = false, dirty = false, finalFocus, onClose, onReset }: ResetDialogProps) {
  const [plan, setPlan] = useState<ResetPlan | null>(null);
  const [planError, setPlanError] = useState<string | null>(null);
  const [unchecked, setUnchecked] = useState<ReadonlySet<string>>(() => new Set());
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // Read by a click that comes before the render of the one before (a double click).
  const busy = useRef(false);

  useEffect(() => {
    let alive = true;
    planReset(target).then(
      (next) => alive && setPlan(next),
      (e: unknown) => alive && setPlanError(resetErrorText(e)),
    );
    return () => {
      alive = false;
    };
  }, [target]);

  const label = plan?.label ?? targetLabel(target);
  // A header shows its differing fields as checkboxes with their German names; its comparison lines (the format's names) would show
  // them twice.
  const isHeader = target.kind === "header";
  const header = plan !== null && plan.fields.length > 0;
  const chosen = plan?.fields.filter((f) => !unchecked.has(f.field)).map((f) => f.field) ?? [];

  const reset = async () => {
    if (plan === null || busy.current) return;
    busy.current = true;
    setRunning(true);
    setError(null);
    try {
      const result = header ? await resetToOriginal(target, chosen) : await resetToOriginal(target);
      void onReset(result, label);
      onClose();
    } catch (e) {
      setError(resetErrorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  const toggle = (field: string, on: boolean) =>
    setUnchecked((before) => {
      const next = new Set(before);
      if (on) next.delete(field);
      else next.add(field);
      return next;
    });

  const closeOnly: DialogAction[] = [{ label: "Schließen", onClick: onClose }];
  const actions: DialogAction[] =
    planError !== null || plan?.same
      ? closeOnly
      : [
          { label: "Abbrechen", onClick: onClose, disabled: running },
          { label: "Zurücksetzen", kind: "primary", onClick: () => void reset(), disabled: plan === null || running || (header && chosen.length === 0) },
        ];

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy.current) onClose();
      }}
      title={`${label} auf Original zurücksetzen?`}
      description={baseLabel === null ? undefined : `Stellt den Stand der Basis-ROM ${baseLabel} wieder her. ⌘Z nimmt es zurück.`}
      data={{ "data-reset-dialog": "" }}
      finalFocus={finalFocus}
      actions={actions}
    >
      <div className={styles.content}>
        {planError !== null ? (
          <Notice kind="error">{planError}</Notice>
        ) : plan === null ? (
          <p role="status" className={styles.wait}>
            Lade Vergleich …
          </p>
        ) : plan.same ? (
          <Notice kind="info">Schon im Stand der Basis-ROM.</Notice>
        ) : (
          <>
            {scriptDirty && <Notice kind="warning">Ungespeicherte Änderungen an dieser Scriptdatei gehen verloren.</Notice>}
            {dirty && <Notice kind="info">Auch ungespeicherte Änderungen werden ersetzt; ⌘Z stellt sie wieder her.</Notice>}
            {plan.writes_files && <Notice kind="info">Wird sofort geschrieben; ⌘Z nimmt es zurück.</Notice>}
            {!isHeader &&
              plan.changes.map((change, i) => (
                <div key={`${i}-${change.label}`} className={styles.change} data-change="">
                  {/* Each change named above its comparison („Karte 0003: 1 Zelle der Bewegungsrechte anders“, a Pokémon's three parts). */}
                  <p className={styles.changeLabel}>{change.label}</p>
                  <BeforeAfter label={change.label} before={change.before} after={change.after} wrap />
                </div>
              ))}
            {header && (
              <ul className={styles.fields}>
                {plan.fields.map((f) => (
                  <li key={f.field} className={styles.field} data-header-field={f.field}>
                    <Checkbox label={fieldLabel(family, f.field)} checked={!unchecked.has(f.field)} onChange={(on) => toggle(f.field, on)} disabled={running} />
                    <span className={styles.values} data-values="">
                      {f.before} → {f.after}
                    </span>
                  </li>
                ))}
              </ul>
            )}
            {running && (
              <p role="status" className={styles.wait}>
                Setze zurück …
              </p>
            )}
            {error !== null && <Notice kind="error">{error}</Notice>}
          </>
        )}
      </div>
    </Dialog>
  );
}
