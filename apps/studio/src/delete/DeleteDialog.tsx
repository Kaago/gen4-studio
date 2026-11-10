import { useId, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { deleteEntity, deleteTargetLabel, isCmdError, resetErrorText } from "../api";
import type { DeleteResult, DeleteTarget, RefDto, StoryMention } from "../api";
import { Badge, Checkbox, Dialog, ListRow, Notice, Reference, useRovingList } from "../ui/primitives";
import type { DialogAction } from "../ui/primitives";
import { matchesAnyLanguage } from "../i18n";
import type { NavTarget } from "../workbench/navigation";
import { useDeletePlan } from "./useDeletePlan";
import styles from "./DeleteDialog.module.css";

export type DeleteDialogProps = {
  target: DeleteTarget;
  /** „Abbrechen“, „Schließen“, Escape, a reference followed, and after the delete. */
  onClose: () => void;
  /** The delete went in: its result, whether the number was freed, and the entity's name („Trainer 0901“). */
  onDeleted: (result: DeleteResult, freed: boolean, label: string) => void;
  /** Where a reference leads; without it the references are only listed. */
  goTo?: (nav: NavTarget) => void;
};

/**
 * Whether `refusal` is the kernel's refusal in the DSPRE layout (`delete.not_a_text_project`): the dialog then adds the way to empty
 * the entity instead. The kernel words it in the language it was set to; a sentence of either catalog counts, so the hint stays when it
 * speaks another language than the app shows.
 */
function isNotATextProject(refusal: string | null): boolean {
  return refusal !== null && matchesAnyLanguage("delete:deleteDialog.notATextProject", refusal);
}

/**
 * „{label} löschen“: asks the kernel for the plan and shows what deleting does. A refused plan (an original header, a script file without
 * a number to free) or a failed one shows only the reason; references lock „Löschen“ and list where they come from (a click leads there);
 * an original warns that the entry stays in the ROM. Where the plan allows it, „Nummer freigeben (nicht rückgängig)“ frees the number
 * for good, else the plan's reason why not is said quietly. A failed delete keeps the dialog open with the reason (references it names
 * lock it as above). In the DSPRE layout the refusal comes with the way to empty an entity instead (`deleteDialog.clearHint`). The stories
 * that mention the entity stand under „Erwähnt in Geschichten“ as chips (a click opens the story), locked or not: they never lock.
 */
export function DeleteDialog({ target, onClose, onDeleted, goTo }: DeleteDialogProps) {
  const { t } = useTranslation("delete");
  const { plan, error: planError } = useDeletePlan(target);
  const [free, setFree] = useState(false);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  // References a delete was refused for (they came after the plan).
  const [lateRefs, setLateRefs] = useState<RefDto[] | null>(null);
  // Read by a click that comes before the render of the one before (a double click).
  const busy = useRef(false);

  const label = plan?.label ?? deleteTargetLabel(target);
  const references = lateRefs ?? plan?.references ?? [];
  const locked = references.length > 0;
  // A tombstone is possible: no refusal, not deleted yet. Freeing is possible where the plan says so.
  const tombstone = plan !== null && plan.refusal === null && !plan.deleted;
  const canFree = plan?.can_free ?? null;
  // Nothing to do at all (whatever points at it): only the reason and „Schließen“.
  const refused = planError !== null || (plan !== null && !tombstone && canFree === null);

  const remove = async () => {
    if (plan === null || busy.current) return;
    busy.current = true;
    setRunning(true);
    setError(null);
    try {
      const result = await deleteEntity(target, free);
      onDeleted(result, free, label);
      onClose();
    } catch (e) {
      if (isCmdError(e) && e.code === "referenced" && e.references !== undefined && e.references.length > 0) setLateRefs(e.references);
      // A locked asset says only its German reason, as a reset does.
      else setError(resetErrorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  const follow = (nav: NavTarget) => {
    goTo?.(nav);
    onClose();
  };

  const actions: DialogAction[] = refused
    ? [{ label: t("deleteDialog.close"), onClick: onClose }]
    : [
        { label: t("deleteDialog.cancel"), onClick: onClose, disabled: running },
        {
          label: free ? t("deleteDialog.deleteAndFree") : t("deleteDialog.delete"),
          kind: "primary",
          onClick: () => void remove(),
          disabled: plan === null || running || locked || (!tombstone && !free),
        },
      ];

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy.current) onClose();
      }}
      title={t("deleteDialog.title", { label })}
      data={{ "data-delete-dialog": "" }}
      actions={actions}
    >
      <div className={styles.content}>
        {planError !== null ? (
          <Notice kind="error">{planError}</Notice>
        ) : plan === null ? (
          <p role="status" className={styles.wait}>
            {t("deleteDialog.checking")}
          </p>
        ) : refused ? (
          <>
            <Notice kind="error">{plan.refusal ?? t("deleteDialog.alreadyDeleted", { label })}</Notice>
            {isNotATextProject(plan.refusal) && <Notice kind="info">{t("deleteDialog.clearHint")}</Notice>}
          </>
        ) : locked ? (
          <>
            <Notice kind="error">{t("deleteDialog.referencesLocked", { count: references.length })}</Notice>
            <RefList refs={references} soft={false} onFollow={goTo === undefined ? undefined : follow} />
            {plan.mentions.length > 0 && <Mentions mentions={plan.mentions} onFollow={goTo === undefined ? undefined : follow} />}
          </>
        ) : (
          <>
            {plan.original && <Notice kind="warning">{t("deleteDialog.originalWarning")}</Notice>}
            {plan.warnings.length > 0 && <RefList refs={plan.warnings} soft onFollow={goTo === undefined ? undefined : follow} />}
            {plan.mentions.length > 0 && <Mentions mentions={plan.mentions} onFollow={goTo === undefined ? undefined : follow} />}
            {plan.refusal !== null && <Notice kind="warning">{plan.refusal}</Notice>}
            {isNotATextProject(plan.refusal) && <Notice kind="info">{t("deleteDialog.clearHint")}</Notice>}
            {plan.refusal === null && plan.deleted && <Notice kind="info">{t("deleteDialog.alreadyDeleted", { label })}</Notice>}
            {/* Freed, nothing stays and ⌘Z brings nothing back: then only what goes is said. */}
            {tombstone && !free && <Notice kind="info">{t("deleteDialog.tombstoneKeeps")}</Notice>}
            {canFree !== null ? (
              <div className={styles.free} data-free-number="">
                <div className={styles.freeHead}>
                  <Checkbox label={t("deleteDialog.freeNumber")} checked={free} onChange={setFree} disabled={running} />
                  <Badge kind="irreversible" />
                </div>
                <p className={styles.files}>{t("deleteDialog.removes", { count: canFree.files.length, files: canFree.files.join(", ") })}</p>
              </div>
            ) : (
              plan.cannot_free !== null && (
                <p className={styles.quiet} data-cannot-free="">
                  {plan.cannot_free}
                </p>
              )
            )}
            {running && (
              <p role="status" className={styles.wait}>
                {t("deleteDialog.deleting")}
              </p>
            )}
            {error !== null && <Notice kind="error">{error}</Notice>}
          </>
        )}
      </div>
    </Dialog>
  );
}

/** The references as rows (label, source in mono), one tab stop and the arrow keys; a row leads to its source (`onFollow`, else inert). */
function RefList({ refs, soft, onFollow }: { refs: readonly RefDto[]; soft: boolean; onFollow?: (nav: NavTarget) => void }) {
  const { t } = useTranslation("delete");
  const list = useRef<HTMLDivElement>(null);
  useRovingList(list);
  return (
    <div ref={list} role="listbox" aria-label={soft ? t("deleteDialog.softReferencesLabel") : t("deleteDialog.referencesLabel")} className={styles.refs}>
      {refs.map((r, i) => (
        <ListRow
          key={i}
          label={r.label}
          tech={r.source_name}
          selected={false}
          disabled={onFollow === undefined}
          onSelect={() => onFollow?.(r.nav)}
          dataAttrs={soft ? { "data-warning-reference": i } : { "data-reference": i }}
        />
      ))}
    </div>
  );
}

/** „Story › Step“, or the story alone when its progress or flags name the entity. */
function mentionLabel(m: StoryMention): string {
  return m.step_name === null ? m.story_name : `${m.story_name} › ${m.step_name}`;
}

/** „Erwähnt in Geschichten“: one chip per story (and step), a click opens the story (`onFollow`, else inert), and that they never lock. */
function Mentions({ mentions, onFollow }: { mentions: readonly StoryMention[]; onFollow?: (nav: NavTarget) => void }) {
  const { t } = useTranslation("delete");
  const titleId = useId();
  return (
    <section className={styles.mentions} aria-labelledby={titleId} data-delete-mentions="">
      <h3 id={titleId} className={styles.mentionsTitle}>
        {t("deleteDialog.mentionsTitle")}
      </h3>
      <div className={styles.chips}>
        {mentions.map((m, i) => (
          <Reference
            key={i}
            area="geschichten"
            label={mentionLabel(m)}
            onOpen={() => onFollow?.({ kind: "story", number: m.number })}
            disabled={onFollow === undefined}
            dataAttrs={{ "data-delete-mention": m.story_id }}
          />
        ))}
      </div>
      <p className={styles.quiet}>{t("deleteDialog.mentionsHint")}</p>
    </section>
  );
}
