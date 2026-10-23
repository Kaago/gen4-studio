import { useRef, useState } from "react";
import type { CSSProperties } from "react";
import { errorText } from "../../api";
import type { AdoptProgressDto, AdoptReportDto } from "../../api";
import { TERMS, id4 } from "../terms";

export interface AdoptBoxProps {
  /** Number of script files of the project. */
  fileCount: number;
  /** Decompiles the project; calls `onProgress` as files are done. Rejects with the backend error. */
  adopt: (onProgress: (progress: AdoptProgressDto) => void) => Promise<AdoptReportDto>;
  /** The report was read: go on to the editor. */
  onClose: () => void;
}

type Phase = { kind: "idle" } | { kind: "running"; progress: AdoptProgressDto | null } | { kind: "done"; report: AdoptReportDto } | { kind: "failed"; message: string };

const box: CSSProperties = { maxWidth: 560, margin: "24px auto", padding: 16, border: "1px solid #d1d5db", borderRadius: 6, background: "var(--g4-flaeche-blatt)", color: "var(--g4-text-primaer)", fontFamily: "system-ui, sans-serif" };

/** The box the Script panel shows until rotom has taken the project's scripts over (spec §8.2). */
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
    <div style={box}>
      <h3 style={{ margin: "0 0 8px" }}>{TERMS.script.plural} übernehmen</h3>
      {phase.kind === "done" ? (
        <Report report={phase.report} onClose={onClose} />
      ) : (
        <>
          <p style={{ margin: "0 0 12px" }}>
            Das Studio legt <code>rotom.toml</code> und <code>.rotom/</code> im Projekt an und dekompiliert alle {fileCount} {TERMS.scriptFile.plural} nach{" "}
            <code>expanded/scripts/</code>. DSPRE benutzt danach dieselben Quellen und schaltet seinen alten Scripteditor ab.
          </p>
          {phase.kind === "running" ? (
            <Progress progress={phase.progress} total={fileCount} />
          ) : (
            <button onClick={() => void start()}>Übernehmen</button>
          )}
          {phase.kind === "failed" && (
            <p role="alert" style={{ color: "#b91c1c", margin: "12px 0 0" }}>
              {phase.message}
            </p>
          )}
        </>
      )}
    </div>
  );
}

function Progress({ progress, total }: { progress: AdoptProgressDto | null; total: number }) {
  const max = progress?.total ?? total;
  const done = progress?.done ?? 0;
  return (
    <div>
      <progress value={done} max={Math.max(max, 1)} style={{ width: "100%" }} />
      <div style={{ color: "#6b7280" }}>
        {done} / {max} {TERMS.scriptFile.plural}
      </div>
    </div>
  );
}

function Report({ report, onClose }: { report: AdoptReportDto; onClose: () => void }) {
  return (
    <div>
      <p style={{ margin: "0 0 8px" }}>
        {report.decompiled} {TERMS.scriptFile.plural} dekompiliert, {report.skipped_empty} leere übersprungen
        {report.failed.length > 0 ? `, ${report.failed.length} fehlgeschlagen` : ""}.
      </p>
      {report.failed.length > 0 && (
        <>
          <p style={{ margin: "0 0 4px" }}>Diese Dateien bleiben binär:</p>
          <ul style={{ margin: "0 0 12px", paddingLeft: 20 }}>
            {report.failed.map(([file, message]) => (
              <li key={file}>
                {TERMS.scriptFile.label} {id4(file)}: {message}
              </li>
            ))}
          </ul>
        </>
      )}
      <button onClick={onClose}>Weiter</button>
    </div>
  );
}
