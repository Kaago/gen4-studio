import type { DraftDto } from "../../api";
import { Badge, Notice } from "../../ui/primitives";
import parts from "./WizardStep.module.css";

/** The badge „Nicht rückgängig“ with what the draft says undo would not take back behind it (the badge alone without a draft). */
export function DraftIrreversible({ draft }: { draft: DraftDto | null }) {
  const lost = draft?.irreversible ?? [];
  return (
    <p className={parts.loss}>
      <Badge kind="irreversible" />
      {lost.length > 0 && <span>{lost.join(", ")}</span>}
    </p>
  );
}

/** A failed plan: the preview is missing, creating stays possible (the creation answers with its own refusal). */
export function DraftFailed({ error }: { error: string | null }) {
  return error === null ? null : <Notice kind="warning">Vorschau nicht möglich: {error}</Notice>;
}

/** The draft in the step „Zusammenfassung“: its sentences as lines, what undo would not take back behind the badge, or the plan's failure. */
export function DraftNotes({ draft, error }: { draft: DraftDto | null; error: string | null }) {
  if (error !== null) return <DraftFailed error={error} />;
  if (draft === null) return null;
  return (
    <div className={parts.lines} data-draft-notes="">
      {draft.notes.map((note) => (
        <p key={note} className={parts.note}>
          {note}
        </p>
      ))}
      {draft.irreversible.length > 0 && <DraftIrreversible draft={draft} />}
    </div>
  );
}
