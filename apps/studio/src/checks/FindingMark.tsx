import { useMemo, useSyncExternalStore } from "react";
import type { HeaderSummary, SubjectKind } from "../api";
import { Icon } from "../ui/primitives";
import { isFinding } from "../workbench/problems/problems";
import type { ProblemRow, ProblemsStore } from "../workbench/problems/problems";
import styles from "./FindingMark.module.css";

/** What a list row is marked with: the worst finding of its subject (infos mark nothing). */
export type MarkSeverity = "error" | "warning";

const worse = (a: MarkSeverity | undefined, b: MarkSeverity): MarkSeverity => (a === "error" || b === "error" ? "error" : "warning");

function add(marks: Map<number, MarkSeverity>, index: number, severity: MarkSeverity) {
  marks.set(index, worse(marks.get(index), severity));
}

/** The worst error or warning of the checks per subject index of `kind` (trainers, encounter files, ...). */
export function marksOf(rows: readonly ProblemRow[], kind: SubjectKind): Map<number, MarkSeverity> {
  const marks = new Map<number, MarkSeverity>();
  for (const row of rows) {
    if (!isFinding(row) || row.severity === "info" || row.subject.kind !== kind) continue;
    add(marks, row.subject.index, row.severity);
  }
  return new Map([...marks].sort(([a], [b]) => a - b));
}

/** The worst error or warning per header: its own findings and those of its event file (its warps, figures, signs and triggers). */
export function headerMarks(rows: readonly ProblemRow[], headers: readonly Pick<HeaderSummary, "id" | "event_file_id">[]): Map<number, MarkSeverity> {
  const own = marksOf(rows, "header");
  const events = marksOf(rows, "eventFile");
  const marks = new Map<number, MarkSeverity>();
  for (const h of headers) {
    const mine = own.get(h.id);
    const theirs = events.get(h.event_file_id);
    if (mine) add(marks, h.id, mine);
    if (theirs) add(marks, h.id, theirs);
  }
  return marks;
}

/** The marks of `kind` from the store, as a new map only when the findings changed. */
export function useFindingMarks(problems: ProblemsStore | null, kind: SubjectKind): ReadonlyMap<number, MarkSeverity> {
  const rows = useSyncExternalStore(problems?.subscribe ?? noSubscribe, problems?.list ?? noRows);
  return useMemo(() => marksOf(rows, kind), [rows, kind]);
}

const NO_ROWS: readonly ProblemRow[] = [];
const noRows = () => NO_ROWS;
const noSubscribe = () => () => {};

const LABEL: Record<MarkSeverity, string> = { error: "Fehler gefunden", warning: "Warnung gefunden" };

/** The mark at the end of a list row whose subject a check found an error or warning at: a symbol with its name, never colour alone. */
export function FindingMark({ severity }: { severity: MarkSeverity }) {
  return (
    <span className={`${styles.mark} ${styles[severity]}`} role="img" aria-label={LABEL[severity]} title={`${LABEL[severity]}, siehe Probleme`} data-finding-mark={severity}>
      <Icon name={severity} weight="fill" />
    </span>
  );
}
