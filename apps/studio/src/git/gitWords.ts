import type { CommitReport, GitStatus, InitReport, TemplatesRewritten } from "../api";
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

/**
 * The tooltip of the status line's item: the count and the last commit, and where a click leads; without a commit only that. None changed
 * reads „keine geänderte Datei“, as the label says „keine Änderung“.
 */
export function statusTooltip(s: GitStatus): string {
  if (s.head === null) return "Git: noch kein Commit.";
  const since = `seit dem letzten Commit „${s.head.summary}“.`;
  if (s.changes === 0) return `Git: keine geänderte Datei ${since} Klick öffnet Technik › Änderungen.`;
  const files = s.changes === 1 ? "1 geänderte Datei" : `${s.changes} geänderte Dateien`;
  return `Git: ${files} ${since} Klick zeigt sie in Technik › Änderungen.`;
}

/** The checklist's row for a found `git`: its version without the „git version “ that `git --version` puts first. */
export function gitFoundText(version: string): string {
  return `Git gefunden: ${version.replace(/^git version /, "")}`;
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

/**
 * After a set-up or a take-back: what became of the Git templates, as a sentence with a leading space („ Die .gitignore von Gen4 Studio
 * versioniert jetzt das Textprojekt.“), the kernel's warning when they could not be written, empty when nothing changed or the project
 * never set up Git.
 */
export function templatesNote(report: TemplatesRewritten): string {
  if (report.templates_warning !== null) return ` ${report.templates_warning}`;
  const t = report.templates;
  if (t === null || !t.files.some((f) => f.path === ".gitignore" && f.action !== "unchanged")) return "";
  return t.layout === "text_project" ? " Die .gitignore von Gen4 Studio versioniert jetzt das Textprojekt." : " Die .gitignore von Gen4 Studio versioniert jetzt den DSPRE-Ordner.";
}

/**
 * The checklist's row of „Stand sichern“: the changed paths since the last commit, as the status counts them; in a repository without a
 * commit (`first`) the files the first commit takes („18.449 Dateien für den ersten Commit“).
 */
export function changedFilesText(n: number, first = false): string {
  if (n === 0) return "Keine Änderung seit dem letzten Commit";
  if (first) return n === 1 ? `1${NBSP}Datei für den ersten Commit` : `${n.toLocaleString("de-DE")}${NBSP}Dateien für den ersten Commit`;
  return n === 1 ? "1 geänderte Datei seit dem letzten Commit" : `${n.toLocaleString("de-DE")} geänderte Dateien seit dem letzten Commit`;
}

/** The message after „Stand sichern“: the new commit's short id, its subject and how many files went in; when Git knew no name, why the author is Gen4 Studio. */
export function commitDoneText(report: CommitReport): string {
  const files = report.files === 1 ? `1${NBSP}Datei` : `${report.files.toLocaleString("de-DE")}${NBSP}Dateien`;
  const where = report.head === null ? "" : `${report.head.short} `;
  const done = `Stand gesichert: ${where}„${report.subject}“ (${files}).`;
  return report.studio_identity ? `${done} Autor ist „Gen4 Studio“, weil Git deinen Namen noch nicht kennt.` : done;
}

/** Why „Stand sichern…“ in Technik › Änderungen waits: committing needs the `git` command line (reading the status does not). */
export const NO_GIT_FOR_COMMIT = "Git ist nicht installiert.";
