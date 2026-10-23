import { describe, expect, it } from "vitest";
import { germanHistory, historyLabel } from "./historyLabel";

describe("historyLabel", () => {
  // Every label the commands pass to a changeset (store.edit_*), in the order of the commands.
  const kernel: [string, string][] = [
    ["paint", "Malen"],
    ["edit matrix cell", "Rasterfeld geändert"],
    ["edit matrix sections", "Raster-Abschnitte geändert"],
    ["resize matrix", "Raster-Größe geändert"],
    ["rename matrix", "Raster umbenannt"],
    ["build area", "Gebiet gebaut"],
    ["extend area", "Gebiet erweitert"],
    ["build room", "Raum gebaut"],
    ["door tile", "Türkachel gesetzt"],
    ["add event", "Event hinzugefügt"],
    ["delete event", "Event gelöscht"],
    ["move event", "Event verschoben"],
    ["edit event", "Event geändert"],
    ["add building", "Gebäude hinzugefügt"],
    ["delete building", "Gebäude gelöscht"],
    ["change building model", "Gebäudemodell gewechselt"],
    ["edit building", "Gebäude geändert"],
    ["edit header", "Header geändert"],
    ["edit area data", "Area-Data geändert"],
    ["edit text", "Text geändert"],
    ["add message", "Nachricht hinzugefügt"],
    ["remove message", "Nachricht gelöscht"],
    ["set permission", "Begehbarkeit geändert"],
    ["assign header 0414 to matrix 0003", "Header 0414 Matrix 0003 zuweisen"],
    ["connect warp to header 0411", "Warp mit Header 0411 verbinden"],
  ];

  it.each(kernel)("names_%s_in_german", (label, german) => {
    expect(historyLabel(label)).toBe(german);
  });

  it("passes_a_german_label_of_the_frontend_through_unchanged", () => {
    expect(historyLabel("Wetter")).toBe("Wetter");
    expect(historyLabel("Skript")).toBe("Skript");
    expect(historyLabel("Tür verschoben")).toBe("Tür verschoben");
  });

  it("reads_Aenderung_for_an_empty_or_unknown_label", () => {
    expect(historyLabel("")).toBe("Änderung");
    expect(historyLabel("something new")).toBe("Änderung");
  });

  it("translates_both_lists_and_keeps_their_order", () => {
    expect(germanHistory({ undo: ["paint", "edit header"], redo: ["add event"] })).toEqual({ undo: ["Malen", "Header geändert"], redo: ["Event hinzugefügt"] });
  });
});
