import { describe, expect, it } from "vitest";
import type { HeaderSummary, ScriptFileDto } from "../api";
import { buildPlaces } from "../workbench/places";
import { archivePlaces, archiveRows, usersByArchive } from "../workbench/text/textModel";
import { filterItems } from "../ui/primitives/pickerFilter";
import { PALETTE_GROUPS } from "../ui/frame/Palette";
import { paletteEntries } from "./paletteEntries";
import type { PaletteSource } from "./paletteEntries";

const header = (id: number, location_name: string, internal_name: string, text_archive_id: number): HeaderSummary => ({
  id,
  internal_name,
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
const commands = { busy: false, hasList: true, inApp: false, deck: "welt" as const };
const source = (over: Partial<PaletteSource> = {}): PaletteSource => ({
  places: buildPlaces(headers, []),
  headers,
  archives,
  scripts,
  hasProject: true,
  commands,
  ...over,
});
const byId = (id: string, over: Partial<PaletteSource> = {}) => paletteEntries(source(over)).find((e) => e.id === id);

describe("paletteEntries", () => {
  it("lists_only_the_commands_without_a_project_and_locks_the_wizards_and_the_project_commands", () => {
    const entries = paletteEntries(source({ hasProject: false, places: [], headers: [], archives: [], scripts: null }));
    expect(new Set(entries.map((e) => e.group))).toEqual(new Set(["Befehle"]));
    expect(entries.map((e) => e.label)).toEqual([
      "Projekt öffnen",
      "Speichern",
      "Bauen",
      "Hier spielen",
      "Gebiet anlegen…",
      "Raum anlegen…",
      "Liste ein/aus",
      "Layout zurücksetzen",
      "Thema wechseln",
      "Dichte wechseln",
      "Begriffe",
      "Einstellungen",
      "Bereich Welt",
      "Bereich Texte",
      "Bereich Technik",
    ]);
    const locked = entries.filter((e) => e.disabled).map((e) => e.label);
    expect(locked).toEqual(["Speichern", "Bauen", "Hier spielen", "Gebiet anlegen…", "Raum anlegen…"]);
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

  it("puts_the_groups_in_the_order_commands_places_headers_archives_scripts", () => {
    const groups = [...new Set(paletteEntries(source()).map((e) => e.group))];
    expect(groups).toEqual(["Befehle", "Orte", "Header", "Textarchive", "Scripts"]);
    expect(PALETTE_GROUPS).toEqual(groups);
  });

  it("finds_header_and_archive_0412_in_two_groups", () => {
    const hits = filterItems(paletteEntries(source({ scripts: null })), "0412");
    expect(hits.filter((e) => e.group === "Header").map((e) => e.id)).toEqual(["header-412"]);
    expect(hits.filter((e) => e.group === "Textarchive").map((e) => e.id)).toContain("archive-412");
    expect(new Set(hits.map((e) => e.group))).toEqual(new Set(["Header", "Textarchive"]));
  });

  it("names_the_commands_shortcuts_and_locks_the_project_commands_while_busy", () => {
    expect(byId("cmd-open")).toMatchObject({ shortcut: "⌘O", icon: "open", disabled: false });
    expect(byId("cmd-save")).toMatchObject({ shortcut: "⌘S", disabled: false, run: { kind: "command", command: "save" } });
    // ⌘R plays only inside the app.
    expect(byId("cmd-play")?.shortcut).toBeUndefined();
    expect(byId("cmd-play", { commands: { ...commands, inApp: true } })?.shortcut).toBe("⌘R");
    const busy = { commands: { ...commands, busy: true } };
    for (const id of ["cmd-open", "cmd-save", "cmd-build", "cmd-play"]) expect(byId(id, busy)?.disabled).toBe(true);
    expect(byId("cmd-area", busy)?.disabled).toBe(false);
    expect(byId("cmd-welt")).toMatchObject({ area: "welt", run: { kind: "command", command: "welt" } });
  });

  it("locks_the_list_toggle_where_the_area_has_no_list_and_names_the_deck_the_layout_reset_works_on", () => {
    expect(byId("cmd-list")?.disabled).toBe(false);
    expect(byId("cmd-list", { commands: { ...commands, hasList: false } })).toMatchObject({ disabled: true, description: "Dieser Bereich hat keine Liste" });
    expect(byId("cmd-layout")?.description).toContain("Welt");
    expect(byId("cmd-layout", { commands: { ...commands, deck: "texte" } })?.description).toContain("Texte");
  });
});
