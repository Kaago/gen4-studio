import { memo, useSyncExternalStore } from "react";
import type { CSSProperties } from "react";
import { TERMS, id4 } from "../terms";
import type { NavTarget } from "../navigation";
import { countsText, scriptFileOf, textArchiveOf } from "./problems";
import type { ProblemRow, ProblemSource, ProblemsStore } from "./problems";

/** What the problems panel needs from App. */
export interface ProblemsPanelContext {
  problems: ProblemsStore;
  /** Follows a link: a script problem opens the file at its line, a text problem the archive. */
  goTo: (target: NavTarget) => void;
}

const SOURCE_LABEL: Record<ProblemSource, string> = { compile: "Compile", text: "Text", live: "live" };

/** Where clicking a problem leads, or null when its file is neither a script file nor a text archive. */
export function problemTarget(row: ProblemRow): NavTarget | null {
  const file = scriptFileOf(row.file);
  if (file !== null) return { kind: "script", file, line: row.line };
  const archive = textArchiveOf(row.file);
  return archive === null ? null : { kind: "textArchive", index: archive };
}

/** "Scriptdatei 0042", "Level-Script 0042", "Textarchiv 0199"; any other path as it is. */
export function fileLabel(path: string): string {
  const script = scriptFileOf(path);
  if (script !== null) return `${path.endsWith(".rotom") ? TERMS.scriptFile.label : TERMS.levelScript.label} ${id4(script)}`;
  const archive = textArchiveOf(path);
  return archive !== null ? `${TERMS.textArchive.label} ${id4(archive)}` : path;
}

const cell: CSSProperties = { padding: "2px 8px", textAlign: "left", whiteSpace: "nowrap" };

/** The problems of the project as a table (spec §8.2). Subscribes to the store itself, so a new problem renders nothing else. */
export const ProblemsPanel = memo(function ProblemsPanel({ context }: { context: ProblemsPanelContext }) {
  const { problems, goTo } = context;
  const rows = useSyncExternalStore(problems.subscribe, problems.list);
  const counts = useSyncExternalStore(problems.subscribe, problems.counts);
  if (rows.length === 0) return <p style={{ margin: 0, padding: "8px 12px", color: "#6b7280", fontFamily: "system-ui, sans-serif" }}>Keine {TERMS.problem.plural}.</p>;
  return (
    <div style={{ height: "100%", overflowY: "auto", fontFamily: "system-ui, sans-serif", fontSize: 13 }}>
      <table style={{ width: "100%", borderCollapse: "collapse" }}>
        <caption style={{ ...cell, captionSide: "top", color: "#6b7280" }}>{countsText(counts)}</caption>
        <thead>
          <tr style={{ color: "#6b7280", fontSize: 11, textTransform: "uppercase" }}>
            <th style={cell}>Schwere</th>
            <th style={cell}>Datei</th>
            <th style={cell}>Zeile</th>
            <th style={{ ...cell, width: "100%" }}>Meldung</th>
            <th style={cell}>Quelle</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row, i) => {
            const target = problemTarget(row);
            return (
              <tr
                key={`${row.source}:${row.file}:${row.line}:${row.col}:${i}`}
                data-problem={row.severity}
                tabIndex={target ? 0 : undefined}
                onClick={target ? () => goTo(target) : undefined}
                onKeyDown={target ? (e) => e.key === "Enter" && goTo(target) : undefined}
                style={{ cursor: target ? "pointer" : "default", borderTop: "1px solid #f3f4f6" }}
              >
                <td style={{ ...cell, color: row.severity === "error" ? "#b91c1c" : "#b45309", fontWeight: 600 }}>{row.severity === "error" ? "Fehler" : "Warnung"}</td>
                <td style={cell} title={row.file}>
                  {fileLabel(row.file)}
                </td>
                <td style={cell}>{row.line}</td>
                <td style={{ ...cell, whiteSpace: "normal" }}>{row.message}</td>
                <td style={cell}>{SOURCE_LABEL[row.source]}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
});
