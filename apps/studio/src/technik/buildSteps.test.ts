import { describe, expect, it } from "vitest";
import type { BuildStep } from "../api";
import { formatSeconds, stepLabel } from "./buildSteps.ts";

const step = (id: string, over: Partial<BuildStep> = {}): BuildStep => ({ id, status: "full", ms: 0, done: 0, total: 0, cached: 0, reason: null, ...over });

describe("buildSteps", () => {
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
});
