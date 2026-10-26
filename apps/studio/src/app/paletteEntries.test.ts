import { describe, expect, it } from "vitest";
import type { DataRowDto, HeaderSummary, ScriptFileDto } from "../api";
import { buildPlaces } from "../workbench/places";
import { archivePlaces, archiveRows, usersByArchive } from "../workbench/text/textModel";
import { filterItems } from "../ui/primitives/pickerFilter";
import { PALETTE_GROUPS } from "../ui/frame/Palette";
import { paletteEntries } from "./paletteEntries";
import type { PaletteSource } from "./paletteEntries";

const header = (id: number, location_name: string, internal_name: string, text_archive_id: number): HeaderSummary => ({
  id,
  internal_name, location_index: 0, kanto: false,
  location_name,
  matrix_id: id,
  area_data_id: 0,
  event_file_id: id,
  text_archive_id,
  script_file_id: id,
  level_script_id: 0,
});

const headers = [header(411, "Zweiblattdorf", "T01", 411), header(412, "Zweiblattdorf", "T01R0101", 412), header(3, "", "UNION", 3)];
const users = usersByArchive(headers);
const archives = archiveRows(413, users, archivePlaces(headers, users));
const scripts: ScriptFileDto[] = [
  { file: 412, kind: "script", size: 80, headers: [412] },
  { file: 9, kind: "levelScript", size: 12, headers: [] },
  { file: 10, kind: "binary", size: 4, headers: [3] },
];
const commands = { busy: false, hasList: true, inApp: false, deck: "welt" as const, docked: true, inspectorFloats: false, cellChosen: false, wizard: false };
const dataRow = (index: number, name: string, extra: DataRowDto["extra"]): DataRowDto => ({ index, name, tech: String(index).padStart(4, "0"), locked: null, extra });
const data = {
  species: [dataRow(396, "Staralili", { types: [0, 2], number: 396, stats: [40, 55, 30, 60, 30, 30] }), dataRow(397, "Staravia", { types: [0, 2], number: 397, stats: [55, 75, 50, 80, 40, 40] }), dataRow(520, "species 0520", null)],
  trainers: [
    dataRow(12, "Jens", { class: 6, class_name: "Wanderer", level_max: 12, party_count: 1, places: [{ header: 5, name: "Route 201" }] }),
    dataRow(40, "Kai", { class: 6, class_name: "Wanderer", level_max: 20, party_count: 1, places: [] }),
  ],
  encounters: [dataRow(140, "", { users: [411, 412] }), dataRow(141, "", { users: [] })],
  moves: [dataRow(17, "Flügelschlag", { type: 2, category: 0, power: 60, accuracy: 100, pp: 35 })],
  items: [dataRow(17, "Trank", { price: 300, field_pocket: 1, icon: true, data_index: 17 })],
  itemTable: true,
};
const source = (over: Partial<PaletteSource> = {}): PaletteSource => ({
  places: buildPlaces(headers, []),
  headers,
  archives,
  scripts,
  data,
  hasProject: true,
  commands,
  ...over,
});
const byId = (id: string, over: Partial<PaletteSource> = {}) => paletteEntries(source(over)).find((e) => e.id === id);

describe("paletteEntries", () => {
  it("lists_only_the_commands_without_a_project_and_locks_the_wizards_and_the_project_commands", () => {
    const entries = paletteEntries(source({ hasProject: false, places: [], headers: [], archives: [], scripts: null, data: null }));
    expect(new Set(entries.map((e) => e.group))).toEqual(new Set(["Befehle"]));
    expect(entries.map((e) => e.label)).toEqual([
      "Projekt öffnen",
      "Neues Spiel aus ROM…",
      "Speichern",
      "Bauen",
      "Hier spielen",
      "Gebiet anlegen…",
      "Raum anlegen…",
      "Welt leeren…",
      "Teile in die Karte importieren…",
      "Liste ein/aus",
      "Anordnung: Werkbank",
      "Anordnung: Doppeldeck",
      "Layout zurücksetzen",
      "Schau maximieren",
      "Inspector schweben lassen",
      "Inspector andocken",
      "Thema wechseln",
      "Dichte wechseln",
      "Begriffe",
      "Einstellungen",
      "Bereich Welt",
      "Bereich Texte",
      "Bereich Pokémon",
      "Trainer öffnen",
      "Attacken öffnen",
      "Bereich Gegenstände",
      "Bereich Technik",
    ]);
    const locked = entries.filter((e) => e.disabled).map((e) => e.label);
    expect(locked).toEqual(["Speichern", "Bauen", "Hier spielen", "Gebiet anlegen…", "Raum anlegen…", "Welt leeren…", "Teile in die Karte importieren…", "Inspector andocken"]);
    expect(entries.find((e) => e.label === "Welt leeren…")?.description).toBe("Erst ein Projekt öffnen");
    expect(entries.find((e) => e.label === "Gebiet anlegen…")?.description).toBe("Erst ein Projekt öffnen");
    for (const e of entries) expect(e.id).toMatch(/^cmd-/);
  });

  it("lists_without_a_project_no_places_headers_archives_or_scripts_even_when_given_some", () => {
    const entries = paletteEntries(source({ hasProject: false }));
    expect(new Set(entries.map((e) => e.group))).toEqual(new Set(["Befehle"]));
  });

  it("names_each_member_with_its_id_and_four_digit_tech_in_its_group", () => {
    expect(byId("header-412")).toMatchObject({ label: "Zweiblattdorf", tech: "0412", description: "T01R0101", group: "Header", area: "welt", run: { kind: "goTo", target: { kind: "header", id: 412 } } });
    // A header without a location name is named by its internal name.
    expect(byId("header-3")).toMatchObject({ label: "UNION", tech: "0003", group: "Header" });
    expect(byId("archive-412")).toMatchObject({ label: "Zweiblattdorf", tech: "0412", group: "Textarchive", area: "texte", run: { kind: "goTo", target: { kind: "textArchive", index: 412 } } });
    // An archive no header uses is named by what it is.
    expect(byId("archive-7")).toMatchObject({ label: "Textarchiv", tech: "0007" });
    expect(byId("script-412")).toMatchObject({ label: "Scriptdatei", tech: "0412", group: "Scripts", icon: "script", run: { kind: "goTo", target: { kind: "script", file: 412 } } });
    expect(byId("script-9")).toMatchObject({ label: "Level-Script", tech: "0009" });
    // A binary file cannot be opened in the editor.
    expect(byId("script-10")).toBeUndefined();
    // A place leads to its first header (outside first, then by id).
    expect(byId("place-Zweiblattdorf")).toMatchObject({ label: "Zweiblattdorf", group: "Orte", area: "welt", run: { kind: "goTo", target: { kind: "header", id: 411 } } });
  });

  it("leaves_the_nameless_place_out_and_the_scripts_when_none_were_read", () => {
    const entries = paletteEntries(source({ scripts: null }));
    expect(entries.some((e) => e.group === "Orte" && e.label === "Ohne Ortsnamen")).toBe(false);
    expect(entries.some((e) => e.group === "Scripts")).toBe(false);
  });

  it("lists_species_moves_and_items_with_their_four_digit_number_and_what_they_are", () => {
    expect(byId("species-397")).toMatchObject({ label: "Staravia", tech: "0397", description: "Normal, Flug", group: "Pokémon", area: "pokemon", run: { kind: "goTo", target: { kind: "species", index: 397 } } });
    // A species without a name in the text archive is named in German.
    expect(byId("species-520")).toMatchObject({ label: "Art 0520", tech: "0520", description: undefined });
    expect(byId("trainer-12")).toMatchObject({ label: "Wanderer Jens", tech: "0012", description: "Route 201", group: "Trainer", area: "pokemon", run: { kind: "goTo", target: { kind: "trainer", index: 12 } } });
    expect(byId("trainer-40")).toMatchObject({ label: "Wanderer Kai", description: "Ohne Ort" });
    // An encounter file by the places of its headers.
    expect(byId("encounter-140")).toMatchObject({ label: "Zweiblattdorf", tech: "0140", description: "Begegnungsdatei, verwendet von 2 Orten", group: "Begegnungen", area: "pokemon", run: { kind: "goTo", target: { kind: "encounter", index: 140 } } });
    expect(byId("encounter-141")).toMatchObject({ label: "Begegnungen 0141", description: "Begegnungsdatei, ohne Ort" });
    expect(byId("move-17")).toMatchObject({ label: "Flügelschlag", tech: "0017", description: "Flug", group: "Attacken", area: "pokemon", run: { kind: "goTo", target: { kind: "move", index: 17 } } });
    expect(byId("item-17")).toMatchObject({ label: "Trank", tech: "0017", description: "300 ₽", group: "Gegenstände", area: "gegenstaende", run: { kind: "goTo", target: { kind: "item", id: 17 } } });
    // Without the item table an item is a data member without a name.
    expect(byId("item-17", { data: { ...data, items: [dataRow(17, "", { price: 300, field_pocket: 1, icon: false, data_index: 17 })], itemTable: false } })).toMatchObject({ label: "Daten 0017" });
  });

  it("finds_the_species_0397_by_its_number", () => {
    const hits = filterItems(paletteEntries(source({ scripts: null })), "0397");
    expect(hits.filter((e) => e.group === "Pokémon").map((e) => e.id)).toEqual(["species-397"]);
    expect(filterItems(paletteEntries(source()), "stara").map((e) => e.id)).toEqual(["species-396", "species-397"]);
  });

  it("lists_no_data_while_it_is_not_read", () => {
    const entries = paletteEntries(source({ data: null }));
    expect(entries.some((e) => e.group === "Pokémon" || e.group === "Trainer" || e.group === "Begegnungen" || e.group === "Attacken" || e.group === "Gegenstände")).toBe(false);
  });

  it("opens_the_pokemon_area_the_moves_and_the_items_by_command", () => {
    expect(byId("cmd-pokemon")).toMatchObject({ label: "Bereich Pokémon", area: "pokemon", run: { kind: "command", command: "pokemon" } });
    expect(byId("cmd-trainer")).toMatchObject({ label: "Trainer öffnen", area: "pokemon", run: { kind: "command", command: "trainer" } });
    expect(byId("cmd-moves")).toMatchObject({ label: "Attacken öffnen", area: "pokemon", run: { kind: "command", command: "moves" } });
    expect(byId("cmd-gegenstaende")).toMatchObject({ label: "Bereich Gegenstände", area: "gegenstaende", run: { kind: "command", command: "gegenstaende" } });
  });

  it("puts_the_groups_in_the_order_commands_places_headers_archives_scripts", () => {
    const groups = [...new Set(paletteEntries(source()).map((e) => e.group))];
    expect(groups).toEqual(["Befehle", "Orte", "Header", "Textarchive", "Pokémon", "Trainer", "Begegnungen", "Attacken", "Gegenstände", "Scripts"]);
    expect(PALETTE_GROUPS).toEqual(groups);
  });

  it("finds_header_and_archive_0412_in_two_groups", () => {
    const hits = filterItems(paletteEntries(source({ scripts: null })), "0412");
    expect(hits.filter((e) => e.group === "Header").map((e) => e.id)).toEqual(["header-412"]);
    expect(hits.filter((e) => e.group === "Textarchive").map((e) => e.id)).toContain("archive-412");
    expect(new Set(hits.map((e) => e.group))).toEqual(new Set(["Header", "Textarchive"]));
  });

  it("says_textarchiv_first_in_an_archive_row_so_typing_textarchiv_finds_them", () => {
    // R10: the place stays the label and the number the grey tech; the second line says what the row is.
    expect(byId("archive-412")).toMatchObject({ label: "Zweiblattdorf", tech: "0412", description: "Textarchiv, verwendet von Header 0412" });
    const hits = filterItems(paletteEntries(source({ scripts: null })), "textarchiv").map((e) => e.id);
    expect(hits).toContain("archive-412");
    expect(hits).toContain("archive-411");
    expect(hits).toContain("archive-7");
  });

  it("names_the_commands_shortcuts_and_locks_the_project_commands_while_busy", () => {
    expect(byId("cmd-open")).toMatchObject({ shortcut: "⌘O", icon: "open", disabled: false });
    // The density has its own icon (rows), not the map legend's.
    expect(byId("cmd-density")).toMatchObject({ label: "Dichte wechseln", icon: "density" });
    expect(byId("cmd-save")).toMatchObject({ shortcut: "⌘S", disabled: false, run: { kind: "command", command: "save" } });
    // ⌘R plays only inside the app.
    expect(byId("cmd-play")?.shortcut).toBeUndefined();
    expect(byId("cmd-play", { commands: { ...commands, inApp: true } })?.shortcut).toBe("⌘R");
    const busy = { commands: { ...commands, busy: true } };
    for (const id of ["cmd-open", "cmd-save", "cmd-build", "cmd-play"]) expect(byId(id, busy)?.disabled).toBe(true);
    expect(byId("cmd-area", busy)?.disabled).toBe(false);
    expect(byId("cmd-welt")).toMatchObject({ area: "welt", run: { kind: "command", command: "welt" } });
  });

  it("offers_a_new_game_from_a_rom_without_a_project_but_not_while_busy_or_while_a_wizard_is_open", () => {
    expect(byId("cmd-newGame", { hasProject: false })).toMatchObject({ label: "Neues Spiel aus ROM…", icon: "plus", disabled: false, run: { kind: "command", command: "newGame" } });
    expect(byId("cmd-newGame", { commands: { ...commands, busy: true } })).toMatchObject({ disabled: true, description: "Erst Speichern oder Bauen abwarten" });
    expect(byId("cmd-newGame", { commands: { ...commands, wizard: true } })).toMatchObject({ disabled: true, description: "Erst den Assistenten abschließen oder abbrechen" });
  });

  it("locks_the_list_toggle_where_the_area_has_no_list_and_names_the_deck_the_layout_reset_works_on", () => {
    expect(byId("cmd-list")?.disabled).toBe(false);
    expect(byId("cmd-list", { commands: { ...commands, hasList: false } })).toMatchObject({ disabled: true, description: "Dieser Bereich hat keine Liste" });
    expect(byId("cmd-layout")?.description).toContain("Welt");
    expect(byId("cmd-layout", { commands: { ...commands, deck: "texte" } })?.description).toContain("Texte");
  });

  it("offers_to_clear_the_world_with_a_project", () => {
    expect(byId("cmd-clearWorld")).toMatchObject({ label: "Welt leeren…", icon: "world", disabled: false, description: "Leere Region mit Startstadt", run: { kind: "command", command: "clearWorld" } });
  });

  it("lists_the_six_arrangement_commands", () => {
    const ids = ["cmd-arrangeWerkbank", "cmd-arrangeDoppeldeck", "cmd-layout", "cmd-maximize", "cmd-floatInspector", "cmd-dockInspector"];
    const labels = ["Anordnung: Werkbank", "Anordnung: Doppeldeck", "Layout zurücksetzen", "Schau maximieren", "Inspector schweben lassen", "Inspector andocken"];
    // In the world they run, also without a project (they arrange the frame, not the project); of the inspector's two only the one that
    // fits its state: docked it can float, floating it can dock.
    const inWorld = paletteEntries(source({ hasProject: false })).filter((e) => ids.includes(e.id));
    expect(inWorld.map((e) => e.label)).toEqual(labels);
    expect(inWorld.map((e) => e.run)).toEqual(ids.map((id) => ({ kind: "command", command: id.slice("cmd-".length) })));
    expect(inWorld.filter((e) => e.disabled).map((e) => [e.id, e.description])).toEqual([["cmd-dockInspector", "Der Inspector ist angedockt"]]);
    const floats = { commands: { ...commands, inspectorFloats: true } };
    expect(byId("cmd-floatInspector", floats)).toMatchObject({ disabled: true, description: "Der Inspector schwebt schon" });
    expect(byId("cmd-dockInspector", floats)?.disabled).toBe(false);
    expect(byId("cmd-layout")?.description).toBe("Welt in der gewählten Anordnung neu aufbauen");
    // Elsewhere only the world has the Werkbank and the inspector: those three say so; Doppeldeck rebuilds the shown dock, whose view is
    // maximized.
    const inTexts = { commands: { ...commands, deck: "texte" as const } };
    for (const id of ["cmd-arrangeWerkbank", "cmd-floatInspector", "cmd-dockInspector"]) {
      expect(byId(id, inTexts)).toMatchObject({ disabled: true, description: "Nur in der Welt" });
    }
    expect(byId("cmd-arrangeDoppeldeck", inTexts)?.disabled).toBe(false);
    expect(byId("cmd-layout", inTexts)).toMatchObject({ disabled: false, description: "Texte in der gewählten Anordnung neu aufbauen" });
    expect(byId("cmd-maximize", inTexts)?.disabled).toBe(false);
    // An area without a dock (Start, Technik, Einstellungen) has no view to maximize or arrange; the reset puts the world's back.
    const noDock = { commands: { ...commands, docked: false } };
    expect(byId("cmd-maximize", noDock)).toMatchObject({ disabled: true, description: "Dieser Bereich hat keine Schau" });
    expect(byId("cmd-arrangeDoppeldeck", noDock)).toMatchObject({ disabled: true, description: "Dieser Bereich hat keine Schau" });
    expect(byId("cmd-floatInspector", noDock)).toMatchObject({ disabled: true, description: "Nur in der Welt" });
    expect(byId("cmd-layout", noDock)?.disabled).toBe(false);
  });

  it("offers_the_import_of_parts_only_for_a_chosen_cell_with_a_map", () => {
    expect(byId("cmd-importParts")).toMatchObject({ label: "Teile in die Karte importieren…", disabled: true, description: "Erst eine Rasterzelle mit Karte wählen" });
    expect(byId("cmd-importParts", { commands: { ...commands, cellChosen: true } })).toMatchObject({ disabled: false, description: "In die Karte der gewählten Rasterzelle", run: { kind: "command", command: "importParts" } });
  });
});
