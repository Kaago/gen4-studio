import type { ProblemDto } from "../../api";
import { id4 } from "../terms";

/** Where a problem comes from: the last save or build (`compile`), a text archive (`text`), or the language server of the shown document (`live`). */
export type ProblemSource = "compile" | "text" | "live";

export interface ProblemRow {
  severity: "error" | "warning";
  /** Relative to the project root with forward slashes. */
  file: string;
  line: number;
  col: number;
  message: string;
  source: ProblemSource;
}

export interface ProblemCounts {
  errors: number;
  warnings: number;
}

const SCRIPT_PATH = /^expanded\/scripts\/(\d+)\.(rotom|json)$/;
const TEXT_PATH = /^expanded\/textArchives\/(\d+)\.json$/;

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

const same = (a: readonly ProblemRow[], b: readonly ProblemRow[]) =>
  a.length === b.length && a.every((p, i) => p.severity === b[i]!.severity && p.file === b[i]!.file && p.line === b[i]!.line && p.col === b[i]!.col && p.message === b[i]!.message);

/** "2 Fehler, 1 Warnung"; an empty string when there is neither. */
export function countsText({ errors, warnings }: ProblemCounts): string {
  const parts: string[] = [];
  if (errors > 0) parts.push(`${errors} Fehler`);
  if (warnings > 0) parts.push(`${warnings} ${warnings === 1 ? "Warnung" : "Warnungen"}`);
  return parts.join(", ");
}

/**
 * The problems of the project, by where they came from: the compile problems of each file, the warnings of each text
 * archive and the live diagnostics of the shown document. Each source is replaced as a whole, so a fixed problem goes
 * and the others stay. Plain store for `useSyncExternalStore`: `list` and `counts` keep their identity until something changed.
 */
export class ProblemsStore {
  private readonly compile = new Map<string, ProblemRow[]>();
  private readonly text = new Map<number, ProblemRow[]>();
  private live: ProblemRow[] = [];
  private rows: readonly ProblemRow[] = [];
  private count: ProblemCounts = { errors: 0, warnings: 0 };
  private readonly listeners = new Set<() => void>();

  private static rows(problems: readonly Omit<ProblemDto, "source">[], source: ProblemSource): ProblemRow[] {
    return problems.map((p) => ({ severity: p.severity, file: p.file, line: p.line, col: p.col, message: p.message, source }));
  }

  /** Replaces the compile problems of one file (an empty list clears it). */
  replaceForFile(file: string, problems: readonly ProblemDto[]): void {
    this.set(this.compile, file, ProblemsStore.rows(problems, "compile"));
  }

  /** Replaces the warnings of one text archive (an empty list clears it). */
  replaceText(archive: number, problems: readonly ProblemDto[]): void {
    this.set(this.text, archive, ProblemsStore.rows(problems, "text"));
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

  /** Forgets everything (another project was opened). */
  clear(): void {
    if (this.compile.size === 0 && this.text.size === 0 && this.live.length === 0) return;
    this.compile.clear();
    this.text.clear();
    this.live = [];
    this.changed();
  }

  /** Errors first, then by file and line; the same array until a source changed. */
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
    const all = [...this.compile.values(), ...this.text.values(), this.live].flat();
    all.sort((a, b) => Number(b.severity === "error") - Number(a.severity === "error") || a.file.localeCompare(b.file) || a.line - b.line || a.col - b.col);
    this.rows = all;
    const errors = all.filter((p) => p.severity === "error").length;
    this.count = { errors, warnings: all.length - errors };
    for (const listener of [...this.listeners]) listener();
  }
}
