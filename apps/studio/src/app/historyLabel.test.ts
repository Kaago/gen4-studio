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
    ["delete warp", "Tür gelöscht"],
    ["assign header 0414 to matrix 0003", "Header 0414 Matrix 0003 zuweisen"],
    ["connect warp to header 0411", "Warp mit Header 0411 verbinden"],
  ];

  it.each(kernel)("names_%s_in_german", (label, german) => {
    expect(historyLabel(label, "tuefteln")).toBe(german);
  });

  // The labels the commands (events, buildings, matrix, world, text, headers, area data) hand to a changeset as English developer strings.
  const commandLabels = [
    "move event",
    "edit event",
    "add event",
    "delete event",
    "change building model",
    "edit building",
    "add building",
    "delete building",
    "edit matrix cell",
    "edit matrix sections",
    "resize matrix",
    "rename matrix",
    "paint",
    "edit text",
    "add message",
    "remove message",
    "edit header",
    "edit area data",
  ];

  it.each(commandLabels)("translates_%s_into_a_text_that_is_not_the_english_label", (label) => {
    for (const depth of ["bauen", "tuefteln"] as const) {
      const german = historyLabel(label, depth);
      expect(german, `${label} (${depth})`).not.toBe(label);
      expect(german, `${label} (${depth})`).not.toBe("Änderung");
    }
  });

  it("translates_the_label_of_imported_map_parts", () => {
    expect(historyLabel("map 0668: model and permissions and terrain imported", "bauen")).toBe("Karte 0668: Modell, Kollision und Laufhöhen importiert");
    expect(historyLabel("map 0668: model and permissions and terrain imported", "tuefteln")).toBe("Karte 0668: Modell, Kollision und BDHC importiert");
    expect(historyLabel("map 0000: permissions imported", "bauen")).toBe("Karte 0000: Kollision importiert");
    expect(historyLabel("map 0003: whole map imported", "bauen")).toBe("Karte 0003: ganze Karte importiert");
    expect(historyLabel("map 0003: buildings and background sound imported", "bauen")).toBe("Karte 0003: Gebäude und Hintergrundklang importiert");
  });

  it("passes_a_german_label_of_the_frontend_through_unchanged", () => {
    expect(historyLabel("Wetter", "tuefteln")).toBe("Wetter");
    expect(historyLabel("Skript", "tuefteln")).toBe("Skript");
    expect(historyLabel("Tür verschoben", "tuefteln")).toBe("Tür verschoben");
  });

  it("reads_Aenderung_for_an_empty_or_unknown_label", () => {
    expect(historyLabel("", "tuefteln")).toBe("Änderung");
    expect(historyLabel("something new", "tuefteln")).toBe("Änderung");
  });

  it("says_no_raster_matrix_or_header_in_building", () => {
    const building: [string, string][] = [
      ["edit matrix cell", "Zelle geändert"],
      ["edit matrix sections", "Abschnitte der Weltkarte geändert"],
      ["resize matrix", "Größe der Weltkarte geändert"],
      ["rename matrix", "Weltkarte umbenannt"],
      ["edit header", "Ort geändert"],
      ["assign header 0414 to matrix 0003", "Ort 0414 Weltkarte 0003 zuweisen"],
      ["connect warp to header 0411", "Warp mit Ort 0411 verbinden"],
    ];
    for (const [label, german] of building) expect(historyLabel(label, "bauen")).toBe(german);
    for (const [label] of kernel) expect(historyLabel(label, "bauen"), label).not.toMatch(/Raster|Matrix|Header/);
    expect(germanHistory({ undo: ["edit header"], redo: ["resize matrix"] }, "bauen")).toEqual({ undo: ["Ort geändert"], redo: ["Größe der Weltkarte geändert"] });
  });

  it("keeps_the_technical_words_in_tinkering", () => {
    for (const [label, german] of kernel) expect(historyLabel(label, "tuefteln")).toBe(german);
  });

  it("translates_both_lists_and_keeps_their_order", () => {
    expect(germanHistory({ undo: ["paint", "edit header"], redo: ["add event"] }, "tuefteln")).toEqual({ undo: ["Malen", "Header geändert"], redo: ["Event hinzugefügt"] });
  });

  it("keeps_agents", () => {
    expect(germanHistory({ undo: ["paint", "edit header"], redo: ["add event"], undo_agents: [null, "Claude Code"], redo_agents: ["Agent"] }, "tuefteln")).toEqual({
      undo: ["Malen", "Header geändert"],
      redo: ["Event hinzugefügt"],
      undo_agents: [null, "Claude Code"],
      redo_agents: ["Agent"],
    });
  });
});
