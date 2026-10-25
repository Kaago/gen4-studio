import { describe, expect, it } from "vitest";
import { PART_TEXTS, isForeignWhole, partText, partView } from "./importParts";

const said = (code: string, args: string[]) => partText({ code, args, message: "the kernel's sentence" });

describe("importParts", () => {
  it("says_every_code_of_the_part_checks_in_a_german_sentence_with_its_arguments", () => {
    for (const code of Object.keys(PART_TEXTS)) {
      const text = said(code, ["70000", "61440", "a, b", "4"]);
      expect(text, code).not.toBe("the kernel's sentence");
      expect(text, code).not.toMatch(/[{}]/);
      expect(text, code).toMatch(/[.!]$/);
    }
  });

  it("words_the_numbers_in_german_digits_and_names_the_way_out", () => {
    expect(said("model.too-big", ["65000", "61440"])).toBe("Das Modell ist 65.000 Byte groß, erlaubt sind 61.440. Wähle ein kleineres Modell.");
    expect(said("permissions.size", ["2046", "2048"])).toBe("Die Kollision ist 2.046 Byte groß, erwartet sind genau 2.048. Wähle eine vollständige .per-Datei.");
    expect(said("bgs.signature", ["0x0000", "0x1234"])).toBe("Der Hintergrundklang hat die Kennung 0x0000 statt 0x1234. Wähle eine .bgs-Datei aus HeartGold oder SoulSilver.");
    expect(said("whole.foreign", ["Hgss"])).toBe("Kartendatei von HeartGold/SoulSilver: ihre Gebäude bleiben weg, außer du schaltest „Gebäude übernehmen“ ein.");
    // The reader's reason stays the kernel's, only as a detail in brackets.
    expect(said("model.invalid", ["bad magic"])).toBe("Die Datei ist kein gültiges NSBMD-Modell (bad magic). Wähle ein Kartenmodell im NSBMD-Format.");
  });

  it("keeps_the_kernel_sentence_for_a_code_it_does_not_know", () => {
    expect(said("model.someday", [])).toBe("the kernel's sentence");
  });

  it("shows_the_german_sentence_of_an_error_or_warning_and_knows_a_foreign_whole_map_by_its_code", () => {
    const report = { part: "permissions" as const, file: null, size: 10, state: { kind: "error" as const, code: "permissions.size", args: ["10", "2048"], message: "permissions are 10 bytes" }, notes: [] };
    expect(partView(report)).toEqual({ state: "error", message: "Die Kollision ist 10 Byte groß, erwartet sind genau 2.048. Wähle eine vollständige .per-Datei." });
    const whole = { part: "whole" as const, file: null, size: 9000, state: { kind: "ok" as const }, notes: [{ code: "whole.foreign", args: ["Hgss"], message: "HeartGold/SoulSilver map file" }] };
    expect(isForeignWhole(whole)).toBe(true);
    expect(isForeignWhole({ ...whole, notes: [] })).toBe(false);
  });
});
