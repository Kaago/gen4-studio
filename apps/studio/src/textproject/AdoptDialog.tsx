import { useRef, useState } from "react";
import { adoptTextProject, errorText, isCmdError } from "../api";
import type { AdoptProgress, AdoptReport } from "../api";
import { CheckList, Dialog, Notice } from "../ui/primitives";
import type { CheckRow, DialogAction } from "../ui/primitives";
import { NBSP } from "../technik/syncWords";
import styles from "./AdoptDialog.module.css";

/**
 * The backend's answer when the files were set up but the open session changed meanwhile and was kept (`textsource::REOPEN`): the project
 * must be opened anew, setting it up again would be refused.
 */
export const REOPEN = "Die Dateien sind umgestellt, aber die offene Sitzung wurde inzwischen geändert und bleibt, wie sie ist; das Projekt bitte neu öffnen.";

/** The unsaved count as the kernel says it („Erst speichern: 1 ungespeicherte Änderung.“). */
export function unsavedText(n: number): string {
  return `Erst speichern: ${n} ungespeicherte ${n === 1 ? "Änderung" : "Änderungen"}.`;
}

/** The progress line of the set-up. */
export function adoptProgressText(p: AdoptProgress): string {
  const counted = p.total > 0 ? ` ${p.done} von ${p.total}` : "";
  switch (p.phase) {
    case "scripts":
      return `Scripts übernehmen …${counted}`;
    case "base_layer":
      return "Basisschicht anlegen …";
    case "entities":
      return `Einträge schreiben …${counted}${p.total > 0 ? " Arten" : ""}`;
    case "sync":
      return "Abgleich vorbereiten …";
  }
}

/** The message after the set-up: the files of the text project and the seconds it took. */
export function adoptedText(report: AdoptReport): string {
  return `Textprojekt eingerichtet: ${report.export.files.toLocaleString("de-DE")}${NBSP}Dateien in ${Math.round(report.ms / 1000)}${NBSP}s.`;
}

export type AdoptDialogProps = {
  /** Unsaved edits of the store and the open scripts: the set-up needs everything saved. */
  unsaved: number;
  /** The base ROM's name when it is found; null when it is not (`baseReason` says why) or not known yet. */
  baseLabel: string | null;
  /** Why the base ROM is missing; null while it is looked for (then the set-up waits too). */
  baseReason: string | null;
  /** The scripts are adopted already; otherwise the set-up adopts them. */
  scriptsAdopted: boolean;
  /** „Abbrechen“, Escape, and after the set-up. */
  onClose: () => void;
  /** The set-up went in: the backend opened the session anew (the app reads it again and says so). */
  onAdopted: (report: AdoptReport) => void | Promise<void>;
  /** Runs the set-up while nothing else may write (the app's session is busy meanwhile); by default it just runs. */
  exclusive?: <T>(work: () => Promise<T>) => Promise<T>;
};

/**
 * „Als Textprojekt einrichten?“: what is checked before (everything saved, the base ROM found, the scripts), then on „Einrichten“ the
 * set-up with its progress. While it runs the dialog has no button and stays (it takes 10 to 120 s and the backend refuses writes
 * meanwhile); a refusal shows its sentence and the dialog can try again.
 */
export function AdoptDialog({ unsaved, baseLabel, baseReason, scriptsAdopted, onClose, onAdopted, exclusive = (work) => work() }: AdoptDialogProps) {
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState<AdoptProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [reopen, setReopen] = useState(false);
  // Read by a click that comes before the render of the one before (a double click).
  const busy = useRef(false);

  const rows: CheckRow[] = [
    unsaved === 0 ? { id: "saved", state: "ok", text: "Alles gespeichert" } : { id: "saved", state: "error", text: unsavedText(unsaved) },
    baseLabel !== null
      ? { id: "base", state: "ok", text: `Basis-ROM gefunden: ${baseLabel}` }
      : baseReason !== null
        ? { id: "base", state: "error", text: baseReason }
        : { id: "base", state: "info", text: "Basis-ROM wird gesucht …" },
    scriptsAdopted ? { id: "scripts", state: "ok", text: "Scripts übernommen" } : { id: "scripts", state: "info", text: "Scripts werden dabei übernommen" },
  ];
  const ready = unsaved === 0 && baseLabel !== null;

  const adopt = async () => {
    if (!ready || busy.current) return;
    busy.current = true;
    setRunning(true);
    setProgress(null);
    setError(null);
    try {
      const report = await exclusive(() => adoptTextProject(setProgress));
      await onAdopted(report);
      onClose();
    } catch (e) {
      setError(errorText(e));
      setReopen(isCmdError(e) && e.message === REOPEN);
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  const actions: DialogAction[] = running
    ? []
    : reopen
      ? [{ label: "Schließen", onClick: onClose }]
      : [
          { label: "Abbrechen", onClick: onClose },
          { label: "Einrichten", kind: "primary", onClick: () => void adopt(), disabled: !ready },
        ];

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy.current) onClose();
      }}
      dismissible={!running}
      title="Als Textprojekt einrichten?"
      description="Schreibt g4-project.json und rund 6 500 Dateien unter data/. Die Spieldateien bleiben, wie sie sind; zurücknehmen geht in Technik › Textprojekt."
      data={{ "data-adopt-dialog": "" }}
      actions={actions}
    >
      <div className={styles.content}>
        <CheckList ariaLabel="Vor dem Einrichten" rows={rows} />
        {running && (
          <p role="status" className={styles.progress} data-adopt-progress="">
            {progress === null ? "Wird eingerichtet …" : adoptProgressText(progress)}
          </p>
        )}
        {error !== null && <Notice kind="error">{error}</Notice>}
      </div>
    </Dialog>
  );
}
