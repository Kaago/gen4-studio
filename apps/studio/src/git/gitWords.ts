import type { GitStatus, InitReport } from "../api";
import { NBSP } from "../words";

/** The changed paths since the last commit: „keine Änderung“, „1 Änderung“, „{n} Änderungen“. */
export function changesWord(n: number): string {
  if (n === 0) return "keine Änderung";
  return n === 1 ? "1 Änderung" : `${n} Änderungen`;
}

/**
 * What the status line says about Git: „main · 3 Änderungen“; without a commit „main · noch kein Commit“ (the branch to be), with a detached
 * `HEAD` „4c1d2e3 (losgelöst) · 2 Änderungen“. Null without a repository (the status line shows nothing then).
 */
export function statusText(s: GitStatus): string | null {
  if (s.repo.kind === "none") return null;
  if (s.head === null) return `${s.branch ?? "HEAD"} · noch kein Commit`;
  const where = s.detached || s.branch === null ? `${s.head.short} (losgelöst)` : s.branch;
  return `${where} · ${changesWord(s.changes)}`;
}

/** The tooltip of the status line's item: the count and the last commit, and where a click leads; without a commit only that. */
export function statusTooltip(s: GitStatus): string {
  if (s.head === null) return "Git: noch kein Commit.";
  const files = s.changes === 1 ? "1 geänderte Datei" : `${s.changes} geänderte Dateien`;
  return `Git: ${files} seit dem letzten Commit „${s.head.summary}“. Klick zeigt sie in Technik › Änderungen.`;
}

/** The message after a first commit: how many files it holds, and when Git knew no name, why the author is Gen4 Studio. */
export function initDoneText(report: InitReport): string {
  const done = `Git eingerichtet: ${report.files.toLocaleString("de-DE")}${NBSP}Dateien im ersten Commit „Ausgangsstand“.`;
  return report.studio_identity ? `${done} Autor ist „Gen4 Studio“, weil Git deinen Namen noch nicht kennt.` : done;
}

/** The kernel's reason when no `git` was found (`git::NOT_INSTALLED`): only the templates were written. */
export const NOT_INSTALLED = "Git ist nicht installiert. Die Vorlagen sind geschrieben; das Repository legst du an, sobald Git da ist (git-scm.com/downloads).";

/** The message when no `git` is installed: only the templates were written. */
export const TEMPLATES_ONLY_TEXT = "Vorlagen geschrieben. Git ist nicht installiert; das Repository legst du an, sobald Git da ist.";
