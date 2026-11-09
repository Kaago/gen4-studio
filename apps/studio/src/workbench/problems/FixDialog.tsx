import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { errorText, isCmdError } from "../../api";
import type { CheckProblemDto, FixPlanDto } from "../../api";
import { findingLabel, findingText } from "../../checks/messages";
import type { ChecksApi } from "../../checks/useChecks";
import type { Depth } from "../../ui/frame/shellState";
import { t as translate } from "../../i18n";
import { Badge, BeforeAfter, Dialog, Notice } from "../../ui/primitives";
import { id4, termLabel } from "../terms";
import type { DialogAction } from "../../ui/primitives";
import styles from "./FixDialog.module.css";

/** Where the dialog stands: the preview is read, shown, applied; or the fix left the finding, found it stale, or could not run. */
type Phase =
  | { kind: "loading" }
  | { kind: "ready"; plan: FixPlanDto }
  | { kind: "applying"; plan: FixPlanDto }
  | { kind: "remaining"; plan: FixPlanDto }
  | { kind: "stale" }
  /** Creating the script met an unsaved text archive: „Speichern und anlegen“ saves the project, then applies the fix again. */
  | { kind: "unsaved"; archive: number; plan: FixPlanDto }
  | { kind: "failed"; title: string; text: string; plan: FixPlanDto | null };

const UNSAVED_ARCHIVE = /^text archive (\d+) has unsaved changes/;

/** The kernel's stand-in for the time a moved file's backup folder gets (UTC, taken when it moves), in the dialog's words. */
const BACKUP_TIME = "<time>";
const backupPath = (to: string) => to.replace(BACKUP_TIME, translate("workbench:fixDialog.backupTime"));

/**
 * A refusal: „Befund ist nicht mehr aktuell“ for a finding whose subject changed since it was found; „Erst speichern“ with „Speichern und
 * anlegen“ when creating a script meets an unsaved text archive (the compile rewrites text archives); else a German sentence with the way
 * out by hand and the kernel's (English) reason as a detail in brackets.
 */
function failure(e: unknown, plan: FixPlanDto | null): Phase {
  if (isCmdError(e) && e.code === "finding_stale") return { kind: "stale" };
  const why = errorText(e);
  const unsaved = UNSAVED_ARCHIVE.exec(why);
  if (unsaved && plan !== null) return { kind: "unsaved", archive: Number(unsaved[1]), plan };
  return { kind: "failed", title: translate("workbench:fixDialog.failedTitle"), text: translate("workbench:fixDialog.failedText", { why: why.replace(/[.!?]$/, "") }), plan };
}

/**
 * „Beheben…“ of a finding (spec 3b.4 §6): the preview of its first fix (`plan_fix`) as before and after per changed asset and a line „{path} →
 * {to}“ per file it moves, „Nicht rückgängig“
 * for a fix undo cannot take back, then one step (`apply_fix`). It closes when the finding is gone; it stays with „Nicht behoben“ when the
 * check still reports it, and says „Befund ist nicht mehr aktuell“ (and offers to check again) when its subject changed meanwhile.
 */
export function FixDialog({ finding, checks, depth, onClose }: { finding: CheckProblemDto; checks: ChecksApi; depth: Depth; onClose: () => void }) {
  const { t } = useTranslation("workbench");
  const fix = finding.fixes[0];
  const [phase, setPhase] = useState<Phase>({ kind: "loading" });
  // Set once the dialog is left (a button, Escape, or its owner took it away): a save under way does not go on to the fix then.
  const left = useRef(false);
  useEffect(() => {
    left.current = false;
    return () => {
      left.current = true;
    };
  }, []);
  const leave = () => {
    left.current = true;
    onClose();
  };

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
      else leave();
    } catch (e) {
      setPhase(failure(e, plan));
    }
  };
  // The dialog is modal (a click on the title bar's „Speichern“ would only close it): it saves the project itself, then applies the fix. A
  // save that failed says why in the app's message, and the fix meets the unsaved archive again. A dialog left while it saved (Abbrechen,
  // Escape) creates nothing: that could not be taken back.
  const saveAndApply = async (plan: FixPlanDto) => {
    setPhase({ kind: "applying", plan });
    await checks.save();
    if (left.current) return;
    await apply(plan);
  };

  const close: DialogAction = { label: t("fixDialog.close"), onClick: leave };
  const actions: DialogAction[] =
    phase.kind === "ready"
      ? [{ label: t("fixDialog.cancel"), onClick: leave }, { label: fix.label, kind: "primary", onClick: () => void apply(phase.plan) }]
      : phase.kind === "stale"
        ? [close, { label: t("fixDialog.recheck"), kind: "primary", onClick: () => (void checks.recheck(), leave()) }]
        : phase.kind === "unsaved"
          ? [close, { label: t("fixDialog.saveAndCreate"), kind: "primary", onClick: () => void saveAndApply(phase.plan) }]
          : phase.kind === "loading" || phase.kind === "applying"
            ? [{ label: t("fixDialog.cancel"), onClick: leave }]
            : [close];
  const plan = "plan" in phase ? phase.plan : null;

  return (
    <Dialog open onOpenChange={(open) => !open && leave()} title={findingLabel(finding, depth)} description={findingText(finding, depth)} actions={actions}>
      <div className={styles.body} data-fix={fix.id}>
        {phase.kind === "loading" && <p role="status">{t("fixDialog.loading")}</p>}
        {plan !== null && (
          <>
            {plan.irreversible && (
              <p className={styles.irreversible}>
                <Badge kind="irreversible" /> {t("fixDialog.irreversible")}
              </p>
            )}
            {plan.changes.map((change) => (
              <BeforeAfter key={`${change.asset.kind}:${change.asset.index}`} label={change.label} before={change.before} after={change.after} />
            ))}
            {plan.files && plan.files.length > 0 && (
              <ul className={styles.files}>
                {plan.files.map((file) => (
                  <li key={file.path} data-fix-file>{`${file.path} → ${backupPath(file.to)}`}</li>
                ))}
              </ul>
            )}
          </>
        )}
        {phase.kind === "applying" && <p role="status">{t("fixDialog.applying")}</p>}
        {phase.kind === "remaining" && (
          <Notice kind="warning" title={t("fixDialog.remainingTitle")}>
            {t("fixDialog.remainingText")}
          </Notice>
        )}
        {phase.kind === "stale" && (
          <Notice kind="error" title={t("fixDialog.staleTitle")}>
            {t("fixDialog.staleText")}
          </Notice>
        )}
        {phase.kind === "unsaved" && (
          <Notice kind="error" title={t("fixDialog.unsavedTitle")}>
            {t("fixDialog.unsavedText", { archive: `${termLabel("textArchive")} ${id4(phase.archive)}` })}
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
