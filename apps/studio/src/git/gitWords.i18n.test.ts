import i18next from "i18next";
import { afterEach, describe, expect, it } from "vitest";
import { commitReportOf, gitStatusOf, initReportOf } from "./gitTesting";
import { changedFilesText, changesWord, commitDoneText, initDoneText, isNotInstalled, noGitForCommit, notInstalledText, statusText, statusTooltip } from "./gitWords";

afterEach(() => i18next.changeLanguage("de"));

describe("gitWords in English", () => {
  it("counts_changes_and_files_in_English", async () => {
    await i18next.changeLanguage("en");
    expect(changesWord(0)).toBe("no changes");
    expect(changesWord(1)).toBe("1 change");
    expect(changesWord(3)).toBe("3 changes");
    expect(statusText(gitStatusOf({ changes: 2 }))).toBe("main · 2 changes");
    expect(statusText(gitStatusOf({ head: null }))).toBe("main · no commit yet");
    expect(statusTooltip(gitStatusOf({ changes: 1 }))).toBe("Git: 1 changed file since the last commit “Ausgangsstand”. Click to show it in Technical › Changes.");
    expect(changedFilesText(18449, true)).toBe("18,449 files for the first commit");
    expect(changedFilesText(1)).toBe("1 changed file since the last commit");
  });

  it("words_the_messages_of_init_and_commit_in_English", async () => {
    await i18next.changeLanguage("en");
    expect(initDoneText(initReportOf({ studio_identity: true }))).toBe(
      "Git set up: 18,449 files in the first commit “Initial state”. The author is “Gen4 Studio” because Git does not know your name yet.",
    );
    expect(commitDoneText(commitReportOf({ files: 1 }))).toBe("Snapshot saved: 9a8b7c6 “Stand 2026-10-31 14:05” (1 file).");
    expect(noGitForCommit()).toBe("Git is not installed.");
  });

  it("recognizes_the_kernel_sentence_in_either_language", async () => {
    // The kernel says `NOT_INSTALLED` in German: the dialog and the wizard must still tell „only the templates“ while the app speaks English.
    const german = notInstalledText();
    await i18next.changeLanguage("en");
    expect(isNotInstalled(german)).toBe(true);
    expect(isNotInstalled(notInstalledText())).toBe(true);
    expect(isNotInstalled("fatal: not a git repository")).toBe(false);
    expect(isNotInstalled(null)).toBe(false);
  });
});
