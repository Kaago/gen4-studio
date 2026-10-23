import { useRef, useState } from "react";
import { errorText } from "../../api";
import type { AdoptProgressDto, AdoptReportDto } from "../../api";
import { Button, Notice } from "../../ui/primitives";
import { TERMS, id4 } from "../terms";
import styles from "./AdoptBox.module.css";

export interface AdoptBoxProps {
  /** Number of script files of the project. */
  fileCount: number;
  /** Decompiles the project; calls `onProgress` as files are done. Rejects with the backend error. */
  adopt: (onProgress: (progress: AdoptProgressDto) => void) => Promise<AdoptReportDto>;
  /** The report was read: go on to the editor. */
  onClose: () => void;
}

type Phase = { kind: "idle" } | { kind: "running"; progress: AdoptProgressDto | null } | { kind: "done"; report: AdoptReportDto } | { kind: "failed"; message: string };

/** The box the Script panel shows until rotom has taken the project's scripts over (spec §7.3): an info notice with progress and report. */
export function AdoptBox({ fileCount, adopt, onClose }: AdoptBoxProps) {
  const [phase, setPhase] = useState<Phase>({ kind: "idle" });
  // Progress that arrives after the end must not bring the bar back.
  const running = useRef(false);

  async function start() {
    running.current = true;
    setPhase({ kind: "running", progress: null });
    try {
      const report = await adopt((progress) => running.current && setPhase({ kind: "running", progress }));
      running.current = false;
      setPhase({ kind: "done", report });
    } catch (e) {
      running.current = false;
      setPhase({ kind: "failed", message: errorText(e) });
    }
  }

  return (
    <div className={styles.box}>
      <Notice kind="info" title={`${TERMS.script.plural} übernehmen`}>
        {phase.kind === "done" ? (
          <Report report={phase.report} onClose={onClose} />
        ) : (
          <div className={styles.body}>
            <p className={styles.text}>
              Das Studio legt <code className={styles.code}>rotom.toml</code> und <code className={styles.code}>.rotom/</code> im Projekt an und dekompiliert alle {fileCount}{" "}
              {TERMS.scriptFile.plural} nach <code className={styles.code}>expanded/scripts/</code>. DSPRE benutzt danach dieselben Quellen und schaltet seinen alten
              Scripteditor ab.
            </p>
            {phase.kind === "running" ? (
              <Progress progress={phase.progress} total={fileCount} />
            ) : (
              <div>
                <Button kind="primary" onClick={() => void start()}>
                  Übernehmen
                </Button>
              </div>
            )}
            {phase.kind === "failed" && <Notice kind="error">{phase.message}</Notice>}
          </div>
        )}
      </Notice>
    </div>
  );
}

function Progress({ progress, total }: { progress: AdoptProgressDto | null; total: number }) {
  const max = Math.max(progress?.total ?? total, 1);
  const done = progress?.done ?? 0;
  const share = Math.min(done / max, 1) * 100;
  return (
    <div className={styles.progress}>
      <div className={styles.track} role="progressbar" aria-label="Fortschritt der Übernahme" aria-valuemin={0} aria-valuemax={max} aria-valuenow={done}>
        <div className={styles.fill} style={{ width: `${share}%` } /* dyn */} />
      </div>
      <span className={styles.count}>
        {done} / {progress?.total ?? total} {TERMS.scriptFile.plural}
      </span>
    </div>
  );
}

function Report({ report, onClose }: { report: AdoptReportDto; onClose: () => void }) {
  return (
    <div className={styles.body}>
      <p className={styles.text}>
        {report.decompiled} {TERMS.scriptFile.plural} dekompiliert, {report.skipped_empty} leere übersprungen
        {report.failed.length > 0 ? `, ${report.failed.length} fehlgeschlagen` : ""}.
      </p>
      {report.failed.length > 0 && (
        <>
          <p className={styles.text}>Diese Dateien bleiben binär:</p>
          <ul className={styles.failed}>
            {report.failed.map(([file, message]) => (
              <li key={file}>
                {TERMS.scriptFile.label} {id4(file)}: {message}
              </li>
            ))}
          </ul>
        </>
      )}
      <div>
        <Button kind="primary" onClick={onClose}>
          Weiter
        </Button>
      </div>
    </div>
  );
}
