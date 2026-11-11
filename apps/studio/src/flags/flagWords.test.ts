import { describe, expect, it } from "vitest";
import { kernelSentence } from "../testing/kernelCatalog";
import { FLAGS, flagEntry, flagList } from "./flagTesting";
import { accessWord, areaWord, displayName, hex4, kindWord, liveCellText, liveCellTip, notATextProject, numberText, pickerItems, usageShort, usageText } from "./flagWords";

describe("flag words", () => {
  it("numbers_show_hex_and_decimal", () => {
    expect(hex4(0x94)).toBe("0x0094");
    expect(hex4(0x4030)).toBe("0x4030");
    expect(hex4(0xab)).toBe("0x00AB");
    expect(hex4(0)).toBe("0x0000");
    expect(numberText(148)).toBe("0x0094 · 148");
    expect(numberText(16432)).toBe("0x4030 · 16432");
  });

  it("display_name_prefers_the_own_id", () => {
    expect(displayName({ name: "FLAG_UNUSED_0x0094", own_id: "FLAG_HAT_MEDAILLE" }, "flag", 0x94)).toBe("FLAG_HAT_MEDAILLE");
    expect(displayName({ name: "FLAG_UNUSED_0x0094", own_id: null }, "flag", 0x94)).toBe("FLAG_UNUSED_0x0094");
    expect(displayName({ name: null, own_id: null }, "flag", 0x94)).toBe("Flag 0x0094");
    expect(displayName({ name: null, own_id: null }, "var", 0x4030)).toBe("Variable 0x4030");
    expect(kindWord("flag")).toBe("Flag");
    expect(kindWord("var")).toBe("Variable");
  });

  it("live_cells_name_flags_as_1_or_0_and_variables_in_decimal", () => {
    expect([liveCellText("flag", 1), liveCellText("flag", 0), liveCellText("var", 1234), liveCellText("var", null)]).toEqual(["1", "0", "1234", "–"]);
    expect([liveCellTip("flag", 1), liveCellTip("flag", 0), liveCellTip("var", 7)]).toEqual(["Live: gesetzt · Klick löscht", "Live: 0 · Klick setzt", "Live 7 · ändern im Detail"]);
  });

  it("usage_text_names_set_checked_and_unclear", () => {
    expect(usageText({ read: 1, write: 3, unknown: 0, sources: 2 })).toBe("gesetzt 3 · geprüft 1");
    expect(usageText({ read: 0, write: 0, unknown: 2, sources: 1 })).toBe("gesetzt 0 · geprüft 0 · unklar 2");
    expect(usageText({ read: 0, write: 0, unknown: 0, sources: 0 })).toBe("unbenutzt");
    expect(accessWord("write")).toBe("setzt");
    expect(accessWord("read")).toBe("prüft");
    expect(accessWord("read_write")).toBe("setzt und prüft");
    expect(accessWord("unknown")).toBe("unklar");
    expect(
      (["map_local", "general", "hidden_items", "trainer", "system", "daily", "temp", "object_gfx", "script_local", "outside"] as const).map(areaWord),
    ).toEqual(["kartenlokal", "allgemein", "versteckte Items", "Trainer", "System", "Tagesflags", "temporär", "Objektgrafik", "Script-lokal", "außerhalb"]);
  });

  it("usage_short_is_the_picker_form_of_the_count", () => {
    expect(usageShort({ read: 338, write: 193, unknown: 0, sources: 9 })).toBe("193 · 338");
    expect(usageShort({ read: 338, write: 193, unknown: 21, sources: 9 })).toBe("193 · 338 · 21");
    expect(usageShort({ read: 0, write: 0, unknown: 2, sources: 1 })).toBe("0 · 0 · 2");
    expect(usageShort({ read: 0, write: 0, unknown: 0, sources: 0 })).toBe("unbenutzt");
  });

  it("picker_items_start_with_none_and_match_name_alias_and_number", () => {
    const all = pickerItems(FLAGS, "");
    // 0 is no flag, and it stays a valid choice: the first row.
    expect(all[0]).toMatchObject({ id: 0, label: "Keins (0)" });
    expect(all.map((i) => i.id)).toEqual([0, 0x20, 0x94, 0x96, 0x173, 0x550]);
    // The own name in front, the pret name as the second line; without an own name the area.
    // The own name in bold.
    expect(all[2]).toMatchObject({ label: "FLAG_HAT_MEDAILLE", strong: true, description: "FLAG_UNUSED_0x0094", tech: "0x0094", count: "3 · 1", countTitle: "gesetzt 3 · geprüft 1", area: "geschichten" });
    expect(all[3]).toMatchObject({ label: "FLAG_UNUSED_0x0096", strong: false, description: "allgemein", count: "0 · 0 · 2", countTitle: "gesetzt 0 · geprüft 0 · unklar 2" });
    expect(all[4]).toMatchObject({ label: "Flag 0x0173", tech: "0x0173", count: "unbenutzt", countTitle: "unbenutzt" });
    const ids = (query: string) => pickerItems(FLAGS, query).map((i) => i.id);
    // Own name, pret name, alias, hex and decimal number; „Keins (0)“ only where the query finds it, never for a number.
    expect(ids("medaille")).toEqual([0x94]);
    expect(ids("FLAG_UNUSED_0x0094")).toEqual([0x94]);
    expect(ids("FLAG_ALT")).toEqual([0x96]);
    expect(ids("0x0094")).toEqual([0x94]);
    expect(ids("0x0173")).toEqual([0x173]);
    expect(ids("0x173")).toEqual([0x173]);
    expect(ids("148")).toEqual([0x94]);
    expect(ids("nichts davon")).toEqual([]);
    expect(ids("keins")).toEqual([0]);
    expect(ids("kein")).toEqual([0]);
    expect(ids("none")).toEqual([0]);
    expect(ids("0")[0]).toBe(0);
    // A list with a number 0 of its own does not list it twice.
    expect(pickerItems(flagList("var", [flagEntry(0), flagEntry(0x4030)]), "").map((i) => i.id)).toEqual([0, 0x4030]);
  });

  it("not_a_text_project_mirrors_the_kernel_sentence", () => {
    // The lock of naming in the DSPRE layout says what `name_flag` would answer: a change in the kernel must show here.
    // The kernel sentence lives in its German catalog as `textproject.not_a_text_project`.
    expect(kernelSentence("de", "textproject", "not_a_text_project")).toBe(notATextProject());
  });
});
