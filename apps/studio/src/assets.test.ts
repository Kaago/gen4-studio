import { describe, expect, it } from "vitest";
import type { AssetKind } from "./api";
import { assetLabel, kindLabel, staleText } from "./assets";

describe("asset labels", () => {
  it("names every asset kind in German", () => {
    const expected: Record<AssetKind, string> = {
      Map: "Karte",
      Matrix: "Matrix",
      AreaData: "Area-Data",
      Event: "Eventdatei",
      TextArchive: "Textarchiv",
      MapTexture: "Karten-Textur",
      BuildingTexture: "Gebäude-Textur",
      BuildingModel: "Gebäudemodell",
      InteriorBuildingModel: "Innen-Gebäudemodell",
      HeaderTable: "Header-Tabelle",
      DynamicHeaders: "Dynamische Header",
      Script: "Scriptdatei",
      Encounter: "Begegnungen",
      Personal: "Pokémon-Daten",
      Learnset: "Lernset",
      Evolution: "Entwicklung",
      Move: "Attacke",
      Item: "Gegenstand",
      TrainerProperties: "Trainer-Eigenschaften",
      TrainerParty: "Trainer-Team",
      TrainerTextTable: "Trainertexte",
      TrainerTextOffsets: "Trainertext-Offsets",
      ItemIcons: "Gegenstand-Symbole",
      MonIcons: "Pokémon-Symbole",
      Trainer: "Trainer",
    };
    for (const [kind, label] of Object.entries(expected)) expect(kindLabel(kind as AssetKind)).toBe(label);
  });

  it("pads the index to four digits", () => {
    expect(assetLabel({ kind: "AreaData", index: 7 })).toBe("Area-Data 0007");
    expect(assetLabel({ kind: "Personal", index: 397 })).toBe("Pokémon-Daten 0397");
  });

  it("words the stale message without raw enum names", () => {
    expect(staleText({ kind: "Map", index: 0 })).toBe("Karte 0000 wurde außerhalb geändert.");
    expect(staleText({ kind: "Event", index: 12 })).toBe("Eventdatei 0012 wurde außerhalb geändert.");
  });
});
