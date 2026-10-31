import { useRef, useState } from "react";
import { errorText, formatBytes, isCmdError } from "../api";
import type { PublishOptionsDto, PublishPhase, PublishReportDto, ReleasesDto } from "../api";
import { openPath } from "../platform/dialogs";
import { Button, Checkbox, Notice, Table, TextField } from "../ui/primitives";
import type { TableColumn } from "../ui/primitives";
import { WizardFrame } from "../workbench/wizards/WizardFrame";
import type { WizardStep } from "../workbench/wizards/WizardFrame";
import { DoneResult } from "../workbench/wizards/WizardResult";
import parts from "../workbench/wizards/WizardStep.module.css";
import { NO_ROM_CONTENT, PHASE_TEXT, fileNameFor, fileProblem, filesLine, publishedSubject, versionProblem, withXdelta } from "./publishWords";
import styles from "./PublishWizard.module.css";

export type PublishWizardProps = {
  /** The releases, the stem and the proposal (null while read; an error shows and locks). */
  defaults: ReleasesDto | null;
  defaultsError: string | null;
  /** The project's own repository is there: „Stand sichern…“ is offered on „Fertig“. */
  gitOwn: boolean;
  onPublish: (opts: PublishOptionsDto, onProgress: (p: { phase: PublishPhase }) => void) => Promise<PublishReportDto>;
  onReveal: (path: string) => Promise<void>;
  /** „Stand sichern…“ on „Fertig“: closes the wizard and opens the dialog with this preset. */
  onCommit: (preset: { message: string; notes: string }) => void;
  /** Left; `published` says the releases list changed. */
  onClose: (published: boolean) => void;
};

/** A refusal of the run: the backend's code (null for an error without one) and its sentence. */
type Refusal = { code: string | null; message: string };

/** The steps by their index in the frame (the result, „Fertig“, is the frame's own). */
const VERSION = 0;
const PATCH = 1;
const CHECK = 2;

const FILE_COLUMNS: readonly TableColumn[] = [
  { key: "file", label: "Datei" },
  { key: "size", label: "Größe", align: "end" },
];

/** The last part of a path, for the preview's file column. */
const baseName = (path: string) => path.slice(Math.max(path.lastIndexOf("/"), path.lastIndexOf("\\")) + 1);

/**
 * „Veröffentlichen“: version, change notes and place (Version), the run that saves, builds and writes the self-checked patch (Patch), the
 * self-check and what is handed on (Prüfen), then the result with „Im Finder zeigen“ and „Stand sichern…“ (Fertig). The run starts with
 * „Weiter“ on „Patch“; once it wrote the release, the steps before it are closed (another run would be another release).
 */
export function PublishWizard({ defaults, defaultsError, gitOwn, onPublish, onReveal, onCommit, onClose }: PublishWizardProps) {
  const [step, setStep] = useState(VERSION);
  // What the user typed or chose; null: the proposal (the next version, the file name that follows it, the build folder).
  const [ownVersion, setOwnVersion] = useState<string | null>(null);
  const [ownFile, setOwnFile] = useState<string | null>(null);
  const [ownDir, setOwnDir] = useState<string | null>(null);
  const [notes, setNotes] = useState("");
  const [readme, setReadme] = useState(false);
  // The version the backend refused (taken meanwhile, or a rule the check here does not know), with its sentence; typing another clears it.
  const [versionRefusal, setVersionRefusal] = useState<{ version: string; message: string } | null>(null);
  const [running, setRunning] = useState(false);
  const [phase, setPhase] = useState<PublishPhase | null>(null);
  const [refusal, setRefusal] = useState<Refusal | null>(null);
  const [report, setReport] = useState<PublishReportDto | null>(null);
  const [done, setDone] = useState(false);
  // Progress that arrives after the run ended must not bring the line back.
  const live = useRef(false);

  const version = ownVersion ?? defaults?.next_version ?? "";
  const file = ownFile ?? (defaults ? fileNameFor(defaults.stem, version) : "");
  const dir = ownDir ?? defaults?.dir ?? "";
  const taken = defaults?.releases.map((r) => r.version) ?? [];
  const versionError = versionRefusal !== null && versionRefusal.version === version ? versionRefusal.message : versionProblem(version, taken);
  const fileError = fileProblem(file);
  const ready = defaults !== null && versionError === null && fileError === null;
  const options: PublishOptionsDto = { version, notes, dir, file, readme };

  async function run(overwrite: boolean) {
    live.current = true;
    setRunning(true);
    setPhase(null);
    setRefusal(null);
    try {
      const published = await onPublish(overwrite ? { ...options, overwrite: true } : options, (p) => live.current && setPhase(p.phase));
      setReport(published);
      setStep(CHECK);
    } catch (e) {
      const code = isCmdError(e) ? e.code : null;
      if (code === "version_taken" || code === "bad_version") {
        // The version goes back to its field with the sentence; nothing was built.
        setVersionRefusal({ version, message: errorText(e) });
        setStep(VERSION);
      } else {
        setRefusal({ code, message: errorText(e) });
      }
    } finally {
      live.current = false;
      setRunning(false);
      setPhase(null);
    }
  }

  async function chooseDir() {
    const chosen = await openPath({ directory: true, title: "Speicherort wählen" });
    if (chosen !== null) setOwnDir(chosen);
  }

  let versionContent;
  if (defaults === null) {
    versionContent = (
      <section className={parts.column} data-publish-step="version">
        {defaultsError !== null ? <Notice kind="error">{defaultsError}</Notice> : <p className={parts.note}>Lese die Veröffentlichungen …</p>}
      </section>
    );
  } else {
    versionContent = (
      <>
        <section className={parts.column} data-publish-step="version">
          <div className={styles.field}>
            <TextField
              label="Version"
              showLabel
              value={version}
              onCommit={(typed) => {
                setVersionRefusal(null);
                setOwnVersion(typed.trim());
              }}
              error={versionError ?? undefined}
            />
          </div>
          <div className={styles.notes}>
            <TextField label="Änderungsnotizen" showLabel multiline value={notes} onCommit={setNotes} hint="Was sich seit der letzten Fassung geändert hat. Steht in der Begleitdatei." />
          </div>
        </section>
        <section className={parts.column} data-publish-step="version">
          <div className={parts.group}>
            <span className={parts.label}>Speicherort</span>
            <p className={styles.path} data-publish-dir="">
              {dir}
            </p>
          </div>
          <Button kind="secondary" icon="open" onClick={() => void chooseDir()}>
            Ordner wählen…
          </Button>
          <div className={styles.field}>
            <TextField label="Dateiname" showLabel value={file} onCommit={(typed) => setOwnFile(typed.trim())} error={fileError ?? undefined} />
          </div>
        </section>
      </>
    );
  }

  const patchContent = (
    <section className={parts.column} data-publish-step="patch">
      <p className={parts.text}>Speichert, baut die ROM und schreibt den Patch gegen die Basis-ROM. Die gebaute ROM bleibt unter build/ im Projekt.</p>
      <Checkbox label="Änderungsnotizen auch als README.txt daneben" checked={readme} onChange={setReadme} />
      {running && (
        <p role="status" className={styles.progress} data-publish-progress="">
          {PHASE_TEXT[phase ?? "build"]}
        </p>
      )}
      {!running && refusal !== null && (
        <>
          <Notice kind="error">{refusal.message}</Notice>
          {refusal.code === "exists" && (
            <Button kind="secondary" data-publish-overwrite="" onClick={() => void run(true)}>
              Ersetzen
            </Button>
          )}
        </>
      )}
    </section>
  );

  let checkContent = null;
  if (report !== null) {
    const { patch } = report;
    checkContent = (
      <>
        <section className={parts.column} data-publish-step="check">
          <Notice kind="success" title="Selbstprüfung bestanden: auf die Basis-ROM angewendet ergibt der Patch genau die gebaute ROM.">
            {`Basis ${patch.base_sha1.slice(0, 8)} → Ziel ${patch.target_sha1.slice(0, 8)}, geprüft in ${patch.verify_ms} ms`}
          </Notice>
        </section>
        <section className={`${parts.column} ${parts.wide}`} data-publish-step="check" data-publish-preview="">
          <h3 className={parts.heading}>Was du weitergibst</h3>
          <div className={styles.table}>
            <Table columns={FILE_COLUMNS} ariaLabel="Was du weitergibst" rows={report.files.map((f) => ({ key: f.path, cells: { file: baseName(f.path), size: formatBytes(f.size) } }))} />
          </div>
          <p className={parts.note}>{NO_ROM_CONTENT}</p>
        </section>
      </>
    );
  }

  const steps: WizardStep[] = [
    { id: "version", title: "Version", canNext: ready, content: versionContent },
    { id: "patch", title: "Patch", canNext: ready && !running, content: patchContent },
    { id: "check", title: "Prüfen", canNext: report !== null, content: checkContent },
  ];

  let result = null;
  if (done && report !== null) {
    const shownVersion = report.release.version;
    const next = ["Den Patch weitergeben: wer dieselbe Basis-ROM hat, wendet ihn mit Rom Patcher JS oder xdelta3 an."];
    if (gitOwn) next.push(`Stand sichern legt einen Commit mit „${publishedSubject(shownVersion)}“ an.`);
    result = (
      <div className={styles.result}>
        <DoneResult title={`Veröffentlicht: v${shownVersion}`} lines={[report.patch.patch, filesLine(report.files)]} next={next} />
        {gitOwn && (
          <Button kind="secondary" icon="git" data-publish-commit="" onClick={() => onCommit({ message: publishedSubject(shownVersion), notes: report.release.notes })}>
            Stand sichern…
          </Button>
        )}
      </div>
    );
  }

  return (
    <WizardFrame
      title="Veröffentlichen"
      icon="publish"
      steps={steps}
      step={step}
      onStep={(i) => {
        // Once the release is written, the steps before „Prüfen“ stay closed: going on from „Patch“ would publish again.
        if (report !== null) return;
        if (step === PATCH && i === CHECK) {
          void run(false);
          return;
        }
        setRefusal(null);
        setStep(i);
      }}
      busy={running}
      // What it is going to write, until it wrote it.
      summary={defaults !== null && report === null ? `Schreibt ${withXdelta(file)} und die Begleitdatei nach ${dir}.` : undefined}
      result={result}
      resultTitle="Fertig"
      submitLabel="Fertig"
      onSubmit={() => setDone(true)}
      onCancel={() => onClose(report !== null)}
      onShow={report !== null ? () => void onReveal(report.patch.patch) : undefined}
      showLabel="Im Finder zeigen"
    />
  );
}
