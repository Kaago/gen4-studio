import { describe, expect, it } from "vitest";
import type { RomInfo } from "../api";
import { destOf, gameName, languageName, nameProblem, suggestName } from "./projectName";

const rom = (change: Partial<RomInfo> = {}): RomInfo => ({
  title: "POKEMON PL",
  game_code: "CPUD",
  family: "pt",
  language: "german",
  revision: 0,
  size: 134_217_728,
  sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4",
  known_base: "Platin DE (Testprojekt)",
  support: { kind: "verified" },
  ...change,
});

describe("suggestName", () => {
  it("names_the_game_of_the_rom", () => {
    expect(suggestName(rom())).toBe("Neues Platin");
    expect(suggestName(rom({ family: "hgss", game_code: "IPKD" }))).toBe("Neues HeartGold");
    expect(suggestName(rom({ family: "hgss", game_code: "IPGE" }))).toBe("Neues SoulSilver");
    expect(suggestName(rom({ family: "dp", game_code: "ADAD" }))).toBe("Neues Spiel");
    expect(suggestName(rom({ family: null, game_code: "ABCD" }))).toBe("Neues Spiel");
  });
});

describe("gameName and languageName", () => {
  it("say_the_game_and_the_language_in_german", () => {
    expect(gameName(rom())).toBe("Platin");
    expect(gameName(rom({ family: "hgss", game_code: "IPKE" }))).toBe("HeartGold");
    expect(gameName(rom({ family: "hgss", game_code: "IPGD" }))).toBe("SoulSilver");
    expect(gameName(rom({ family: "dp", game_code: "APAD" }))).toBe("Diamant/Perl");
    expect(gameName(rom({ family: null }))).toBe("Unbekanntes Spiel");
    expect(languageName("german")).toBe("Deutsch");
    expect(languageName("english")).toBe("Englisch");
    expect(languageName("french")).toBe("Französisch");
    expect(languageName("italian")).toBe("Italienisch");
    expect(languageName("spanish")).toBe("Spanisch");
    expect(languageName("japanese")).toBe("Japanisch");
    expect(languageName("korean")).toBe("Koreanisch");
    expect(languageName(null)).toBe("Unbekannte Sprache");
  });
});

describe("nameProblem", () => {
  it("accepts_a_plain_name", () => {
    expect(nameProblem("Neues Platin")).toBeNull();
    expect(nameProblem("x".repeat(64))).toBeNull();
  });

  it("refuses_an_empty_name_separators_a_leading_dot_and_more_than_64_characters", () => {
    expect(nameProblem("")).toBe("Gib dem Projekt einen Namen.");
    expect(nameProblem("   ")).toBe("Gib dem Projekt einen Namen.");
    const separators = "Der Name darf kein /, \\ oder : enthalten und nicht mit einem Punkt beginnen.";
    expect(nameProblem("a/b")).toBe(separators);
    expect(nameProblem("a\\b")).toBe(separators);
    expect(nameProblem("a:b")).toBe(separators);
    expect(nameProblem(".versteckt")).toBe(separators);
    expect(nameProblem("x".repeat(65))).toBe("Höchstens 64 Zeichen.");
  });
});

describe("destOf", () => {
  it("joins_parent_and_name_with_or_without_the_dspre_ending", () => {
    expect(destOf("/roms/_testkopien", "Neues Platin", false)).toBe("/roms/_testkopien/Neues Platin");
    expect(destOf("/roms/_testkopien/", "Neues Platin", false)).toBe("/roms/_testkopien/Neues Platin");
    expect(destOf("/roms/_testkopien", "Neues Platin", true)).toBe("/roms/_testkopien/Neues Platin_DSPRE_contents");
    expect(destOf("/roms/_testkopien/", "Neues Platin", true)).toBe("/roms/_testkopien/Neues Platin_DSPRE_contents");
  });

  it("keeps_a_windows_parent_and_the_file_system_root", () => {
    expect(destOf("C:\\Users\\a\\Documents\\Gen4 Studio\\", "Neu", true)).toBe("C:\\Users\\a\\Documents\\Gen4 Studio\\Neu_DSPRE_contents");
    expect(destOf("/", "Neu", false)).toBe("/Neu");
  });

  it("trims_the_name", () => {
    expect(destOf("/p", "  Neu  ", false)).toBe("/p/Neu");
  });
});
