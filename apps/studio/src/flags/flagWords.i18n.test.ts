// @vitest-environment jsdom
import i18next from "i18next";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { groupRefsByHeader } from "./flagFilter";
import { FLAGS, flagEntry, flagList, flagRef } from "./flagTesting";
import { accessWord, areaWord, liveCellTip, namingNeedsTextProject, notATextProject, outsideTheSave, pickerItems, usageText } from "./flagWords";
import { flagsSubtitle } from "./FlagsSurface";

describe("flag words in English", () => {
  beforeEach(async () => {
    await i18next.changeLanguage("en");
  });
  afterEach(async () => {
    await i18next.changeLanguage("de");
  });

  it("says usage, areas and access in English", () => {
    expect(usageText({ read: 1, write: 3, unknown: 0, sources: 2 })).toBe("set 3 · checked 1");
    expect(usageText({ read: 1, write: 3, unknown: 2, sources: 2 })).toBe("set 3 · checked 1 · unclear 2");
    expect(usageText({ read: 0, write: 0, unknown: 0, sources: 0 })).toBe("unused");
    expect(areaWord("map_local")).toBe("Map-local");
    expect(accessWord("read_write")).toBe("sets and checks");
    expect(liveCellTip("var", 7)).toBe("Live 7 · change it in the detail view");
    expect(outsideTheSave()).toBe("Lies outside the save game.");
  });

  it("keeps the kernel's German sentence apart from the English one", () => {
    expect(notATextProject()).toContain("No text project");
    expect(namingNeedsTextProject()).toBe(`${notATextProject()} (Technical › Text project)`);
  });

  it("lists None first and finds it by either language's word", () => {
    expect(pickerItems(FLAGS, "").map((i) => i.label)[0]).toBe("None (0)");
    expect(pickerItems(FLAGS, "none").map((i) => i.id)).toEqual([0]);
    expect(pickerItems(FLAGS, "kein").map((i) => i.id)).toEqual([0]);
  });

  it("groups references under English header names", () => {
    const groups = groupRefsByHeader([flagRef({ header: null, header_name: null }), flagRef({ header: 7, header_name: null })]);
    expect(groups.map((g) => g.name)).toEqual(["No header", "Header 0007"]);
  });

  it("counts flags and variables with the English plural and thousands separator", () => {
    const many = flagList("flag", Array.from({ length: 4097 }, (_, i) => flagEntry(i)), { pool_free: 135 });
    expect(flagsSubtitle(many)).toBe("4,097 flags · 135 free");
    expect(flagsSubtitle(flagList("var", [flagEntry(0x4030)], { pool_free: 1 }))).toBe("1 variable · 1 free");
    expect(flagsSubtitle(null)).toBe("Reading…");
  });
});
