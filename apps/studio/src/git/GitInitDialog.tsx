import { useEffect, useRef, useState } from "react";
import { errorText, gitInit } from "../api";
import type { GitStatus, InitPhase, InitReport } from "../api";
import { toast } from "../ui/frame/toasts";
import { CheckList, Dialog, Notice } from "../ui/primitives";
import type { CheckRow, DialogAction } from "../ui/primitives";
import { NOT_INSTALLED, TEMPLATES_ONLY_TEXT, gitFoundText, initDoneText } from "./gitWords";
import styles from "./GitInitDialog.module.css";

/** Always in the dialog: the project holds the game's data, so it does not belong in a public repository. */
export const LEGAL_NOTE = "Dein Projekt enthält Daten aus dem Spiel. Lade es nicht öffentlich hoch; zum Teilen gibt es den Patch (Technik › Veröffentlichen).";

const DESCRIPTION =
  "Legt in diesem Projektordner ein lokales Git-Repository an: .gitignore und .gitattributes, dann einen ersten Commit „Ausgangsstand“. Gen4 Studio richtet kein Remote ein und lädt nichts hoch.";

/** What the `.gitignore` of each layout keeps out. */
const IGNORED: Record<GitStatus["layout"], string> = {
  dspre: "Ignoriert: ROMs, Patches, build/, Spielstände und die Archive, die der Build packt",
  text_project: "Ignoriert: ROMs, Patches, build/, Spielstände und die Spieldateien (sie entstehen aus der Basis-ROM)",
};

/** The progress line of each phase. */
const PHASE_TEXT: Record<InitPhase, string> = {
  unpack: "Archive auspacken …",
  templates: "Vorlagen schreiben …",
  init: "Repository anlegen …",
  add: "Dateien aufnehmen … (bei DSPRE-Ordnern rund 10 s)",
  commit: "Erster Commit …",
};

export type GitInitDialogProps = {
  open: boolean;
  /** The Git state as last read (layout, the `git` found, the templates there); null while it is read. */
  status: GitStatus | null;
  /** Unsaved edits of the store or the open scripts: the first commit takes the saved state, so they lock „Einrichten“. */
  unsaved: boolean;
  /** „Abbrechen“, Escape, „Schließen“, and after it went through. */
  onClose: () => void;
  /** It ran (the repository, or at least the templates, are there): the app reads the status again. Also when the commit failed. */
  onDone: (report: InitReport) => void | Promise<void>;
  /** Runs it while nothing else may write (the app's session is busy meanwhile); by default it just runs. */
  exclusive?: <T>(work: () => Promise<T>) => Promise<T>;
};

/**
 * „Git einrichten?“: what the set-up does and keeps out, the legal note, what is checked before (everything saved, the `git` found, the
 * templates already there), then on „Einrichten“ the phases. While it runs the dialog has no button and stays. A refusal shows its
 * sentence and the dialog can try again; a commit that failed (signing, a hook) shows Git's reason and leaves only „Schließen“, since the
 * repository and the templates are there.
 */
export function GitInitDialog({ open, status, unsaved, onClose, onDone, exclusive = (work) => work() }: GitInitDialogProps) {
  const [running, setRunning] = useState(false);
  const [phase, setPhase] = useState<InitPhase | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [failed, setFailed] = useState<string | null>(null);
  // Read by a click that comes before the render of the one before (a double click).
  const busy = useRef(false);

  // Each opening starts over.
  useEffect(() => {
    if (!open) return;
    setPhase(null);
    setError(null);
    setFailed(null);
  }, [open]);

  const layout = status?.layout ?? "dspre";
  const git = status === null ? undefined : status.cli;
  const rows: CheckRow[] = [
    unsaved ? { id: "saved", state: "error", text: "Erst speichern" } : { id: "saved", state: "ok", text: "Alles gespeichert" },
    git === undefined
      ? { id: "git", state: "info", text: "Git wird gesucht …" }
      : git === null
        ? { id: "git", state: "warning", text: "Git ist nicht installiert: nur die Vorlagen werden geschrieben" }
        : { id: "git", state: "ok", text: gitFoundText(git.version) },
    { id: "ignored", state: "info", text: IGNORED[layout] },
    ...(status?.templates ?? []).map((file): CheckRow => ({ id: `template-${file}`, state: "info", text: `${file} vorhanden: der Block von Gen4 Studio kommt dazu` })),
  ];
  const ready = !unsaved && git !== undefined;

  const run = async () => {
    if (!ready || busy.current) return;
    busy.current = true;
    setRunning(true);
    setPhase(null);
    setError(null);
    try {
      const report = await exclusive(() => gitInit({}, (p) => setPhase(p.phase)));
      await onDone(report);
      if (report.committed) {
        toast.info(initDoneText(report));
        onClose();
      } else if (report.reason === NOT_INSTALLED) {
        toast.info(TEMPLATES_ONLY_TEXT);
        onClose();
      } else {
        setFailed(report.reason ?? "Der erste Commit ist gescheitert.");
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
          { label: git === null ? "Vorlagen schreiben" : "Einrichten", kind: "primary", onClick: () => void run(), disabled: !ready },
        ];
  // Before the first phase reports (always in the dev bridge): the first phase of the layout.
  const shownPhase = phase ?? (layout === "dspre" ? "unpack" : "templates");

  return (
    <Dialog
      open={open}
      onOpenChange={(next) => {
        if (!next && !busy.current) onClose();
      }}
      dismissible={!running}
      title="Git einrichten?"
      description={DESCRIPTION}
      data={{ "data-git-init-dialog": "" }}
      actions={actions}
    >
      <div className={styles.content}>
        <Notice kind="warning">{LEGAL_NOTE}</Notice>
        {failed === null && <CheckList ariaLabel="Vor dem Einrichten" rows={rows} />}
        {running && (
          <p role="status" className={styles.progress} data-git-init-progress="">
            {PHASE_TEXT[shownPhase]}
          </p>
        )}
        {failed !== null && <Notice kind="warning">{failed}</Notice>}
        {error !== null && <Notice kind="error">{error}</Notice>}
      </div>
    </Dialog>
  );
}
