// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { TextProjectStatus } from "../api";
import { NO_DP, dismissHint, hintOf, shouldHint } from "./textProjectHint";

const DSPRE: TextProjectStatus = { state: "dspre", adoptable: true, reason: null };
const NO_BASE: TextProjectStatus = { state: "dspre", adoptable: false, reason: "Diesem Projekt ist keine Basis-ROM zugeordnet." };

describe("textProjectHint", () => {
  beforeEach(() => localStorage.clear());
  afterEach(() => vi.restoreAllMocks());

  it("hints_once_per_root_until_dismissed", () => {
    expect(shouldHint("/a", DSPRE)).toBe(true);
    // Not dismissed: the next opening of the folder hints again.
    expect(shouldHint("/a", DSPRE)).toBe(true);
    dismissHint("/a");
    expect(shouldHint("/a", DSPRE)).toBe(false);
    expect(localStorage.getItem("g4.textProjectHint./a")).not.toBeNull();
    // Another folder is asked on its own.
    expect(shouldHint("/b", DSPRE)).toBe(true);
    // A folder without its base ROM is hinted too: the hint says what it needs.
    expect(shouldHint("/b", NO_BASE)).toBe(true);
    expect(hintOf(DSPRE)).toEqual({
      text: "Dieses Projekt lässt sich als Textprojekt einrichten: eine lesbare Datei je Eintrag, bereit für Git. DSPRE öffnet den Ordner weiter.",
      setUp: true,
    });
    expect(hintOf(NO_BASE)).toEqual({
      text: "Dieses Projekt lässt sich als Textprojekt einrichten: eine lesbare Datei je Eintrag, bereit für Git. Dafür braucht es die Basis-ROM.",
      setUp: false,
    });
  });

  it("no_hint_for_a_text_project_or_dp", () => {
    expect(shouldHint("/a", { state: "current", schema: 1, last_sync_at: null })).toBe(false);
    expect(shouldHint("/a", { state: "older", schema: 0 })).toBe(false);
    expect(shouldHint("/a", { state: "newer", schema: 2 })).toBe(false);
    expect(shouldHint("/a", { state: "broken", reason: "g4-project.json:1:1: kaputt" })).toBe(false);
    expect(shouldHint("/dp", { state: "dspre", adoptable: false, reason: NO_DP })).toBe(false);
  });

  it("storage_errors_do_not_break_the_hint", () => {
    vi.spyOn(Storage.prototype, "getItem").mockImplementation(() => {
      throw new Error("SecurityError");
    });
    vi.spyOn(Storage.prototype, "setItem").mockImplementation(() => {
      throw new Error("QuotaExceededError");
    });
    // Nothing remembered can be read: the hint shows; dismissing it cannot be kept, and says nothing.
    expect(shouldHint("/a", DSPRE)).toBe(true);
    expect(() => dismissHint("/a")).not.toThrow();
  });
});
