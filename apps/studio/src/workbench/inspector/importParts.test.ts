import { describe, expect, it } from "vitest";
import type { ImportPreview, PartReport } from "../../api";
import { PART_TEXTS, isForeignWhole, partText, partView, previewLines, sourceMapItems } from "./importParts";

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
    // `PartCode::ALL` has 51 codes; the Rust test holds the order against it.
    const codes = Object.keys(PART_TEXTS);
    expect(codes).toHaveLength(51);
    expect(codes).toContain("bdhc.structure");
    expect(said("bdhc.structure", ["plate 0 names point 99, past the 2 points"])).toBe(
      "Das Terrain ist beschädigt (plate 0 names point 99, past the 2 points). Wähle eine unbeschädigte .bdhc-Datei.",
    );
  });

  it("says_the_findings_of_a_converted_model", () => {
    expect(said("model.uv-range", ["kachel", "s", "4096"])).toBe(
      "Die Textur kachel läuft auf einer Fläche über 4.096 Texel in s; das Spiel trägt höchstens 2.047. Teile die Fläche oder wiederhole die Textur seltener.",
    );
    expect(said("model.texture-count", ["300", "255"])).toBe("Das Modell nutzt 300 Texturen, ein Kartenmodell trägt höchstens 255. Fasse Materialien zusammen.");
    expect(said("model.converted", ["OBJ", "1200", "1100", "100", "40", "12"])).toBe(
      "OBJ-Modell umgewandelt: 1.200 Flächen, 1.100 Vierecke, 100 Dreiecke, 40 Streifen.",
    );
    expect(said("part.no-area", ["2"])).toBe(
      "Kein Header zeigt die Karte 2; ohne Gebiet gibt es kein Texturpaket und kein Licht. Setze die Karte zuerst in eine Matrix mit Header.",
    );
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

  it("words_the_parts_a_model_leaves_out", () => {
    expect(said("model.parts-ignored", ["lines,animations"])).toBe("Nicht übernommen: Linien, Animationen.");
    expect(said("model.parts-ignored", ["texture-options"])).toBe("Nicht übernommen: Texturoptionen.");
    expect(said("model.parts-ignored", ["lines,points,animations,skins,cameras,lights,vertex-colours,texture-options"])).toBe(
      "Nicht übernommen: Linien, Punkte, Animationen, Skelette, Kameras, Lichter, Eckenfarben, Texturoptionen.",
    );
  });

  it("says_the_textures_of_an_import_with_a_german_decimal_comma", () => {
    expect(said("texture.new", ["g4_imp_cube", "16", "16", "16", "7.6"])).toBe("Neue Textur g4_imp_cube: 16 × 16, 16 Farben, mittlerer Farbfehler 7,6.");
    expect(said("texture.new", ["g4_imp_40", "64", "64", "16", "0.0"])).toContain("mittlerer Farbfehler 0,0.");
    expect(said("texture.side", ["kachel", "24", "16"])).toBe(
      "Die Textur kachel ist 24 × 16 Pixel groß; jede Seite muss eine Zweierpotenz von 8 bis 1.024 sein. Skaliere das Bild.",
    );
    expect(said("texture.taken", ["texture", "g4_imp_cube"])).toBe(
      "Der Name g4_imp_cube ist im Gebiets-Paket schon vergeben. Benenne die Datei um; Ersetzen gibt es nicht.",
    );
    expect(said("texture.pack-full", ["entries", "256", "255"])).toBe("Das Gebiets-Paket ist voll (entries: 256, höchstens 255). Nimm weniger oder kleinere Texturen.");
    expect(said("texture.vram-high", ["3", "120000", "110640"])).toBe(
      "Gebiet 3: die Texturen belegen 120.000 B VRAM, mehr als das größte Vanilla-Gebiet (110.640 B). Prüfe die Karte im Spiel.",
    );
  });

  it("words_the_preview_of_a_converted_cube_in_three_lines", () => {
    const preview: ImportPreview = {
      pack: 0,
      area: 0,
      style: "outdoor",
      vram_bytes: 23544,
      vram_limit: 110640,
      textures: [{ name: "g4_imp_cube", file: "/p/g4_imp_cube.png", width: 16, height: 16, colors: 16, new: true, mean_error: 7.6 }],
      model: { format: "obj", stats: { polygons: 6, quads: 6, triangles: 0, dropped: 0 }, strips: 2, single_quads: 0, budget: { used: 1200, max: 61440, state: "calm" } },
      files: ["/p/g4_imp_cube.obj"],
    };
    const cube: PartReport = { part: "model", file: "/p/g4_imp_cube.obj", size: 1200, state: { kind: "ok" }, notes: [], preview };
    expect(previewLines(cube)).toEqual([
      "OBJ · 6 Vierecke · 0 Dreiecke · 2 Streifen",
      "g4_imp_cube · 16 × 16 · 16 Farben · neu",
      "VRAM des Gebiets 23.544 B · Vanilla höchstens 110.640 B",
    ]);
    // A texture of the pack says so, without the colours it does not read.
    const reused = { ...preview, model: { ...preview.model!, format: "gltf" as const }, textures: [{ ...preview.textures[0], file: null, colors: 0, new: false, mean_error: null }] };
    expect(previewLines({ ...cube, preview: reused })).toEqual([
      "glTF · 6 Vierecke · 0 Dreiecke · 2 Streifen",
      "g4_imp_cube · 16 × 16 · im Paket",
      "VRAM des Gebiets 23.544 B · Vanilla höchstens 110.640 B",
    ]);
  });

  it("words_the_preview_of_textures_without_a_model_line_and_nothing_without_a_preview", () => {
    const preview: ImportPreview = {
      pack: 3,
      area: 2,
      style: "indoor",
      vram_bytes: 4096,
      vram_limit: 110640,
      textures: [
        { name: "gras", file: "/p/gras.png", width: 32, height: 32, colors: 4, new: true, mean_error: 2.5 },
        { name: "weg", file: "/p/weg.png", width: 8, height: 16, colors: 16, new: true, mean_error: 0 },
      ],
      model: null,
      files: ["/p/gras.png", "/p/weg.png"],
    };
    const textures: PartReport = { part: "textures", file: null, size: 2048, state: { kind: "ok" }, notes: [], preview };
    expect(previewLines(textures)).toEqual([
      "gras · 32 × 32 · 4 Farben · neu",
      "weg · 8 × 16 · 16 Farben · neu",
      "VRAM des Gebiets 4.096 B · Vanilla höchstens 110.640 B",
    ]);
    expect(previewLines({ part: "permissions", file: null, size: 2048, state: { kind: "ok" }, notes: [] })).toEqual([]);
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
