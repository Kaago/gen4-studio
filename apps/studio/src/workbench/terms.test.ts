import { describe, expect, it } from "vitest";
import { TERMS, id4, techLabel } from "./terms";

describe("terms", () => {
  it("writes ids with four digits and the technical name", () => {
    expect(id4(7)).toBe("0007");
    expect(techLabel("matrix", 131)).toBe("Matrix 0131");
    expect(techLabel("header", 415)).toBe("Header 0415");
    expect(techLabel("eventFile", 3)).toBe("EventFile 0003");
  });

  it("carries the technical names of the spec", () => {
    expect(TERMS.matrix).toMatchObject({ label: "Raster", tech: "Matrix" });
    expect(TERMS.header.tech).toBe("MapHeader");
    expect(TERMS.map.tech).toBe("MapFile");
    expect(TERMS.outside.short).toContain("Region");
  });

  it("has a plural for every term, the nouns the explorer counts with", () => {
    for (const term of Object.values(TERMS)) expect(term.plural).not.toBe("");
    expect([TERMS.place.plural, TERMS.header.plural, TERMS.map.plural, TERMS.room.plural]).toEqual(["Orte", "Header", "Karten", "Räume"]);
  });
});
