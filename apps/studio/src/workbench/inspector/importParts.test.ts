import { describe, expect, it } from "vitest";
import { PART_TEXTS, isForeignWhole, partText, partView, sourceMapItems } from "./importParts";

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

  it("says_a_broken_model_or_terrain_inside_a_whole_map_file_with_the_way_out_of_the_map_file", () => {
    const whole = (code: string, message: string, kind: "error" | "warning" = "error") => ({
      part: "whole" as const,
      file: "/x/m.bin",
      size: 9000,
      state: { kind, code, args: ["2"], message },
      notes: [],
    });
    expect(partView(whole("bdhc.magic", "terrain: terrain does not start with BDHC")).message).toBe(
      "Das Terrain in der Kartendatei ist beschädigt (terrain does not start with BDHC). Wähle eine andere Kartendatei.",
    );
    expect(partView(whole("model.count", "model: the model block holds 2 models; the game needs exactly 1")).message).toBe(
      "Das Modell in der Kartendatei ist ungültig (the model block holds 2 models; the game needs exactly 1). Wähle eine andere Kartendatei.",
    );
    // The same codes on their own part keep their own way out.
    const terrain = { ...whole("bdhc.magic", "terrain does not start with BDHC"), part: "bdhc" as const };
    expect(partView(terrain).message).toBe("Die Datei beginnt nicht mit „BDHC“ und ist kein Terrain. Wähle eine .bdhc-Datei.");
    // A whole map file's own errors and warnings say their sentence.
    expect(partView(whole("whole.bgs-dropped", "background sound (6 bytes) is dropped", "warning")).message).toContain("Der Hintergrundklang (2 Byte) fällt weg");
  });

  it("covers_every_part_code_of_the_kernel_and_says_a_broken_terrain_structure", () => {
    // `PartCode::ALL` has 39 codes; the Rust test holds the order against it.
    const codes = Object.keys(PART_TEXTS);
    expect(codes).toHaveLength(39);
    expect(codes).toContain("bdhc.structure");
    expect(said("bdhc.structure", ["plate 0 names point 99, past the 2 points"])).toBe(
      "Das Terrain ist beschädigt (plate 0 names point 99, past the 2 points). Wähle eine unbeschädigte .bdhc-Datei.",
    );
  });

  it("says_the_findings_of_a_converted_model", () => {
    expect(said("model.out-of-range", ["640", "0", "0", "-512", "511"])).toBe(
      "Eine Ecke liegt bei (640, 0, 0), außerhalb von -512 bis 511 Einheiten. Verkleinere das Modell oder wähle eine kleinere Einheit.",
    );
    expect(said("model.faces-dropped", ["1200"])).toBe("1.200 Flächen fallen weg: nach dem Runden auf ganze Einheiten haben sie keine Fläche mehr.");
    expect(said("model.material-untextured", [""])).toBe(
      "Das Material „“ hat keine Textur, und das Gebiets-Paket hat keine Textur dieses Namens. Gib dem Material eine PNG-Textur.",
    );
    expect(said("model.source-unreadable", ["OBJ", "face parse error"])).toBe(
      "Die Datei ist kein lesbares OBJ-Modell (face parse error). Exportiere das Modell neu als OBJ oder glTF.",
    );
    expect(said("model.file-outside", ["../x.mtl"])).toBe(
      "Das Modell verweist auf ../x.mtl außerhalb seines Ordners. Lege Material und Texturen in den Ordner des Modells.",
    );
    expect(said("model.file-missing", ["fehlt.mtl"])).toBe(
      "Die Datei fehlt.mtl, auf die das Modell verweist, fehlt. Lege sie neben das Modell oder exportiere es neu.",
    );
    expect(said("texture.unreadable", ["g4_imp_cube.png", "invalid signature"])).toBe(
      "Das Bild g4_imp_cube.png lässt sich nicht lesen (invalid signature). Wähle ein PNG.",
    );
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

  it("listet_jede_Karte_die_Zielkarte_gesperrt", () => {
    const items = sourceMapItems(3, 1);
    expect(items).toHaveLength(3);
    expect(items[1].disabled).toBe(true);
    expect(items[0].disabled).toBe(false);
    expect(items[0].label).toBe("Karte 0000");
    expect(items[2].tech).toBe("2");
  });
});
