import { useRef, useState } from "react";
import { errorText, migrateTextProject } from "../api";
import type { MigrateReport } from "../api";
import { Dialog, Notice } from "../ui/primitives";

/** The schema version of the text project this build of Gen4 Studio writes (`textproject::SCHEMA_VERSION`). */
export const TEXT_PROJECT_SCHEMA = 1;

/** The message after lifting: from which version to which. */
export function liftedText(report: MigrateReport): string {
  return `Textprojekt angehoben: Schema-Version ${report.from} → ${report.to}.`;
}

export type MigrateDialogProps = {
  /** The project's schema version (older than `TEXT_PROJECT_SCHEMA`). */
  schema: number;
  /** „Abbrechen“ and Escape: the project closes (unlifted it can only be read). */
  onCancel: () => void;
  /** It was lifted: the backend opened the session anew when something changed (the app reads it again). */
  onLifted: (report: MigrateReport) => void | Promise<void>;
  /** Runs it while nothing else may write (the app's session is busy meanwhile); by default it just runs. */
  exclusive?: <T>(work: () => Promise<T>) => Promise<T>;
};

/**
 * „Textprojekt anheben?“, asked when a text project of an older schema opens: lifting saves the changed files first; „Abbrechen“ closes the
 * project. A failure stays in the dialog with its sentence.
 */
export function MigrateDialog({ schema, onCancel, onLifted, exclusive = (work) => work() }: MigrateDialogProps) {
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const busy = useRef(false);

  const lift = async () => {
    if (busy.current) return;
    busy.current = true;
    setRunning(true);
    setError(null);
    try {
      await onLifted(await exclusive(() => migrateTextProject()));
    } catch (e) {
      setError(errorText(e));
    } finally {
      busy.current = false;
      setRunning(false);
    }
  };

  return (
    <Dialog
      open
      onOpenChange={(open) => {
        if (!open && !busy.current) onCancel();
      }}
      dismissible={!running}
      title="Textprojekt anheben?"
      description={`Dieses Textprojekt hat Schema-Version ${schema}; diese Fassung von Gen4 Studio schreibt Version ${TEXT_PROJECT_SCHEMA}. Anheben sichert die geänderten Dateien vorher in .g4/backup/.`}
      data={{ "data-migrate-dialog": "" }}
      actions={[
        { label: "Abbrechen", onClick: onCancel, disabled: running },
        { label: "Anheben", kind: "primary", onClick: () => void lift(), disabled: running },
      ]}
    >
      {error !== null && <Notice kind="error">{error}</Notice>}
    </Dialog>
  );
}
