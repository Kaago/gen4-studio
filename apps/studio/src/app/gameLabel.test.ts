import { describe, expect, it } from "vitest";
import { gameLabel, projectName } from "./gameLabel";

describe("gameLabel", () => {
  it("names_the_family_and_language_the_backend_reports", () => {
    expect(gameLabel({ family: "Pt", language: "German" })).toBe("Platin, DE");
    expect(gameLabel({ family: "Hgss", language: "English" })).toBe("HeartGold/SoulSilver, EN");
    expect(gameLabel({ family: "Dp", language: "Japanese" })).toBe("Diamant/Perl, JP");
  });

  it("reads_codes_in_any_case_and_keeps_what_it_does_not_know", () => {
    expect(gameLabel({ family: "HGSS", language: "en" })).toBe("HeartGold/SoulSilver, EN");
    expect(gameLabel({ family: "Xyz", language: "Klingon" })).toBe("Xyz, KLINGON");
  });
});

describe("projectName", () => {
  it("is_the_last_folder_of_the_path", () => {
    expect(projectName("/Users/me/Roms/pt-3c1")).toBe("pt-3c1");
    expect(projectName("/Users/me/Roms/pt-3c1/")).toBe("pt-3c1");
    expect(projectName("C:\\Roms\\hg")).toBe("hg");
    expect(projectName("/")).toBe("/");
  });
});
