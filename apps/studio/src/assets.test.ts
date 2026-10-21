import { describe, expect, it } from "vitest";
import type { AssetKind } from "./api";
import { assetLabel, kindLabel, staleText } from "./assets";

describe("asset labels", () => {
  it("names every asset kind in German", () => {
    const expected: Record<AssetKind, string> = {
      Map: "Karte",
      Matrix: "Matrix",
      AreaData: "Area-Data",
      Event: "Event-Datei",
      TextArchive: "Textarchiv",
      MapTexture: "Karten-Textur",
      BuildingTexture: "Gebäude-Textur",
      BuildingModel: "Gebäudemodell",
      InteriorBuildingModel: "Innen-Gebäudemodell",
      HeaderTable: "Header-Tabelle",
    };
    for (const [kind, label] of Object.entries(expected)) expect(kindLabel(kind as AssetKind)).toBe(label);
  });

  it("pads the index to four digits", () => {
    expect(assetLabel({ kind: "AreaData", index: 7 })).toBe("Area-Data 0007");
  });

  it("words the stale message without raw enum names", () => {
    expect(staleText({ kind: "Map", index: 0 })).toBe("Karte 0000 wurde außerhalb geändert.");
    expect(staleText({ kind: "Event", index: 12 })).toBe("Event-Datei 0012 wurde außerhalb geändert.");
  });
});
