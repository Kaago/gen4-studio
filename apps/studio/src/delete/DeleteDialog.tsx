import { useId, useRef, useState } from "react";
import { deleteEntity, deleteTargetLabel, isCmdError, resetErrorText } from "../api";
import type { DeleteResult, DeleteTarget, RefDto, StoryMention } from "../api";
import { Badge, Checkbox, Dialog, ListRow, Notice, Reference, useRovingList } from "../ui/primitives";
import type { DialogAction } from "../ui/primitives";
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

/** What the dialog says about an original of the base ROM. */
export const ORIGINAL_WARNING = "Original aus der Basis-ROM: der Eintrag bleibt im ROM; „Auf Original zurücksetzen“ holt ihn zurück.";
/** What a tombstone keeps; not said while „Nummer freigeben“ is ticked (gate 3d.4 B11). */
export const TOMBSTONE_KEEPS = "Inhalt und Member bleiben; ⌘Z holt den Eintrag zurück.";
const FREE_LABEL = "Nummer freigeben (nicht rückgängig)";
/** The kernel's refusal in the DSPRE layout (`delete::NOT_A_TEXT_PROJECT`). */
export const NOT_A_TEXT_PROJECT = "Echtes Löschen gibt es im Textprojekt (Technik › Textprojekt).";
/** What the dialog says beside it: emptying stays an ordinary edit. */
export const CLEAR_HINT = "Leeren geht als gewöhnliche Bearbeitung: Eventdatei leeren im Inspector, Begegnungen auf 0.";

/**
 * What the dialog says under the stories that mention the entity: they are documentation and never lock; once the
 * delete is saved, the text project check reports each use that names nothing any more (`story.dangling-ref`).
 */
export const MENTIONS_HINT = "Geschichten sperren nicht; nach dem Speichern meldet die Prüfung des Textprojekts jeden Verweis, der nichts mehr nennt, als offen.";

/** „{n} Verweise: zuerst umhängen.“ (one: „Verweis“). */
export function referencesText(n: number): string {
  return `${n} ${n === 1 ? "Verweis" : "Verweise"}: zuerst umhängen.`;
}

/** „Entfernt {n} Dateien: a, b“ (one: „Datei“). */
function removesText(files: readonly string[]): string {
  return `Entfernt ${files.length} ${files.length === 1 ? "Datei" : "Dateien"}: ${files.join(", ")}`;
}

/**
 * „{label} löschen“: asks the kernel for the plan and shows what deleting does. A refused plan (an original header, a script file without
 * a number to free) or a failed one shows only the reason; references lock „Löschen“ and list where they come from (a click leads there);
 * an original warns that the entry stays in the ROM. Where the plan allows it, „Nummer freigeben (nicht rückgängig)“ frees the number
 * for good, else the plan's reason why not is said quietly. A failed delete keeps the dialog open with the reason (references it names
 * lock it as above). In the DSPRE layout the refusal comes with the way to empty an entity instead ({@link CLEAR_HINT}). The stories
 * that mention the entity stand under „Erwähnt in Geschichten“ as chips (a click opens the story), locked or not: they never lock.
 */
export function DeleteDialog({ target, onClose, onDeleted, goTo }: DeleteDialogProps) {
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
    ? [{ label: "Schließen", onClick: onClose }]
    : [
        { label: "Abbrechen", onClick: onClose, disabled: running },
        {
          label: free ? "Löschen und Nummer freigeben" : "Löschen",
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
      title={`${label} löschen`}
      data={{ "data-delete-dialog": "" }}
      actions={actions}
    >
      <div className={styles.content}>
        {planError !== null ? (
          <Notice kind="error">{planError}</Notice>
        ) : plan === null ? (
          <p role="status" className={styles.wait}>
            Prüfe Verweise …
          </p>
        ) : refused ? (
          <>
            <Notice kind="error">{plan.refusal ?? `${label} ist schon gelöscht.`}</Notice>
            {plan.refusal === NOT_A_TEXT_PROJECT && <Notice kind="info">{CLEAR_HINT}</Notice>}
          </>
        ) : locked ? (
          <>
            <Notice kind="error">{referencesText(references.length)}</Notice>
            <RefList refs={references} soft={false} onFollow={goTo === undefined ? undefined : follow} />
            {plan.mentions.length > 0 && <Mentions mentions={plan.mentions} onFollow={goTo === undefined ? undefined : follow} />}
          </>
        ) : (
          <>
            {plan.original && <Notice kind="warning">{ORIGINAL_WARNING}</Notice>}
            {plan.warnings.length > 0 && <RefList refs={plan.warnings} soft onFollow={goTo === undefined ? undefined : follow} />}
            {plan.mentions.length > 0 && <Mentions mentions={plan.mentions} onFollow={goTo === undefined ? undefined : follow} />}
            {plan.refusal !== null && <Notice kind="warning">{plan.refusal}</Notice>}
            {plan.refusal === NOT_A_TEXT_PROJECT && <Notice kind="info">{CLEAR_HINT}</Notice>}
            {plan.refusal === null && plan.deleted && <Notice kind="info">{`${label} ist schon gelöscht.`}</Notice>}
            {/* Freed, nothing stays and ⌘Z brings nothing back: then only what goes is said. */}
            {tombstone && !free && <Notice kind="info">{TOMBSTONE_KEEPS}</Notice>}
            {canFree !== null ? (
              <div className={styles.free} data-free-number="">
                <div className={styles.freeHead}>
                  <Checkbox label={FREE_LABEL} checked={free} onChange={setFree} disabled={running} />
                  <Badge kind="irreversible" />
                </div>
                <p className={styles.files}>{removesText(canFree.files)}</p>
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
                Lösche …
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
  const list = useRef<HTMLDivElement>(null);
  useRovingList(list);
  return (
    <div ref={list} role="listbox" aria-label={soft ? "Weiche Verweise" : "Verweise"} className={styles.refs}>
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
  const titleId = useId();
  return (
    <section className={styles.mentions} aria-labelledby={titleId} data-delete-mentions="">
      <h3 id={titleId} className={styles.mentionsTitle}>
        Erwähnt in Geschichten
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
      <p className={styles.quiet}>{MENTIONS_HINT}</p>
    </section>
  );
}
