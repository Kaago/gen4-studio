import i18next from "i18next";
import { afterEach, describe, expect, it } from "vitest";
import { resources } from "../i18n";
import { TERMS, id4, inSentence, techLabel, termFor, termLabel, termPlural, termShort } from "./terms";
import type { TermKey } from "./terms";

const KEYS = Object.keys(TERMS) as TermKey[];

afterEach(async () => {
  // The suite runs in German (vitest.setup.ts); a test that switched the language puts it back.
  await i18next.changeLanguage("de");
});

describe("terms", () => {
  it("writes ids with four digits and the technical name", () => {
    expect(id4(7)).toBe("0007");
    expect(techLabel("matrix", 131)).toBe("Matrix 0131");
    expect(techLabel("header", 415)).toBe("Header 0415");
    expect(techLabel("eventFile", 3)).toBe("EventFile 0003");
  });

  it("carries the technical names of the spec", () => {
    expect(termLabel("matrix")).toBe("Raster");
    expect(TERMS.matrix.tech).toBe("Matrix");
    expect(TERMS.header.tech).toBe("MapHeader");
    expect(TERMS.map.tech).toBe("MapFile");
    expect(termShort("outside")).toContain("Region");
  });

  it("names_world_maps_areas_rooms_places_and_the_landscape", () => {
    expect([termLabel("worldMap"), termPlural("worldMap")]).toEqual(["Weltkarte", "Weltkarten"]);
    expect([termLabel("area"), termPlural("area")]).toEqual(["Gebiet", "Gebiete"]);
    expect(termLabel("landscape")).toBe("Landschaft");
    expect(termShort("place")).toMatch(/^Der Name, den das Spiel beim Betreten zeigt/);
    expect(termShort("room")).toContain("Eingang");
    expect(termShort("matrix")).toContain("Nur in Tüfteln");
    expect(termShort("header")).toContain("Nur in Tüfteln");
  });

  it("says_no_raster_matrix_or_header_in_building", () => {
    expect(termFor("header", "bauen", "area")).toBe("Gebiet");
    expect(termFor("header", "bauen", "room")).toBe("Raum");
    expect(termFor("header", "bauen")).toBe("Ort");
    expect(termFor("matrix", "bauen")).toBe("Weltkarte");
    // The words Bauen explains with: the game's, not the ROM's.
    for (const key of ["worldMap", "area", "room", "place", "landscape", "map", "warp"] as const) {
      expect(`${termLabel(key)} ${termPlural(key)} ${termShort(key)}`).not.toMatch(/Raster|Matrix|Header/);
    }
  });

  it("keeps_the_technical_words_in_tinkering", () => {
    expect(termFor("header", "tuefteln", "area")).toBe("Header");
    expect(termFor("header", "tuefteln")).toBe("Header");
    expect(termFor("matrix", "tuefteln")).toBe("Matrix");
  });

  it("names the text archive and its message", () => {
    expect(termLabel("textArchive")).toBe("Textarchiv");
    expect(TERMS.textArchive.tech).toBe("TextArchive");
    expect(termLabel("message")).toBe("Nachricht");
    expect(techLabel("textArchive", 199)).toBe("TextArchive 0199");
  });

  it("writes Script with a c everywhere", () => {
    expect(termLabel("scriptFile")).toBe("Scriptdatei");
    expect(TERMS.scriptFile.tech).toBe("ScriptFile");
    expect(termLabel("script")).toBe("Script");
    expect(termLabel("levelScript")).toBe("Level-Script");
    expect(termLabel("problem")).toBe("Problem");
    expect(techLabel("scriptFile", 42)).toBe("ScriptFile 0042");
    for (const key of KEYS) expect(`${termLabel(key)}${termPlural(key)}${termShort(key)}`).not.toMatch(/Skript/);
  });

  it("shows no raw markup in the short texts, which render as plain text", () => {
    for (const key of KEYS) expect(termShort(key)).not.toContain("`");
  });

  it("has a plural for every term, the nouns the explorer counts with", () => {
    for (const key of KEYS) expect(termPlural(key)).not.toBe("");
    expect([termPlural("place"), termPlural("header"), termPlural("map"), termPlural("room")]).toEqual(["Orte", "Header", "Karten", "Räume"]);
  });

  it("every term has label, plural and short in both languages", () => {
    // The catalogs are the source of the words: every `TermKey` has an entry in German and in English, none more.
    const de = resources.de.terms as Record<string, { label?: string; plural?: string; short?: string }>;
    const en = resources.en.terms as Record<string, { label?: string; plural?: string; short?: string }>;
    expect(Object.keys(de).sort()).toEqual([...KEYS].sort());
    expect(Object.keys(en).sort()).toEqual([...KEYS].sort());
    for (const key of KEYS) {
      for (const [lang, catalog] of [["de", de], ["en", en]] as const) {
        for (const field of ["label", "plural", "short"] as const) {
          const value = catalog[key][field];
          expect(typeof value === "string" && value.trim() !== "", `${lang} ${key}.${field}`).toBe(true);
        }
      }
    }
  });

  it("speaks English when the language is English", async () => {
    await i18next.changeLanguage("en");
    expect([termLabel("worldMap"), termPlural("worldMap")]).toEqual(["World map", "World maps"]);
    expect([termLabel("area"), termLabel("room"), termLabel("place")]).toEqual(["Area", "Room", "Place"]);
    expect(termShort("map")).not.toBe("");
    expect(termFor("header", "bauen", "room")).toBe("Room");
    expect(termFor("matrix", "bauen")).toBe("World map");
    // The technical names are the ROM's and do not change.
    expect(techLabel("matrix", 131)).toBe("Matrix 0131");
  });
});

describe("inSentence", () => {
  it("lower_cases_a_label_inside_an_English_sentence_but_keeps_German_nouns_and_abbreviations", async () => {
    expect(inSentence("Eventdatei")).toBe("Eventdatei");
    await i18next.changeLanguage("en");
    expect(inSentence("Event file")).toBe("event file");
    expect(inSentence("HP")).toBe("HP");
    expect(i18next.t("workbench:inspectorField.changeLabel", { label: inSentence(termLabel("eventFile")) })).toBe("Change event file…");
  });
});
