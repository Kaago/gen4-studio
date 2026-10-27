import { memo, useId, useState, useSyncExternalStore } from "react";
import type { ReactNode } from "react";
import type { CheckProblemDto } from "../../api";
import { AREA_LABELS, CHECK_CODES, areaOf, checkTitle, findingLabel, findingText, progressText, skippedText } from "../../checks/messages";
import type { CheckArea } from "../../checks/messages";
import { notChecked } from "../../checks/useChecks";
import type { ChecksApi, ChecksView } from "../../checks/useChecks";
import { problemText } from "../../data/trainer/trainerModel";
import type { Depth } from "../../ui/frame/shellState";
import { Badge, Button, CheckList, EmptyState, Icon, Notice, PanelHead, Reference, Section } from "../../ui/primitives";
import type { Area, CheckRow } from "../../ui/primitives";
import { useSectionFolds } from "../inspector/foldState";
import { TERMS, id4 } from "../terms";
import type { NavTarget } from "../navigation";
import { FixDialog } from "./FixDialog";
import { NO_STATE_HINT_FILE, buildProblemText, countsText, encounterFileOf, findingId, isFinding, problemTarget, scriptFileOf, textArchiveOf, trainerOf } from "./problems";
import type { ProblemRow, ProblemSource, ProblemsStore } from "./problems";
import styles from "./ProblemsPanel.module.css";

/** What the problems panel needs from App. */
export interface ProblemsPanelContext {
  problems: ProblemsStore;
  /** Tüfteln shows the path of a file instead of its label, and the code of a check. */
  depth: Depth;
  /** Follows a link: a script problem opens the file at its line, a text problem the archive, a finding its target. */
  goTo: (target: NavTarget) => void;
  /** The checks of the project: „Beheben…“, „Ausblenden“, the foot with „Erneut prüfen“. Null: none (the rows still show). */
  checks: ChecksApi | null;
}

const SOURCE_LABEL: Record<ProblemSource, string> = { compile: "Compile", text: "Text", live: "live", encounter: "Begegnungen", trainer: "Trainer", check: "Prüfung", build: "Bauen" };
const SEVERITY_LABEL: Record<ProblemRow["severity"], string> = { error: "Fehler", warning: "Warnung", info: "Hinweis" };

/** "Scriptdatei 0042", "Level-Script 0042", "Textarchiv 0199", "Begegnungen 0012", "Trainer 0012", "Bauzustand"; any other path as it is. */
export function fileLabel(path: string): string {
  if (path === NO_STATE_HINT_FILE) return "Bauzustand";
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

/** The order of the areas in the panel. */
const AREAS: readonly CheckArea[] = ["welt", "scripts", "texte", "pokemon", "projekt"];

const position = (code: string) => {
  const at = CHECK_CODES.indexOf(code);
  return at < 0 ? CHECK_CODES.length : at;
};

/** The findings by area, and in each area by code in catalogue order. */
function groupFindings(findings: readonly CheckProblemDto[]) {
  const byArea = new Map<CheckArea, Map<string, CheckProblemDto[]>>();
  for (const f of findings) {
    const area = byArea.get(areaOf(f)) ?? byArea.set(areaOf(f), new Map()).get(areaOf(f))!;
    (area.get(f.code) ?? area.set(f.code, []).get(f.code)!).push(f);
  }
  return AREAS.filter((area) => byArea.has(area)).map((area) => {
    const codes = [...byArea.get(area)!].sort(([a], [b]) => position(a) - position(b));
    return { area, count: codes.reduce((n, [, rows]) => n + rows.length, 0), codes };
  });
}

/** The sentence of a finding and, below it, where it is („Eventdatei 0411 · Warp 2“; Tüfteln adds the source line). */
function FindingLines({ f, depth }: { f: CheckProblemDto; depth: Depth }) {
  return (
    <>
      <span className={styles.sentence}>{findingText(f, depth)}</span>
      <span className={styles.where}>
        {findingLabel(f, depth)}
        {depth === "tuefteln" && f.file !== "" ? ` · ${f.file}:${f.line}` : ""}
      </span>
    </>
  );
}

/**
 * A finding as a row of a `CheckList`: its state, its words with its buttons below them (the panel is often a narrow column of the hand),
 * `data-check` (the code) and `data-finding`.
 */
export function findingRow(f: CheckProblemDto, depth: Depth, actions?: ReactNode): CheckRow {
  const id = findingId(f);
  return {
    id,
    state: f.severity,
    text: (
      <>
        <FindingLines f={f} depth={depth} />
        {actions}
      </>
    ),
    dataAttrs: { "data-check": f.code, "data-finding": id },
  };
}

/** „Öffnen“ (with a target, unless `open` is false: the record is shown already), „Beheben…“ (with a fix), „Ausblenden“ (never for an error). */
export function FindingActions({ f, goTo, checks, onFix, open = true }: { f: CheckProblemDto; goTo?: (target: NavTarget) => void; checks: ChecksApi | null; onFix: (f: CheckProblemDto) => void; open?: boolean }) {
  const target = f.target;
  const canOpen = open && target !== undefined && goTo !== undefined;
  const canFix = checks !== null && f.fixes.length > 0;
  const canIgnore = checks !== null && f.severity !== "error";
  if (!canOpen && !canFix && !canIgnore) return null;
  return (
    <span className={styles.actions}>
      {canOpen && (
        <Button kind="quiet" size="s" onClick={() => goTo(target)}>
          Öffnen
        </Button>
      )}
      {canFix && (
        <Button kind="quiet" size="s" onClick={() => onFix(f)}>
          Beheben…
        </Button>
      )}
      {canIgnore && (
        <Button kind="quiet" size="s" onClick={() => void checks.ignore(f)}>
          Ausblenden
        </Button>
      )}
    </span>
  );
}

/** The findings of one code: its title and count, then its rows. */
function CodeGroup({ code, rows, depth, goTo, checks, onFix }: { code: string; rows: readonly CheckProblemDto[]; depth: Depth; goTo: (target: NavTarget) => void; checks: ChecksApi | null; onFix: (f: CheckProblemDto) => void }) {
  const headId = useId();
  const title = checkTitle(code, depth);
  return (
    <div role="group" aria-labelledby={headId} className={styles.code} data-check={code}>
      <h4 id={headId} className={styles.codeHead}>
        {title}
        <span className={styles.count}>{rows.length}</span>
        {depth === "tuefteln" && <span className={styles.tech}>{code}</span>}
      </h4>
      <CheckList ariaLabel={title} rows={rows.map((f) => findingRow(f, depth, <FindingActions f={f} goTo={goTo} checks={checks} onFix={onFix} />))} />
    </div>
  );
}

/** The findings of the checks, by area (Welt, Scripts, Texte, Pokémon) and code, each with its count; an area folds. */
function FindingGroups({ findings, depth, goTo, checks, onFix }: { findings: readonly CheckProblemDto[]; depth: Depth; goTo: (target: NavTarget) => void; checks: ChecksApi | null; onFix: (f: CheckProblemDto) => void }) {
  const folds = useSectionFolds();
  return (
    <div className={styles.findings}>
      {groupFindings(findings).map(({ area, count, codes }) => (
        <Section key={area} title={AREA_LABELS[area]} count={count} open={folds.isOpen(`problems:${area}`)} onToggle={() => folds.toggle(`problems:${area}`)}>
          {codes.map(([code, rows]) => (
            <CodeGroup key={code} code={code} rows={rows} depth={depth} goTo={goTo} checks={checks} onFix={onFix} />
          ))}
        </Section>
      ))}
    </div>
  );
}

/**
 * The problems (spec §7.4) and the findings of the checks (spec 3b.4 §8); the panel and Technik › Prüfen und Bauen both render it. The
 * findings come grouped by area and code with „Öffnen“, „Beheben…“ and „Ausblenden“; the problems of a save, a build, a text and the
 * live diagnostics as a table below them.
 */
export function ProblemsTable({ rows, depth, goTo, checks = null }: { rows: readonly ProblemRow[]; depth: Depth; goTo: (target: NavTarget) => void; checks?: ChecksApi | null }) {
  const [fixing, setFixing] = useState<CheckProblemDto | null>(null);
  const findings = rows.filter(isFinding);
  const others = rows.filter((row) => !isFinding(row));
  return (
    <div className={styles.tables}>
      {findings.length > 0 && <FindingGroups findings={findings} depth={depth} goTo={goTo} checks={checks} onFix={setFixing} />}
      {others.length > 0 && (
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
            {others.map((row, i) => {
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
                    {row.source === "trainer" ? problemText(row) : row.source === "build" ? buildProblemText(row) : row.message}
                  </td>
                  <td role="cell">
                    <Badge kind="neutral">{SOURCE_LABEL[row.source]}</Badge>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      )}
      {fixing !== null && checks !== null && <FixDialog finding={fixing} checks={checks} depth={depth} onClose={() => setFixing(null)} />}
    </div>
  );
}

/** Why an empty list says nothing while the checks have no result, and the way out (the foot's „Erneut prüfen“). */
export const UNCHECKED_WHY = "Die Prüfungen haben für dieses Projekt noch kein Ergebnis. Starte sie mit „Erneut prüfen“.";

/**
 * Beside problems of a save or a build, while the checks have no result and none runs: a note says so (`data-unchecked`), so the list does
 * not read as all there is. Nothing otherwise.
 */
export function UncheckedNotice({ checks }: { checks: ChecksApi }) {
  const view = useSyncExternalStore(checks.feed.subscribe, checks.feed.get);
  if (!notChecked(view)) return null;
  return (
    <div className={styles.notes} data-unchecked="">
      <Notice kind="info" title="Nicht geprüft">
        {UNCHECKED_WHY}
      </Notice>
    </div>
  );
}

/**
 * What the list says while the checks have told nothing: the first run is under way (null view: no checks, nothing to say), or no report
 * arrived and none runs.
 */
export function checksPending(view: ChecksView | null): "first" | "unchecked" | null {
  if (view === null || view.reported) return null;
  return view.running ? "first" : notChecked(view) ? "unchecked" : null;
}

/** The checks that did not run, or left part of their work undone, as hint lines beside the findings. */
export function ChecksNotes({ checks, depth }: { checks: ChecksApi; depth: Depth }) {
  const view = useSyncExternalStore(checks.feed.subscribe, checks.feed.get);
  if (view.skipped.length === 0) return null;
  return (
    <div className={styles.notes}>
      <CheckList ariaLabel="Nicht geprüft" rows={view.skipped.map((s) => ({ id: `${s.code}:${s.why}`, state: "info", text: skippedText(s, depth), dataAttrs: { "data-check": s.code } }))} />
    </div>
  );
}

/**
 * The foot of the checks: a running pass and how far it got, „12 ausgeblendet · Wieder melden…“ with the ignored findings, „Erneut prüfen“.
 * The list of the ignored ones starts folded, folds when it empties and after „Ausgangslage übernehmen“ (a baseline adds a hundred at once;
 * a single „Ausblenden“ leaves it open), and scrolls within its own height.
 */
export function ChecksFoot({ checks, depth }: { checks: ChecksApi; depth: Depth }) {
  const view = useSyncExternalStore(checks.feed.subscribe, checks.feed.get);
  const [showIgnored, setShowIgnored] = useState(false);
  const ignored = view.ignored.length;
  const [seen, setSeen] = useState({ ignored, baselines: view.baselines });
  if (seen.ignored !== ignored || seen.baselines !== view.baselines) {
    setSeen({ ignored, baselines: view.baselines });
    if (ignored === 0 || view.baselines !== seen.baselines) setShowIgnored(false);
  }
  return (
    <footer className={styles.foot}>
      <div className={styles.footRow}>
        {view.running && (
          <span role="status" className={styles.progress}>
            <span className={styles.spinner} aria-hidden="true" />
            {progressText(view.progress)}
          </span>
        )}
        {ignored > 0 && (
          <span className={styles.ignored}>
            {`${ignored} ausgeblendet`}
            <span aria-hidden="true">·</span>
            <Button kind="quiet" size="s" aria-expanded={showIgnored} onClick={() => setShowIgnored(!showIgnored)}>
              Wieder melden…
            </Button>
          </span>
        )}
        <Button kind="secondary" size="s" disabled={view.running} onClick={() => void checks.recheck()}>
          Erneut prüfen
        </Button>
      </div>
      {showIgnored && ignored > 0 && (
        <div className={styles.ignoredList}>
          <CheckList
            ariaLabel="Ausgeblendet"
            rows={view.ignored.map((f) =>
              findingRow(
                f,
                depth,
                <span className={styles.actions}>
                  <Button kind="quiet" size="s" onClick={() => void checks.unignore(f)}>
                    Wieder melden
                  </Button>
                </span>,
              ),
            )}
          />
        </div>
      )}
    </footer>
  );
}

const noSubscribe = () => () => {};
const noView = () => null;

/** The problems of the project (spec §7.4) and the findings of its checks. Subscribes to the stores itself, so a new problem renders nothing else. */
export const ProblemsPanel = memo(function ProblemsPanel({ context }: { context: ProblemsPanelContext }) {
  const { problems, depth, goTo, checks } = context;
  const rows = useSyncExternalStore(problems.subscribe, problems.list);
  const counts = useSyncExternalStore(problems.subscribe, problems.counts);
  const view = useSyncExternalStore(checks?.feed.subscribe ?? noSubscribe, checks?.feed.get ?? noView);
  // Before the first report of a project the list is empty because nothing is known yet, not because all is well.
  const pending = checksPending(view);
  return (
    <section className={styles.panel} aria-label="Probleme">
      {/* The dock tab says "Probleme": the head keeps the counts, and is left out while there are none. */}
      {countsText(counts) !== "" && <PanelHead subtitle={countsText(counts)} />}
      <div className={styles.body}>
        {checks !== null && <ChecksNotes checks={checks} depth={depth} />}
        {rows.length > 0 && checks !== null && <UncheckedNotice checks={checks} />}
        {rows.length === 0 && pending === "first" ? (
          <EmptyState what="Prüft das Projekt …" why="Die Befunde der Prüfungen erscheinen hier, sobald sie fertig sind." />
        ) : rows.length === 0 && pending === "unchecked" ? (
          <EmptyState what="Nicht geprüft." why={UNCHECKED_WHY} />
        ) : rows.length === 0 ? (
          <EmptyState what={`Keine ${TERMS.problem.plural}.`} why="Beim Speichern eines Scripts oder Texts landen Fehler hier." />
        ) : (
          <ProblemsTable rows={rows} depth={depth} goTo={goTo} checks={checks} />
        )}
      </div>
      {checks !== null && <ChecksFoot checks={checks} depth={depth} />}
    </section>
  );
});
