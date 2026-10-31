import type { DraftDto } from "../../api";
import { depthAtLeast } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
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

/**
 * The draft in the step „Zusammenfassung“. The kernel writes its sentences in Tüfteln's words (header, event file, numbers), so
 * only Tüfteln shows them: the notes as lines and what undo would not take back (the frame's foot carries the badge). Bauen and
 * Erzählen have the wizard's own lines and the foot's badge; there only a failed plan shows.
 */
export function DraftNotes({ draft, error, depth }: { draft: DraftDto | null; error: string | null; depth: Depth }) {
  if (error !== null) return <DraftFailed error={error} />;
  if (draft === null || !depthAtLeast(depth, "tuefteln")) return null;
  return (
    <div className={parts.lines} data-draft-notes="">
      {draft.notes.map((note) => (
        <p key={note} className={parts.note}>
          {note}
        </p>
      ))}
      {draft.irreversible.length > 0 && <p className={parts.note}>Nicht rückgängig: {draft.irreversible.join(", ")}</p>}
    </div>
  );
}
