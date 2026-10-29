import { describe, expect, it } from "vitest";
import { gitStatusOf, noRepo } from "./gitTesting";
import { changesWord, statusText, statusTooltip } from "./gitWords";

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

  it("the_tooltip_names_the_count_and_the_last_commit", () => {
    expect(statusTooltip(gitStatusOf({ changes: 3 }))).toBe("Git: 3 geänderte Dateien seit dem letzten Commit „Ausgangsstand“. Klick zeigt sie in Technik › Änderungen.");
    expect(statusTooltip(gitStatusOf({ changes: 1 }))).toBe("Git: 1 geänderte Datei seit dem letzten Commit „Ausgangsstand“. Klick zeigt sie in Technik › Änderungen.");
    expect(statusTooltip(gitStatusOf({ head: null }))).toBe("Git: noch kein Commit.");
  });
});
