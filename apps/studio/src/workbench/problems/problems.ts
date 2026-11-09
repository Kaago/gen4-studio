import type { CheckProblemDto, ProblemDto } from "../../api";
import { findingText } from "../../checks/messages";
import { t } from "../../i18n";
import type { NavTarget } from "../navigation";
import { id4 } from "../terms";

/**
 * Where a problem comes from: the last save or build (`compile`), a text archive (`text`), the language server of the shown document
 * (`live`), a trainer's battle texts (`trainer`), the project checks (`check`), the build itself (`build`: the no-state hint, a hidden
 * file, a text conflict); `encounter` is the source a data document gives the rules of an encounter file, which the checks
 * (`encounter.*`) report now.
 */
export type ProblemSource = ProblemDto["source"] | "live";

/** A row of the list: a `ProblemDto` with the source `live` besides; a finding of the checks keeps all its fields (see `isFinding`). */
export type ProblemRow = Omit<ProblemDto, "source"> & { source: ProblemSource };

export interface ProblemCounts {
  errors: number;
  warnings: number;
  /** Only the checks report infos („Hinweise“). */
  infos: number;
}

/** Whether `row` is a finding of the checks (source `check`, with its code, subject, key and fingerprint). */
export const isFinding = (row: ProblemRow): row is CheckProblemDto => row.source === "check" && row.code !== undefined;

/** What names a finding (`data-finding`): its code, subject and key, "warp.dest-warp|eventFile:411|warp:2". */
export const findingId = (f: Pick<CheckProblemDto, "code" | "subject" | "key">): string => `${f.code}|${f.subject.kind}:${f.subject.index}|${f.key}`;

const SCRIPT_PATH = /^expanded\/scripts\/(\d+)\.(rotom|json)$/;
const TEXT_PATH = /^expanded\/textArchives\/(\d+)\.json$/;
const ENCOUNTER_PATH = /^encounters\/(\d+)$/;
const TRAINER_PATH = /^trainers\/(\d+)$/;

/** The file of the build's hint that text archives had no build state (the kernel's `NO_STATE_HINT_FILE`). */
export const NO_STATE_HINT_FILE = ".g4/build/state.json";
const NO_STATE_HINT = /^(\d+) text archives? without build state/;
/** How the build's problem for a hidden file below `files/` begins (the code of the check `build.hidden-file`). */
const HIDDEN_FILE_PREFIX = "build.hidden-file: ";

/**
 * The sentence of a build row (source `build`) in the language that is shown: the no-state hint („3 Textarchive ohne Bauzustand: die JSON-Fassung gilt.“) and a
 * hidden file (the sentence of the check `build.hidden-file`) from the kernel's English; any other message (a text conflict, which the
 * backend words in German) as it is.
 */
export function buildProblemText(row: Pick<ProblemDto, "file" | "message">): string {
  if (row.message.startsWith(HIDDEN_FILE_PREFIX)) {
    return findingText({ code: "build.hidden-file", key: row.file, args: [row.file], message: row.message }, "bauen");
  }
  const hint = row.file === NO_STATE_HINT_FILE ? NO_STATE_HINT.exec(row.message) : null;
  if (hint) {
    return t("workbench:problemsModel.noBuildState", { count: Number(hint[1]) });
  }
  return row.message;
}

/** The project-relative path of a script file's source (`.json` for a level script). */
export function scriptPath(file: number, kind: "script" | "levelScript"): string {
  return `expanded/scripts/${id4(file)}.${kind === "script" ? "rotom" : "json"}`;
}

/** The script file a problem's path names, or null for any other path. */
export function scriptFileOf(path: string): number | null {
  const match = SCRIPT_PATH.exec(path);
  return match ? Number(match[1]) : null;
}

/** The text archive a problem's path names, or null for any other path. */
export function textArchiveOf(path: string): number | null {
  const match = TEXT_PATH.exec(path);
  return match ? Number(match[1]) : null;
}

/** The encounter file a problem's path names ("encounters/0012"), or null for any other path. */
export function encounterFileOf(path: string): number | null {
  const match = ENCOUNTER_PATH.exec(path);
  return match ? Number(match[1]) : null;
}

/** The trainer a problem's path names ("trainers/0012"), or null for any other path. */
export function trainerOf(path: string): number | null {
  const match = TRAINER_PATH.exec(path);
  return match ? Number(match[1]) : null;
}

/** Structural equality of plain data (JSON values): the same keys with equal values, in any order; a missing key equals `undefined`. */
function equalData(a: unknown, b: unknown): boolean {
  if (a === b) return true;
  if (typeof a !== "object" || typeof b !== "object" || a === null || b === null || Array.isArray(a) !== Array.isArray(b)) return false;
  const left = a as Record<string, unknown>;
  const right = b as Record<string, unknown>;
  const keys = new Set([...Object.keys(left), ...Object.keys(right)]);
  for (const key of keys) if (!equalData(left[key], right[key])) return false;
  return true;
}

/**
 * Whether two rows are the same: a finding by everything it shows (its code, subject, key and fingerprint name it; its sentence's
 * numbers, label, fixes and targets render it, and they may change while it stays the same finding: „Beheben…“ appears once the scripts
 * are adopted); any other row by its severity, place and message.
 */
function sameRow(a: ProblemRow, b: ProblemRow): boolean {
  if (isFinding(a) || isFinding(b)) return isFinding(a) && isFinding(b) && equalData(a, b);
  return a.severity === b.severity && a.file === b.file && a.line === b.line && a.col === b.col && a.message === b.message;
}

const same = (a: readonly ProblemRow[], b: readonly ProblemRow[]) => a.length === b.length && a.every((p, i) => sameRow(p, b[i]!));

/** Errors first, then warnings, then infos. */
const RANK: Record<ProblemRow["severity"], number> = { error: 0, warning: 1, info: 2 };

/** "2 Fehler, 1 Warnung, 3 Hinweise"; an empty string when there is nothing. */
export function countsText({ errors, warnings, infos }: ProblemCounts): string {
  const parts: string[] = [];
  if (errors > 0) parts.push(t("workbench:problemsModel.errors", { count: errors }));
  if (warnings > 0) parts.push(t("workbench:problemsModel.warnings", { count: warnings }));
  if (infos > 0) parts.push(t("workbench:problemsModel.infos", { count: infos }));
  return parts.join(", ");
}

/**
 * Where „Öffnen“ (or a click on a row's file) leads: a finding to its `target` (none: nowhere); any other row by its file, a script file at
 * its line, a text archive, a trainer, an encounter file; null for any other file.
 */
export function problemTarget(row: ProblemRow): NavTarget | null {
  if (isFinding(row)) return row.target ?? null;
  const file = scriptFileOf(row.file);
  if (file !== null) return { kind: "script", file, line: row.line };
  const trainer = trainerOf(row.file);
  if (trainer !== null) return { kind: "trainer", index: trainer };
  const encounter = encounterFileOf(row.file);
  if (encounter !== null) return { kind: "encounter", index: encounter };
  const archive = textArchiveOf(row.file);
  return archive === null ? null : { kind: "textArchive", index: archive };
}

/**
 * The problems of the project, by where they came from: the compile problems of each file, the warnings of each text
 * archive, the rows of each data member, the live diagnostics of the shown document and the findings of the checks. Each
 * source is replaced as a whole, so a fixed problem goes and the others stay. A live row with the same file, line and
 * severity as a compile row is not listed or counted. Plain store for `useSyncExternalStore`: `list` and `counts` keep
 * their identity until something changed.
 */
export class ProblemsStore {
  private readonly compile = new Map<string, ProblemRow[]>();
  private readonly text = new Map<number, ProblemRow[]>();
  private readonly data = new Map<string, ProblemRow[]>();
  private live: ProblemRow[] = [];
  private checks: ProblemRow[] = [];
  private rows: readonly ProblemRow[] = [];
  private count: ProblemCounts = { errors: 0, warnings: 0, infos: 0 };
  private readonly listeners = new Set<() => void>();

  private static rows(problems: readonly Omit<ProblemDto, "source">[], source: ProblemSource): ProblemRow[] {
    return problems.map((p) => ({ severity: p.severity, file: p.file, line: p.line, col: p.col, message: p.message, source }));
  }

  /** Replaces the compile problems of one file (an empty list clears it); the build's own rows keep their source `build`. */
  replaceForFile(file: string, problems: readonly ProblemDto[]): void {
    this.set(
      this.compile,
      file,
      problems.map((p) => ({ severity: p.severity, file: p.file, line: p.line, col: p.col, message: p.message, source: p.source === "build" ? "build" : "compile" })),
    );
  }

  /** Replaces the warnings of one text archive (an empty list clears it). */
  replaceText(archive: number, problems: readonly ProblemDto[]): void {
    this.set(this.text, archive, ProblemsStore.rows(problems, "text"));
  }

  /**
   * Replaces the rows of one data member (`file` "encounters/0012" or "trainers/0012") with `problems` as the backend gives them in the
   * document (`data_get`, source "encounter" or "trainer"): every read of the member replaces its rows, so they never pile up.
   */
  replaceData(file: string, problems: readonly ProblemDto[]): void {
    this.set(
      this.data,
      file,
      problems.map((p) => ({ severity: p.severity, file: p.file, line: p.line, col: p.col, message: p.message, source: p.source })),
    );
  }

  /**
   * Replaces the findings of the checks as a whole (`useChecks` hands over what the last runs found, rows of source `check`): a finding that
   * is gone goes, the same findings again change nothing.
   */
  replaceChecks(findings: readonly ProblemDto[]): void {
    const next: ProblemRow[] = findings.map((f) => ({ ...f, source: "check" }));
    if (same(this.checks, next)) return;
    this.checks = next;
    this.changed();
  }

  /** Replaces the live diagnostics (those of the shown document; none when no document is shown). */
  replaceLive(problems: readonly Omit<ProblemDto, "source">[]): void {
    const next = ProblemsStore.rows(problems, "live");
    if (same(this.live, next)) return;
    this.live = next;
    this.changed();
  }

  /**
   * Takes the problems a save or build reported: compile problems per file, text problems per archive. `cleared`
   * names the files and archives that were checked and may have none left (a file without problems is not in the list).
   */
  applyReport(problems: readonly ProblemDto[], cleared: { files?: readonly string[]; archives?: readonly number[] } = {}): void {
    const byFile = new Map<string, ProblemDto[]>((cleared.files ?? []).map((f) => [f, []]));
    const byArchive = new Map<number, ProblemDto[]>((cleared.archives ?? []).map((a) => [a, []]));
    for (const p of problems) {
      if (p.source === "text") {
        const archive = textArchiveOf(p.file);
        // A text that belongs to no file yet has nothing to attach to.
        if (archive !== null) (byArchive.get(archive) ?? byArchive.set(archive, []).get(archive)!).push(p);
      } else {
        (byFile.get(p.file) ?? byFile.set(p.file, []).get(p.file)!).push(p);
      }
    }
    for (const [file, list] of byFile) this.replaceForFile(file, list);
    for (const [archive, list] of byArchive) this.replaceText(archive, list);
  }

  /**
   * Takes the problems of a build that succeeded. Any script that does not compile stops the build, so no compile error
   * is left: the error rows (and the build's hints) of files the report does not name are stale and go. Warnings stay (a file that was not
   * stale is not compiled again, so its warnings are not reported again); a file the report names is replaced as a whole.
   */
  applyBuildReport(problems: readonly ProblemDto[]): void {
    for (const [file, rows] of [...this.compile]) {
      this.replaceForFile(file, rows.filter((r) => r.severity === "warning").map((r) => ({ ...r, source: r.source === "build" ? "build" : "compile" })));
    }
    this.applyReport(problems);
  }

  /** Forgets everything (another project was opened). */
  clear(): void {
    if (this.compile.size === 0 && this.text.size === 0 && this.data.size === 0 && this.live.length === 0 && this.checks.length === 0) return;
    this.compile.clear();
    this.text.clear();
    this.data.clear();
    this.live = [];
    this.checks = [];
    this.changed();
  }

  /** Errors first, then warnings, then infos, each by file and line; the same array until a source changed. */
  list = (): readonly ProblemRow[] => this.rows;

  /** The same object until a source changed. */
  counts = (): ProblemCounts => this.count;

  subscribe = (listener: () => void): (() => void) => {
    this.listeners.add(listener);
    return () => void this.listeners.delete(listener);
  };

  private set<K>(map: Map<K, ProblemRow[]>, key: K, next: ProblemRow[]): void {
    const previous = map.get(key) ?? [];
    if (same(previous, next)) return;
    if (next.length === 0) map.delete(key);
    else map.set(key, next);
    this.changed();
  }

  private changed(): void {
    const compile = [...this.compile.values()].flat();
    // The language server and the compile find the same fault: one row, the compile's (whose source says "Compile").
    const known = new Set(compile.map((p) => `${p.file}\0${p.line}\0${p.severity}`));
    const live = this.live.filter((p) => !known.has(`${p.file}\0${p.line}\0${p.severity}`));
    const all = [...compile, ...[...this.text.values()].flat(), ...[...this.data.values()].flat(), ...live, ...this.checks];
    all.sort((a, b) => RANK[a.severity] - RANK[b.severity] || a.file.localeCompare(b.file) || a.line - b.line || a.col - b.col);
    this.rows = all;
    const count = (severity: ProblemRow["severity"]) => all.filter((p) => p.severity === severity).length;
    this.count = { errors: count("error"), warnings: count("warning"), infos: count("info") };
    for (const listener of [...this.listeners]) listener();
  }
}
