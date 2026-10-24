import { useEffect, useState } from "react";
import { errorText, isCmdError } from "../../api";
import type { CheckProblemDto, FixPlanDto } from "../../api";
import { findingText } from "../../checks/messages";
import type { ChecksApi } from "../../checks/useChecks";
import { Badge, BeforeAfter, Dialog, Notice } from "../../ui/primitives";
import { TERMS, id4 } from "../terms";
import type { DialogAction } from "../../ui/primitives";
import styles from "./FixDialog.module.css";

/** Where the dialog stands: the preview is read, shown, applied; or the fix left the finding, found it stale, or could not run. */
type Phase =
  | { kind: "loading" }
  | { kind: "ready"; plan: FixPlanDto }
  | { kind: "applying"; plan: FixPlanDto }
  | { kind: "remaining"; plan: FixPlanDto }
  | { kind: "stale" }
  | { kind: "failed"; title: string; text: string; plan: FixPlanDto | null };

const UNSAVED_ARCHIVE = /^text archive (\d+) has unsaved changes/;

/**
 * A refusal: „Befund ist nicht mehr aktuell“ for a finding whose subject changed since it was found; „Erst speichern“ when creating a script
 * meets an unsaved text archive (the compile rewrites text archives); else the kernel's reason with the way out by hand.
 */
function failure(e: unknown, plan: FixPlanDto | null): Phase {
  if (isCmdError(e) && e.code === "finding_stale") return { kind: "stale" };
  const why = errorText(e);
  const unsaved = UNSAVED_ARCHIVE.exec(why);
  if (unsaved) {
    const text = `${TERMS.textArchive.label} ${id4(Number(unsaved[1]))} hat ungespeicherte Änderungen. Speichere zuerst, dann lege das Script an: es wird kompiliert, und das schreibt Textarchive.`;
    return { kind: "failed", title: "Erst speichern", text, plan };
  }
  return { kind: "failed", title: "Lässt sich nicht automatisch beheben", text: `${/[.!?]$/.test(why) ? why : `${why}.`} Behebe den Befund von Hand.`, plan };
}

/**
 * „Beheben…“ of a finding (spec 3b.4 §6): the preview of its first fix (`plan_fix`) as before and after per changed asset, „Nicht rückgängig“
 * for a fix undo cannot take back, then one step (`apply_fix`). It closes when the finding is gone; it stays with „Nicht behoben“ when the
 * check still reports it, and says „Befund ist nicht mehr aktuell“ (and offers to check again) when its subject changed meanwhile.
 */
export function FixDialog({ finding, checks, onClose }: { finding: CheckProblemDto; checks: ChecksApi; onClose: () => void }) {
  const fix = finding.fixes[0];
  const [phase, setPhase] = useState<Phase>({ kind: "loading" });

  useEffect(() => {
    if (!fix) return;
    let current = true;
    checks.planFix(finding, fix.id).then(
      (plan) => current && setPhase({ kind: "ready", plan }),
      (e: unknown) => current && setPhase(failure(e, null)),
    );
    return () => {
      current = false;
    };
  }, [checks, finding, fix]);

  if (!fix) return null;

  const apply = async (plan: FixPlanDto) => {
    setPhase({ kind: "applying", plan });
    try {
      const result = await checks.applyFix(finding, fix.id);
      if (result.remaining) setPhase({ kind: "remaining", plan });
      else onClose();
    } catch (e) {
      setPhase(failure(e, plan));
    }
  };

  const close: DialogAction = { label: "Schließen", onClick: onClose };
  const actions: DialogAction[] =
    phase.kind === "ready"
      ? [{ label: "Abbrechen", onClick: onClose }, { label: fix.label, kind: "primary", onClick: () => void apply(phase.plan) }]
      : phase.kind === "stale"
        ? [close, { label: "Erneut prüfen", kind: "primary", onClick: () => (void checks.recheck(), onClose()) }]
        : phase.kind === "loading" || phase.kind === "applying"
          ? [{ label: "Abbrechen", onClick: onClose }]
          : [close];
  const plan = "plan" in phase ? phase.plan : null;

  return (
    <Dialog open onOpenChange={(open) => !open && onClose()} title={finding.label} description={findingText(finding)} actions={actions}>
      <div className={styles.body} data-fix={fix.id}>
        {phase.kind === "loading" && <p role="status">Lade Vorschau …</p>}
        {plan !== null && (
          <>
            {plan.irreversible && (
              <p className={styles.irreversible}>
                <Badge kind="irreversible" /> Wird sofort geschrieben, ⌘Z nimmt es nicht zurück.
              </p>
            )}
            {plan.changes.map((change) => (
              <BeforeAfter key={`${change.asset.kind}:${change.asset.index}`} label={change.label} before={change.before} after={change.after} />
            ))}
          </>
        )}
        {phase.kind === "applying" && <p role="status">Behebe …</p>}
        {phase.kind === "remaining" && (
          <Notice kind="warning" title="Nicht behoben">
            Die Änderung ist geschrieben, aber die Prüfung meldet den Befund noch. Sieh dir die Stelle an und behebe ihn von Hand.
          </Notice>
        )}
        {phase.kind === "stale" && (
          <Notice kind="error" title="Befund ist nicht mehr aktuell">
            Die Stelle hat sich seit der Prüfung geändert, es wurde nichts geschrieben. Prüfe erneut und behebe den Befund dann.
          </Notice>
        )}
        {phase.kind === "failed" && (
          <Notice kind="error" title={phase.title}>
            {phase.text}
          </Notice>
        )}
      </div>
    </Dialog>
  );
}
