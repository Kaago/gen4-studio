import type { CommitReport, GitStatus, InitReport, TemplatesRewritten } from "../api";
import { matchesAnyLanguage, t } from "../i18n";

/** The changed paths since the last commit: „keine Änderung“, „1 Änderung“, „{n} Änderungen“. */
export function changesWord(n: number): string {
  if (n === 0) return t("git:gitWords.noChange");
  return t("git:gitWords.changesCount", { count: n });
}

/**
 * What the status line says about Git: „main · 3 Änderungen“; without a commit „main · noch kein Commit“ (the branch to be), with a detached
 * `HEAD` „4c1d2e3 (losgelöst) · 2 Änderungen“. Null without a repository (the status line shows nothing then).
 */
export function statusText(s: GitStatus): string | null {
  if (s.repo.kind === "none") return null;
  if (s.head === null) return `${s.branch ?? "HEAD"} · ${t("git:gitWords.statusText.noCommit")}`;
  const where = s.detached || s.branch === null ? t("git:gitWords.statusText.detached", { id: s.head.short }) : s.branch;
  return `${where} · ${changesWord(s.changes)}`;
}

/**
 * The tooltip of the status line's item: the count and the last commit, and where a click leads; without a commit only that. None changed
 * reads „keine geänderte Datei“, as the label says „keine Änderung“.
 */
export function statusTooltip(s: GitStatus): string {
  if (s.head === null) return t("git:gitWords.statusTooltip.noCommit");
  const summary = s.head.summary;
  if (s.changes === 0) return t("git:gitWords.statusTooltip.none", { summary });
  return t("git:gitWords.statusTooltip.changed", { count: s.changes, summary });
}

/** The checklist's row for a found `git`: its version without the „git version “ that `git --version` puts first. */
export function gitFoundText(version: string): string {
  return t("git:gitWords.found", { version: version.replace(/^git version /, "") });
}

/** The message after a first commit: how many files it holds, and when Git knew no name, why the author is Gen4 Studio. */
export function initDoneText(report: InitReport): string {
  const done = t("git:gitWords.initDone", { count: report.files });
  return report.studio_identity ? `${done} ${t("git:gitWords.studioIdentity")}` : done;
}

/** The kernel's reason when no `git` was found (`git.not_installed`) in the language that is shown: only the templates were written. */
export const notInstalledText = (): string => t("git:gitWords.notInstalled");

/**
 * Whether `reason` is the kernel's `git.not_installed`. The kernel words it in the language it was set to; a sentence of either catalog
 * counts, so the check holds when it speaks another language than the app shows.
 */
export function isNotInstalled(reason: string | null): boolean {
  return reason !== null && matchesAnyLanguage("git:gitWords.notInstalled", reason);
}

/** The message when no `git` is installed: only the templates were written. */
export const templatesOnlyText = (): string => t("git:gitWords.templatesOnly");

/**
 * After a set-up or a take-back: what became of the Git templates, as a sentence with a leading space („ Die .gitignore von Gen4 Studio
 * versioniert jetzt das Textprojekt.“), the kernel's warning when they could not be written, empty when nothing changed or the project
 * never set up Git.
 */
export function templatesNote(report: TemplatesRewritten): string {
  if (report.templates_warning !== null) return ` ${report.templates_warning}`;
  const rewritten = report.templates;
  if (rewritten === null || !rewritten.files.some((f) => f.path === ".gitignore" && f.action !== "unchanged")) return "";
  return ` ${rewritten.layout === "text_project" ? t("git:gitWords.templatesNote.textProject") : t("git:gitWords.templatesNote.dspre")}`;
}

/**
 * The checklist's row of „Stand sichern“: the changed paths since the last commit, as the status counts them; in a repository without a
 * commit (`first`) the files the first commit takes („18.449 Dateien für den ersten Commit“).
 */
export function changedFilesText(n: number, first = false): string {
  if (n === 0) return t("git:gitWords.changedFiles.none");
  if (first) return t("git:gitWords.changedFiles.first", { count: n });
  return t("git:gitWords.changedFiles.since", { count: n });
}

/** The message after „Stand sichern“: the new commit's short id, its subject and how many files went in; when Git knew no name, why the author is Gen4 Studio. */
export function commitDoneText(report: CommitReport): string {
  const files = t("git:gitWords.commitDone.files", { count: report.files });
  const where = report.head === null ? "" : `${report.head.short} `;
  const done = t("git:gitWords.commitDone.text", { where, subject: report.subject, files });
  return report.studio_identity ? `${done} ${t("git:gitWords.studioIdentity")}` : done;
}

/** Why „Stand sichern…“ in Technik › Änderungen waits: committing needs the `git` command line (reading the status does not). */
export const noGitForCommit = (): string => t("git:gitWords.noGitForCommit");
