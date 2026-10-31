import { useEffect, useRef, useState } from "react";
import { errorText, gitCommitAll, gitCommitPrepare } from "../api";
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
  /** The backend's checks ran (on opening, after a refusal) and may have refreshed the template block: the app reads the status again. */
  onRefresh?: () => void | Promise<void>;
  /** Runs it while nothing else may write (the app's session is busy meanwhile); by default it just runs. */
  exclusive?: <T>(work: () => Promise<T>) => Promise<T>;
  /** Where focus goes once the dialog has closed (`Dialog.finalFocus`), asked then; null or absent: back to the element that had it. */
  finalFocus?: () => HTMLElement | null;
};

/**
 * „Stand sichern?“: one commit of every change of the project (never a choice of files), in its own repository. What is checked before
 * (everything saved, the `git` found, something changed), the subject and an optional note, then on „Sichern“ the phases. While it runs
 * the dialog has no button and stays. A refusal shows its sentence and the dialog can try again; a commit that failed (a hook, signing)
 * shows Git's reason and leaves only „Schließen“. On opening (with everything saved and a `git` found) and after a refusal it runs the
 * backend's checks (`gitCommitPrepare`): they bring an outdated template block up to date, so the changes it counts are those the commit
 * would take (after a release in a repository of 4.4, `.g4/releases.json` and the block).
 */
export function CommitDialog({ open, status, unsaved, preset, onClose, onDone, onRefresh, exclusive = (work) => work(), finalFocus }: CommitDialogProps) {
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
  // The Git state the backend's checks answered; dropped when the owner reads a newer one.
  const [prepared, setPrepared] = useState<GitStatus | null>(null);
  const [checking, setChecking] = useState(false);
  // Each run of the checks; an answer of an older one (or after the dialog went) is dropped.
  const prepareRun = useRef(0);
  const onRefreshRef = useRef(onRefresh);
  onRefreshRef.current = onRefresh;

  /** The backend's checks and the fresh state; `keepError`: a refusal of the commit stays shown instead of the checks' own. */
  const prepare = async (keepError: boolean) => {
    const run = ++prepareRun.current;
    setChecking(true);
    try {
      const fresh = await gitCommitPrepare();
      if (run !== prepareRun.current) return;
      setPrepared(fresh);
    } catch (e) {
      if (run !== prepareRun.current) return;
      if (!keepError) setError(errorText(e));
    } finally {
      if (run === prepareRun.current) setChecking(false);
    }
    await onRefreshRef.current?.();
  };

  // Each opening starts over: the preset's subject and notes, or the subject of now.
  useEffect(() => {
    if (!open) return;
    setSubject(presetRef.current?.message ?? commitSubjectNow(new Date()));
    setNotes(presetRef.current?.notes ?? "");
    setPhase(null);
    setError(null);
    setFailed(null);
  }, [open]);

  // A newer read of the owner replaces the answer of the checks.
  useEffect(() => setPrepared(null), [status]);

  const gitFound = status !== null && status.cli !== null;
  // With everything saved and a `git` found (else a row already says what is missing), the checks run once per opening.
  useEffect(() => {
    if (!open || unsaved || !gitFound) return;
    void prepare(false);
    return () => {
      prepareRun.current += 1;
      setChecking(false);
    };
    // `prepare` reads only refs and setters.
  }, [open, unsaved, gitFound]);

  const shown = prepared ?? status;
  const git = shown === null ? undefined : shown.cli;
  const changes = shown?.changes ?? null;
  const rows: CheckRow[] = [
    unsaved ? { id: "saved", state: "error", text: "Erst speichern" } : { id: "saved", state: "ok", text: "Alles gespeichert" },
    git === undefined
      ? { id: "git", state: "info", text: "Git wird gesucht …" }
      : git === null
        ? { id: "git", state: "error", text: "Git ist nicht installiert" }
        : { id: "git", state: "ok", text: gitFoundText(git.version) },
    ...(checking
      ? [{ id: "changes", state: "info", text: "Änderungen werden geprüft …" } satisfies CheckRow]
      : changes === null
        ? []
        : [{ id: "changes", state: changes === 0 ? "warning" : "ok", text: changedFilesText(changes) } satisfies CheckRow]),
  ];
  const message = subject.trim();
  const ready = !unsaved && !checking && git !== undefined && git !== null && changes !== null && changes > 0 && message !== "";

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
      // The state may have moved (a save elsewhere, a refreshed block): the checks again, the refusal stays shown.
      void prepare(true);
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
