import { useRef, useState } from "react";
import type { JSX } from "react";
import { errorText } from "../api";
import type { FlagKind, Progress, ProjectInfo } from "../api";
import { FlagField } from "../flags/FlagField";
import { useFlagList } from "../flags/useFlagList";
import { Dialog, Notice, Segmented, TextField } from "../ui/primitives";
import { flagIdOf } from "./storyWords";
import styles from "./StoriesSurface.module.css";

export type NewStoryRequest = { name: string; progress: Progress; notes?: string };

export type NewStoryDialogProps = {
  open: boolean;
  onClose: () => void;
  /** Creates the story (`story_create`, seconds under load); a refusal rejects with the kernel's sentence and the dialog stays. */
  onCreate: (req: NewStoryRequest) => Promise<void>;
  project: ProjectInfo | null;
  revision: number;
};

const KINDS = [
  { value: "var", label: "Variable" },
  { value: "flag", label: "Flag" },
] as const;

/**
 * „Neue Geschichte“: the name (required), the progress as a variable or a flag (the field of 3d.1 with its picker and „Freie Variable
 * nehmen“; 0 is nothing chosen) and notes. „Geschichte anlegen“ is locked without a name or a progress and while the create runs (no
 * second one); a refusal of the backend stays in the dialog as an error, the dialog open. The id sent is the number's own name, else its
 * pret name, else its fallback id: each resolves to the number in the kernel.
 */
export function NewStoryDialog({ open, onClose, onCreate, project, revision }: NewStoryDialogProps): JSX.Element {
  const [name, setName] = useState("");
  const [kind, setKind] = useState<FlagKind>("var");
  const [number, setNumber] = useState(0);
  const [notes, setNotes] = useState("");
  const [refusal, setRefusal] = useState<string | null>(null);
  const [running, setRunning] = useState(false);
  const busy = useRef(false);
  // The names of the chosen kind (the field reads the same list): the id the story keeps.
  const { list } = useFlagList(kind, { project: open && number !== 0 ? project : null, revision });

  // Every opening starts empty.
  const [wasOpen, setWasOpen] = useState(open);
  if (open !== wasOpen) {
    setWasOpen(open);
    if (open) {
      setName("");
      setKind("var");
      setNumber(0);
      setNotes("");
      setRefusal(null);
    }
  }

  const ready = name.trim() !== "" && number !== 0;
  const submit = async () => {
    if (busy.current || !ready) return;
    busy.current = true;
    setRunning(true);
    setRefusal(null);
    const id = flagIdOf(kind, number, list?.entries.find((e) => e.number === number));
    const text = notes.trim();
    try {
      await onCreate({ name: name.trim(), progress: kind === "var" ? { var: id } : { flag: id }, ...(text === "" ? {} : { notes: text }) });
      onClose();
    } catch (e) {
      setRefusal(errorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next && !busy.current) onClose();
      }}
      dismissible={!running}
      title="Neue Geschichte"
      description="Eine Geschichte hält ihren Fortschritt in einer Variable oder einem Flag; ihre Schritte legen die Rezepte an."
      data={{ "data-new-story-dialog": "" }}
      actions={[
        { label: "Abbrechen", onClick: onClose, disabled: running },
        { label: "Geschichte anlegen", kind: "primary", onClick: () => void submit(), disabled: running || !ready },
      ]}
    >
      <div className={styles.dialog}>
        <TextField label="Name" showLabel value={name} onCommit={setName} onChange={setName} disabled={running} />
        <div className={styles.progress} role="group" aria-label="Fortschritt">
          <span className={styles.label}>Fortschritt</span>
          <div data-progress-kind={kind}>
            <Segmented
              ariaLabel="Art des Fortschritts"
              size="s"
              options={KINDS.map((k) => ({ ...k, disabled: running }))}
              value={kind}
              onChange={(next) => {
                setKind(next);
                setNumber(0);
              }}
            />
          </div>
          <FlagField bare kind={kind} label="Fortschritt" value={number} onCommit={setNumber} disabled={running} depth="bauen" project={project} revision={revision} field="progress" />
        </div>
        <TextField label="Notizen" showLabel multiline value={notes} onCommit={setNotes} onChange={setNotes} disabled={running} />
        {running && (
          <p className={styles.note} role="status">
            Wird angelegt …
          </p>
        )}
        {refusal !== null && <Notice kind="error">{refusal}</Notice>}
      </div>
    </Dialog>
  );
}
