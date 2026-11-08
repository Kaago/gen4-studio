import { describe, expect, it } from "vitest";
import type { BuildStep } from "../api";
import { formatSeconds, patchBuildNote, stepLabel } from "./stepWords";

const step = (id: string, over: Partial<BuildStep> = {}): BuildStep => ({ id, status: "full", ms: 0, done: 0, total: 0, cached: 0, reason: null, ...over });

describe("stepWords", () => {
  it("labels_each_step_kind", () => {
    expect(stepLabel(step("rotom", { status: "incremental", done: 2, total: 412 }), "bauen")).toEqual({ name: "Scripts", result: "2 von 412 neu" });
    expect(stepLabel(step("chatot", { status: "incremental", done: 3, total: 724 }), "bauen")).toEqual({ name: "Texte", result: "3 kodiert" });
    expect(stepLabel(step("pack:maps", { status: "incremental", done: 1, total: 666 }), "bauen")).toEqual({ name: "Karten packen", result: "1 Member eingesetzt" });
    expect(stepLabel(step("pack:maps", { status: "full", done: 666, total: 666 }), "bauen")).toEqual({ name: "Karten packen", result: "666 Member gepackt" });
    const words: [string, string][] = [
      ["textArchives", "Texte"],
      ["scripts", "Scripts"],
      ["eventFiles", "Eventdateien"],
      ["areaData", "Gebietsdaten"],
      ["trainerProperties", "Trainer"],
      ["trainerParty", "Trainer"],
      ["encounters", "Begegnungen"],
      ["personalPokeData", "Pokémon-Daten"],
    ];
    for (const [folder, word] of words) {
      expect(stepLabel(step(`pack:${folder}`), "bauen").name).toBe(`${word} packen`);
      expect(stepLabel(step(`pack:${folder}`), "tuefteln").name).toBe(`${word} packen`);
    }
    // Matrices are world maps in Bauen; a folder without a word is game data there and keeps its name in Tüfteln.
    expect(stepLabel(step("pack:matrices"), "bauen").name).toBe("Weltkarten packen");
    expect(stepLabel(step("pack:matrices"), "tuefteln").name).toBe("Matrizen packen");
    expect(stepLabel(step("pack:pokeHeight"), "bauen").name).toBe("Spieldaten packen");
    expect(stepLabel(step("pack:pokeHeight"), "tuefteln").name).toBe("pokeHeight packen");

    expect(stepLabel(step("blz:arm9", { status: "incremental", total: 1, cached: 1 }), "bauen")).toEqual({ name: "Spielcode (ARM9)", result: "1 aus dem Cache" });
    expect(stepLabel(step("blz:overlays", { status: "incremental", total: 127, cached: 127 }), "bauen")).toEqual({ name: "Overlays", result: "127 aus dem Cache" });
    expect(stepLabel(step("blz:overlays", { status: "full", done: 127, total: 127, cached: 0 }), "bauen").result).toBe("127 komprimiert");
    expect(stepLabel(step("blz:overlays", { status: "incremental", done: 2, total: 127, cached: 125 }), "bauen").result).toBe("125 aus dem Cache, 2 komprimiert");
    expect(stepLabel(step("blz:arm9", { status: "skipped", reason: "not-compressed" }), "bauen").result).toBe("nicht komprimiert");
    expect(stepLabel(step("blz:overlays", { status: "full", reason: "fallback" }), "bauen").result).toBe("ohne Cache komprimiert");
    expect(stepLabel(step("rom", { status: "skipped" }), "bauen")).toEqual({ name: "ROM schreiben", result: "aktuell" });
    expect(stepLabel(step("rom", { status: "full" }), "bauen")).toEqual({ name: "ROM schreiben", result: "geschrieben" });
    expect(stepLabel(step("crc", { status: "full", reason: "no-base-mark" }), "bauen")).toEqual({ name: "Prüfsummen", result: "keine Basis-Marke" });
    expect(stepLabel(step("crc", { status: "full" }), "bauen").result).toBe("aus der Basis übernommen");
    expect(stepLabel(step("crc", { status: "full", reason: "secure-area-changed" }), "bauen").result).toBe("nicht übernommen: Sicherheitsbereich geändert");
    // Any skipped step says so.
    expect(stepLabel(step("pack:maps", { status: "skipped" }), "bauen").result).toBe("übersprungen");
    expect(stepLabel(step("chatot", { status: "skipped" }), "bauen").result).toBe("übersprungen");
    // A step this version does not know (a later plugin step) shows no identifier in Bauen.
    expect(stepLabel(step("plugin:x", { status: "incremental" }), "bauen")).toEqual({ name: "Weiterer Schritt", result: "teilweise neu gebaut" });
    expect(stepLabel(step("plugin:x"), "tuefteln").name).toBe("plugin:x");
  });

  it("names_the_text_project_step_before_the_bake_row", () => {
    // The text project step runs first and „Karten backen“ follows it: Bauen names it instead of „Weiterer Schritt“.
    expect(stepLabel(step("textproject", { reason: "no-state" }), "bauen")).toEqual({ name: "Textprojekt", result: "voll gebaut (erster Build, voll)" });
    expect(stepLabel(step("textproject"), "tuefteln").name).toBe("Textprojekt");
    expect(stepLabel(step("world"), "bauen").name).toBe("Karten backen");
  });

  it("words_a_removed_output_of_the_rom_step_as_another_target_or_a_deleted_rom", () => {
    // The pack steps keep „Datei entfernt“.
    expect(stepLabel(step("pack:maps", { done: 665, total: 665, reason: "removed" }), "tuefteln").result).toBe("665 Member gepackt (Datei entfernt, voll gebaut)");
    expect(stepLabel(step("rom", { reason: "removed" }), "bauen")).toEqual({ name: "ROM schreiben", result: "geschrieben (anderes Ziel oder ROM gelöscht, voll gebaut)" });
    expect(stepLabel(step("rom", { reason: "removed" }), "tuefteln").result).toBe("geschrieben (anderes Ziel oder ROM gelöscht, voll gebaut)");
    // The other reasons of the rom step keep their words.
    expect(stepLabel(step("rom", { reason: "output-changed" }), "bauen").result).toBe("geschrieben (Ausgabe geändert, voll gebaut)");
  });

  it("names_the_patch_build_with_its_rom_relative_to_the_project_when_inside_it", () => {
    expect(patchBuildNote("/p/build/4-2-abn-patch-IPKD.nds", "/p")).toBe("Build von „Veröffentlichen“ nach build/4-2-abn-patch-IPKD.nds");
    expect(patchBuildNote("/p/build/x.nds", "/p/")).toBe("Build von „Veröffentlichen“ nach build/x.nds");
    // A sibling folder that only shares the prefix, and a path elsewhere, stay whole.
    expect(patchBuildNote("/pp/build/x.nds", "/p")).toBe("Build von „Veröffentlichen“ nach /pp/build/x.nds");
    expect(patchBuildNote("/roms/x.nds", "/p")).toBe("Build von „Veröffentlichen“ nach /roms/x.nds");
  });

  it("formats_seconds_with_a_comma", () => {
    expect(formatSeconds(330)).toBe("0,33 s");
    expect(formatSeconds(50)).toBe("0,05 s");
    expect(formatSeconds(10)).toBe("0,01 s");
    expect(formatSeconds(9)).toBe("< 0,01 s");
    expect(formatSeconds(0)).toBe("< 0,01 s");
    expect(formatSeconds(4712)).toBe("4,71 s");
    expect(formatSeconds(12_345)).toBe("12,35 s");
  });

  it("names_the_full_reason", () => {
    expect(stepLabel(step("pack:maps", { done: 666, total: 666, reason: "no-state" }), "bauen").result).toBe("666 Member gepackt (erster Build, voll)");
    expect(stepLabel(step("pack:maps", { done: 666, total: 666, reason: "build.cache-stale" }), "bauen").result).toBe("666 Member gepackt (neues Werkzeug, voll gebaut)");
    expect(stepLabel(step("pack:maps", { done: 666, total: 666, reason: "output-changed" }), "bauen").result).toBe("666 Member gepackt (Ausgabe geändert, voll gebaut)");
    expect(stepLabel(step("pack:maps", { done: 665, total: 665, reason: "removed" }), "bauen").result).toBe("665 Member gepackt (Datei entfernt, voll gebaut)");
    expect(stepLabel(step("rom", { reason: "no-state" }), "bauen").result).toBe("geschrieben (erster Build, voll)");
    // An unknown reason names no identifier in Bauen; Tüfteln shows it.
    expect(stepLabel(step("chatot", { done: 3, reason: "x-new" }), "bauen").result).toBe("3 kodiert (voll gebaut)");
    expect(stepLabel(step("chatot", { done: 3, reason: "x-new" }), "tuefteln").result).toBe("3 kodiert (voll gebaut: x-new)");
    // The reasons of blz and crc are their result, not an addition.
    expect(stepLabel(step("crc", { reason: "no-base-mark" }), "bauen").result).toBe("keine Basis-Marke");
    expect(stepLabel(step("pack:maps", { status: "incremental", done: 1, total: 666 }), "bauen").result).toBe("1 Member eingesetzt");
  });

  it("after_a_build_without_cache_says_so_instead_of_first_build", () => {
    expect(stepLabel(step("pack:maps", { done: 666, total: 666, reason: "no-state" }), "bauen", true).result).toBe("666 Member gepackt (ohne Cache, voll)");
    expect(stepLabel(step("rom", { reason: "no-state" }), "tuefteln", true).result).toBe("geschrieben (ohne Cache, voll)");
    // Other reasons keep their words; a skipped step is still skipped.
    expect(stepLabel(step("pack:maps", { done: 666, total: 666, reason: "build.cache-stale" }), "bauen", true).result).toBe("666 Member gepackt (neues Werkzeug, voll gebaut)");
    expect(stepLabel(step("rom", { status: "skipped" }), "bauen", true).result).toBe("aktuell");
  });
  it("world_step_is_named", () => {
    expect(stepLabel(step("world", { status: "incremental", done: 1, total: 3 }), "bauen")).toEqual({ name: "Karten backen", result: "teilweise neu gebaut" });
    expect(stepLabel(step("world", { status: "skipped" }), "tuefteln")).toEqual({ name: "Karten backen", result: "übersprungen" });
  });
});
