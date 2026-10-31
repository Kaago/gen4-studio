import { useEffect, useRef, useState } from "react";
import { errorText, gitCommitAll } from "../api";
import type { CommitPhase, CommitReport, GitStatus } from "../api";
import { commitSubjectNow } from "../publish/publishWords";
import { toast } from "../ui/frame/toasts";
import { CheckList, Dialog, Notice, TextField } from "../ui/primitives";
import type { CheckRow, DialogAction } from "../ui/primitives";
import { changedFilesText, commitDoneText, gitFoundText } from "./gitWords";
import styles from "./CommitDialog.module.css";

const DESCRIPTION = "Legt einen Commit mit allen Änderungen im lokalen Repository an (git add -A, git commit). Nichts wird hochgeladen.";

/** The progress line of each phase. */
const PHASE_TEXT: Record<CommitPhase, string> = {
  templates: "Vorlagen prüfen …",
  add: "Dateien aufnehmen …",
  commit: "Commit …",
};

export type CommitDialogProps = {
  open: boolean;
  /** The Git state as last read (the `git` found, the changes since the last commit); null while it is read. */
  status: GitStatus | null;
  /** Unsaved edits of the store or the open scripts: the commit takes the saved state, so they lock „Sichern“. */
  unsaved: boolean;
  /** From the wizard: subject „Veröffentlicht v…“ and the notes; null: „Stand <Datum> <Uhrzeit>“ and an empty note. */
  preset: { message: string; notes: string } | null;
  /** „Abbrechen“, Escape, „Schließen“, and after the commit went through. */
  onClose: () => void;
  /** It ran (also when the commit failed): the app reads the status again. */
  onDone: (report: CommitReport) => void | Promise<void>;
  /** Runs it while nothing else may write (the app's session is busy meanwhile); by default it just runs. */
  exclusive?: <T>(work: () => Promise<T>) => Promise<T>;
  /** Where focus goes once the dialog has closed (`Dialog.finalFocus`), asked then; null or absent: back to the element that had it. */
  finalFocus?: () => HTMLElement | null;
};

/**
 * „Stand sichern?“: one commit of every change of the project (never a choice of files), in its own repository. What is checked before
 * (everything saved, the `git` found, something changed), the subject and an optional note, then on „Sichern“ the phases. While it runs
 * the dialog has no button and stays. A refusal shows its sentence and the dialog can try again; a commit that failed (a hook, signing)
 * shows Git's reason and leaves only „Schließen“.
 */
export function CommitDialog({ open, status, unsaved, preset, onClose, onDone, exclusive = (work) => work(), finalFocus }: CommitDialogProps) {
  const [subject, setSubject] = useState(() => preset?.message ?? commitSubjectNow(new Date()));
  const [notes, setNotes] = useState(() => preset?.notes ?? "");
  const [running, setRunning] = useState(false);
  const [phase, setPhase] = useState<CommitPhase | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  // Read by a click that comes before the render of the one before (a double click).
  const busy = useRef(false);
  // The preset of this opening, read when it opens (the owner may pass a new object with each render).
  const presetRef = useRef(preset);
  presetRef.current = preset;

  // Each opening starts over: the preset's subject and notes, or the subject of now.
  useEffect(() => {
    if (!open) return;
    setSubject(presetRef.current?.message ?? commitSubjectNow(new Date()));
    setNotes(presetRef.current?.notes ?? "");
    setPhase(null);
    setError(null);
    setFailed(null);
  }, [open]);

  const git = status === null ? undefined : status.cli;
  const changes = status?.changes ?? null;
  const rows: CheckRow[] = [
    unsaved ? { id: "saved", state: "error", text: "Erst speichern" } : { id: "saved", state: "ok", text: "Alles gespeichert" },
    git === undefined
      ? { id: "git", state: "info", text: "Git wird gesucht …" }
      : git === null
        ? { id: "git", state: "error", text: "Git ist nicht installiert" }
        : { id: "git", state: "ok", text: gitFoundText(git.version) },
    ...(changes === null ? [] : [{ id: "changes", state: changes === 0 ? "warning" : "ok", text: changedFilesText(changes) } satisfies CheckRow]),
  ];
  const message = subject.trim();
  const ready = !unsaved && git !== undefined && git !== null && changes !== null && changes > 0 && message !== "";

  const run = async () => {
    if (!ready || busy.current) return;
    busy.current = true;
    setRunning(true);
    setPhase(null);
    setError(null);
    try {
      const report = await exclusive(() => gitCommitAll({ message, notes }, (p) => setPhase(p.phase)));
      await onDone(report);
      if (report.committed) {
        toast.info(commitDoneText(report));
        onClose();
      } else {
        setFailed(report.reason ?? "Der Commit ist gescheitert.");
      }
    } catch (e) {
      setError(errorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  const actions: DialogAction[] = running
    ? []
    : failed !== null
      ? [{ label: "Schließen", onClick: onClose }]
      : [
          { label: "Abbrechen", onClick: onClose },
          { label: "Sichern", kind: "primary", onClick: () => void run(), disabled: !ready },
        ];

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next && !busy.current) onClose();
      }}
      dismissible={!running}
      title="Stand sichern?"
      description={DESCRIPTION}
      data={{ "data-commit-dialog": "" }}
      finalFocus={finalFocus}
      actions={actions}
    >
      <div className={styles.content}>
        {failed === null && (
          <>
            <CheckList ariaLabel="Vor dem Sichern" rows={rows} />
            <TextField label="Betreff" showLabel value={subject} onCommit={setSubject} disabled={running} />
            <TextField label="Notiz" showLabel multiline value={notes} onCommit={setNotes} disabled={running} hint="Optional; steht im Commit unter dem Betreff." />
          </>
        )}
        {running && (
          <p role="status" className={styles.progress} data-commit-progress="">
            {/* Before the first phase reports (always in the dev bridge): the first phase. */}
            {PHASE_TEXT[phase ?? "templates"]}
          </p>
        )}
        {failed !== null && <Notice kind="warning">{failed}</Notice>}
        {error !== null && <Notice kind="error">{error}</Notice>}
      </div>
    </Dialog>
  );
}
