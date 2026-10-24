import { memo, useSyncExternalStore } from "react";
import type { Depth } from "../../ui/frame/shellState";
import { Badge, EmptyState, Icon, PanelHead, Reference } from "../../ui/primitives";
import type { Area } from "../../ui/primitives";
import { TERMS, id4 } from "../terms";
import type { NavTarget } from "../navigation";
import { countsText, encounterFileOf, scriptFileOf, textArchiveOf, trainerOf } from "./problems";
import type { ProblemRow, ProblemSource, ProblemsStore } from "./problems";
import styles from "./ProblemsPanel.module.css";

/** What the problems panel needs from App. */
export interface ProblemsPanelContext {
  problems: ProblemsStore;
  /** Tüfteln shows the path of a file instead of its label. */
  depth: Depth;
  /** Follows a link: a script problem opens the file at its line, a text problem the archive. */
  goTo: (target: NavTarget) => void;
}

const SOURCE_LABEL: Record<ProblemSource, string> = { compile: "Compile", text: "Text", live: "live", encounter: "Begegnungen", trainer: "Trainer" };
const SEVERITY_LABEL: Record<ProblemRow["severity"], string> = { error: "Fehler", warning: "Warnung" };

/** Where clicking a problem leads, or null when its file is neither a script file nor a text archive. */
export function problemTarget(row: ProblemRow): NavTarget | null {
  const file = scriptFileOf(row.file);
  if (file !== null) return { kind: "script", file, line: row.line };
  const archive = textArchiveOf(row.file);
  return archive === null ? null : { kind: "textArchive", index: archive };
}

/** "Scriptdatei 0042", "Level-Script 0042", "Textarchiv 0199", "Begegnungen 0012", "Trainer 0012"; any other path as it is. */
export function fileLabel(path: string): string {
  const script = scriptFileOf(path);
  if (script !== null) return `${path.endsWith(".rotom") ? TERMS.scriptFile.label : TERMS.levelScript.label} ${id4(script)}`;
  const encounter = encounterFileOf(path);
  if (encounter !== null) return `Begegnungen ${id4(encounter)}`;
  const trainer = trainerOf(path);
  if (trainer !== null) return `Trainer ${id4(trainer)}`;
  const archive = textArchiveOf(path);
  return archive !== null ? `${TERMS.textArchive.label} ${id4(archive)}` : path;
}

/** The area a file belongs to: scripts are Welt, text archives Texte, encounter files and trainers Pokémon, anything else (a project file) Technik. */
function fileArea(path: string): Area {
  if (scriptFileOf(path) !== null) return "welt";
  if (encounterFileOf(path) !== null || trainerOf(path) !== null) return "pokemon";
  return textArchiveOf(path) !== null ? "texte" : "technik";
}

/** The problems as a table (spec §7.4); the panel and Technik › Prüfen und Bauen both render it. */
export function ProblemsTable({ rows, depth, goTo }: { rows: readonly ProblemRow[]; depth: Depth; goTo: (target: NavTarget) => void }) {
  return (
    <table role="table" className={styles.table}>
      <thead>
        <tr>
          <th scope="col">Schwere</th>
          <th scope="col">Datei</th>
          <th scope="col">Zeile:Spalte</th>
          <th scope="col" className={styles.message}>
            Meldung
          </th>
          <th scope="col">Quelle</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((row, i) => {
          const target = problemTarget(row);
          return (
            <tr key={`${row.source}:${row.file}:${row.line}:${row.col}:${i}`} role="row" data-problem={row.severity}>
              <td role="cell">
                <span role="img" aria-label={SEVERITY_LABEL[row.severity]} className={`${styles.severity} ${styles[row.severity]}`}>
                  <Icon name={row.severity} weight="fill" />
                </span>
              </td>
              <td role="cell">
                <Reference area={fileArea(row.file)} label={depth === "tuefteln" ? row.file : fileLabel(row.file)} onOpen={() => target && goTo(target)} disabled={target === null} />
              </td>
              <td role="cell" className={styles.position}>
                {row.line}:{row.col}
              </td>
              <td role="cell" className={styles.message}>
                {row.message}
              </td>
              <td role="cell">
                <Badge kind="neutral">{SOURCE_LABEL[row.source]}</Badge>
              </td>
            </tr>
          );
        })}
      </tbody>
    </table>
  );
}

/** The problems of the project (spec §7.4). Subscribes to the store itself, so a new problem renders nothing else. */
export const ProblemsPanel = memo(function ProblemsPanel({ context }: { context: ProblemsPanelContext }) {
  const { problems, depth, goTo } = context;
  const rows = useSyncExternalStore(problems.subscribe, problems.list);
  const counts = useSyncExternalStore(problems.subscribe, problems.counts);
  return (
    <section className={styles.panel} aria-label="Probleme">
      <PanelHead title="Probleme" subtitle={countsText(counts)} />
      {rows.length === 0 ? (
        <EmptyState what={`Keine ${TERMS.problem.plural}.`} why="Beim Speichern eines Scripts oder Texts landen Fehler hier." />
      ) : (
        <div className={styles.body}>
          <ProblemsTable rows={rows} depth={depth} goTo={goTo} />
        </div>
      )}
    </section>
  );
});
