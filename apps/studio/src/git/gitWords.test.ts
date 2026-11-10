import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import enGit from "../i18n/locales/en/git.json";
import { commitReportOf, gitStatusOf, noRepo } from "./gitTesting";
import { changedFilesText, changesWord, commitDoneText, gitFoundText, notInstalledText, statusText, statusTooltip } from "./gitWords";

describe("gitWords", () => {
  it("changes_have_none_singular_and_plural", () => {
    expect(changesWord(0)).toBe("keine Änderung");
    expect(changesWord(1)).toBe("1 Änderung");
    expect(changesWord(3)).toBe("3 Änderungen");
    expect(changesWord(1204)).toBe("1204 Änderungen");
  });

  it("status_text_covers_unborn_detached_and_no_repo", () => {
    expect(statusText(gitStatusOf({ changes: 3 }))).toBe("main · 3 Änderungen");
    expect(statusText(gitStatusOf())).toBe("main · keine Änderung");
    // An enclosing repository reads like an own one.
    expect(statusText(gitStatusOf({ repo: { kind: "enclosing", workdir: "/home/u" }, branch: "dotfiles", changes: 1 }))).toBe("dotfiles · 1 Änderung");
    // Without a commit the branch to be is named, the changes are not counted.
    expect(statusText(gitStatusOf({ head: null, changes: 7 }))).toBe("main · noch kein Commit");
    // Detached: the short id instead of the branch.
    expect(statusText(gitStatusOf({ branch: null, detached: true, changes: 2 }))).toBe("4c1d2e3 (losgelöst) · 2 Änderungen");
    expect(statusText(noRepo())).toBeNull();
  });

  it("git_found_names_the_version_without_git_version", () => {
    expect(gitFoundText("git version 2.54.0 (Apple Git-157)")).toBe("Git gefunden: 2.54.0 (Apple Git-157)");
    expect(gitFoundText("git version 2.47.1.windows.1")).toBe("Git gefunden: 2.47.1.windows.1");
    // Anything else stays as git said it.
    expect(gitFoundText("2.47.1")).toBe("Git gefunden: 2.47.1");
  });

  it("the_tooltip_names_the_count_and_the_last_commit", () => {
    expect(statusTooltip(gitStatusOf({ changes: 3 }))).toBe("Git: 3 geänderte Dateien seit dem letzten Commit „Ausgangsstand“. Klick zeigt sie in Technik › Änderungen.");
    expect(statusTooltip(gitStatusOf({ changes: 1 }))).toBe("Git: 1 geänderte Datei seit dem letzten Commit „Ausgangsstand“. Klick zeigt sie in Technik › Änderungen.");
    expect(statusTooltip(gitStatusOf({ head: null }))).toBe("Git: noch kein Commit.");
  });

  it("the_tooltip_says_none_as_the_label_does", () => {
    // The label says „keine Änderung“: the tooltip does not count „0 geänderte Dateien“.
    expect(statusTooltip(gitStatusOf({ changes: 0 }))).toBe("Git: keine geänderte Datei seit dem letzten Commit „Ausgangsstand“. Klick öffnet Technik › Änderungen.");
    expect(statusTooltip(gitStatusOf({ changes: 0, detached: true, branch: null }))).toBe(
      "Git: keine geänderte Datei seit dem letzten Commit „Ausgangsstand“. Klick öffnet Technik › Änderungen.",
    );
  });

  it("not_installed_mirrors_the_kernel_sentence", () => {
    // The dialog and the wizard tell „only the templates“ from a failed commit by this sentence: a change in the kernel must show here.
    // The kernel sentence lives in its catalogs as `git.not_installed`; German against the German shown, English against the English one.
    const sentence = (lang: "de" | "en") => {
      const catalog = readFileSync(`../../crates/g4-kernel/locales/${lang}.yml`, "utf8");
      const git = /^git:\n((?:(?: .*)?\n)*)/m.exec(catalog)?.[1] ?? "";
      return /^ {2}not_installed: "((?:[^"\\]|\\.)*)"$/m.exec(git)?.[1];
    };
    expect(sentence("de")).toBe(notInstalledText());
    expect(sentence("en")).toBe(enGit.gitWords.notInstalled);
  });

  it("changed_files_have_none_singular_and_plural", () => {
    expect(changedFilesText(0)).toBe("Keine Änderung seit dem letzten Commit");
    expect(changedFilesText(1)).toBe("1 geänderte Datei seit dem letzten Commit");
    expect(changedFilesText(3)).toBe("3 geänderte Dateien seit dem letzten Commit");
  });

  it("changed_files_without_a_commit_are_those_of_the_first_commit", () => {
    expect(changedFilesText(1, true)).toBe("1\u00a0Datei für den ersten Commit");
    expect(changedFilesText(18449, true)).toBe("18.449\u00a0Dateien für den ersten Commit");
    // Nothing to take: the sentence stays.
    expect(changedFilesText(0, true)).toBe("Keine Änderung seit dem letzten Commit");
  });

  it("commit_done_text_names_short_subject_and_count", () => {
    expect(commitDoneText(commitReportOf())).toBe("Stand gesichert: 9a8b7c6 „Stand 2026-10-31 14:05“ (3\u00a0Dateien).");
    expect(commitDoneText(commitReportOf({ files: 1, subject: "Veröffentlicht v0.1.0" }))).toBe("Stand gesichert: 9a8b7c6 „Veröffentlicht v0.1.0“ (1\u00a0Datei).");
    expect(commitDoneText(commitReportOf({ files: 18449 }))).toBe("Stand gesichert: 9a8b7c6 „Stand 2026-10-31 14:05“ (18.449\u00a0Dateien).");
    // Without a name in Git the commit is Gen4 Studio's, and the message says why.
    expect(commitDoneText(commitReportOf({ studio_identity: true }))).toBe(
      "Stand gesichert: 9a8b7c6 „Stand 2026-10-31 14:05“ (3\u00a0Dateien). Autor ist „Gen4 Studio“, weil Git deinen Namen noch nicht kennt.",
    );
  });
});
