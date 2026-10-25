import type { DataRowDto, HeaderSummary, ScriptFileDto, TrainerRowExtra } from "../api";
import { itemPrice, moveType, rowLabel, speciesTypes } from "../data/dataModel";
import { encounterRowLabel, rowUsers, usedByText } from "../data/encounters/encounterModel";
import type { PickerItem } from "../ui/primitives";
import type { IconName } from "../ui/primitives/icons";
import type { PaletteGroup } from "../ui/frame/Palette";
import type { DeckArea } from "../workbench/layout";
import type { NavTarget } from "../workbench/navigation";
import { UNNAMED_KEY } from "../workbench/places";
import type { Place } from "../workbench/places";
import { TERMS, id4 } from "../workbench/terms";
import type { ArchiveRow } from "../workbench/text/textModel";

/** What a command of the palette does; App carries it out. */
export type PaletteCommand =
  | "open"
  | "save"
  | "build"
  | "play"
  | "area"
  | "room"
  | "list"
  | "arrangeWerkbank"
  | "arrangeDoppeldeck"
  | "layout"
  | "maximize"
  | "floatInspector"
  | "dockInspector"
  | "theme"
  | "density"
  | "terms"
  | "settings"
  | "welt"
  | "texte"
  | "pokemon"
  | "trainer"
  | "moves"
  | "gegenstaende"
  | "technik"
  | "importParts";

/** What picking an entry does: follow a link (`goTo`) or run a command. */
export type PaletteRun = { kind: "goTo"; target: NavTarget } | { kind: "command"; command: PaletteCommand };

export interface PaletteEntry extends PickerItem {
  id: string;
  group: PaletteGroup;
  run: PaletteRun;
}

/** What the commands depend on besides the project. */
export interface CommandState {
  /** A save or build runs: opening, saving, building and playing wait (the title bar's buttons are disabled too). */
  busy: boolean;
  /** Whether the shown area has a list that "Liste ein/aus" can fold. */
  hasList: boolean;
  /** Inside the Tauri window, where ⌘R plays (in a browser it reloads the page). */
  inApp: boolean;
  /** The deck "Layout zurücksetzen" puts back: the shown one, or the world's on an area without a deck. */
  deck: DeckArea;
  /** Whether the shown area has a dock (`deck` is then the shown one): only there is a view to maximize or arrange. */
  docked: boolean;
  /** Whether the world's inspector floats (read when the palette opens): it can then be docked, otherwise floated. */
  inspectorFloats: boolean;
  /** Whether a matrix cell is pinned in the inspector (the import of parts works on its map). */
  cellChosen: boolean;
}

export interface PaletteData {
  species: readonly DataRowDto[];
  /** The trainers (not read yet or not given: none). */
  trainers?: readonly DataRowDto[];
  /** The encounter files (not read yet or not given: none). */
  encounters?: readonly DataRowDto[];
  moves: readonly DataRowDto[];
  items: readonly DataRowDto[];
  itemTable: boolean;
}

export interface PaletteSource {
  places: readonly Place[];
  headers: readonly HeaderSummary[];
  archives: readonly ArchiveRow[];
  /** The script files; null when the scripts are not adopted (or not read yet). */
  scripts: readonly ScriptFileDto[] | null;
  /** The species, moves and items (a list not read yet is empty); `itemTable`: whether items are item ids with names. Null: none read. */
  data: PaletteData | null;
  hasProject: boolean;
  commands: CommandState;
}

const NO_PROJECT = "Erst ein Projekt öffnen";
const BUSY = "Erst Speichern oder Bauen abwarten";
const NO_LIST = "Dieser Bereich hat keine Liste";
const NO_CELL = "Erst eine Rasterzelle mit Karte wählen";
const WORLD_ONLY = "Nur in der Welt";
const NO_VIEW = "Dieser Bereich hat keine Schau";
const FLOATS = "Der Inspector schwebt schon";
const DOCKED = "Der Inspector ist angedockt";
const DECK_NAME: Record<DeckArea, string> = { welt: "Welt", texte: "Texte", pokemon: "Pokémon", gegenstaende: "Gegenstände" };

/** "Verwendet von Header 0412, 0413" (lower case "verwendet" after a comma): the headers that use an archive or a script file. */
const usedBy = (ids: readonly number[], first = true) => `${first ? "Verwendet" : "verwendet"} von ${TERMS.header.label} ${ids.map(id4).join(", ")}`;

type CommandSpec = { command: PaletteCommand; label: string; icon: IconName; area?: PickerItem["area"]; shortcut?: string; description?: string; locked?: string | null };

function commandEntries({ hasProject, commands }: PaletteSource): PaletteEntry[] {
  const { busy, hasList, inApp, deck, docked, inspectorFloats, cellChosen } = commands;
  // Why a command that works on the open project cannot run now (null: it can).
  const project = !hasProject ? NO_PROJECT : busy ? BUSY : null;
  // Only the world's dock has the Werkbank and an inspector; every dock has the Doppeldeck.
  const world = docked && deck === "welt" ? null : WORLD_ONLY;
  const view = docked ? null : NO_VIEW;
  const specs: CommandSpec[] = [
    { command: "open", label: "Projekt öffnen", icon: "open", shortcut: "⌘O", locked: busy ? BUSY : null },
    { command: "save", label: "Speichern", icon: "save", shortcut: "⌘S", locked: project },
    { command: "build", label: "Bauen", icon: "build", locked: project },
    { command: "play", label: "Hier spielen", icon: "play", shortcut: inApp ? "⌘R" : undefined, locked: project },
    { command: "area", label: "Gebiet anlegen…", icon: "plus", locked: hasProject ? null : NO_PROJECT },
    { command: "room", label: "Raum anlegen…", icon: "room", locked: hasProject ? null : NO_PROJECT },
    {
      command: "importParts",
      label: "Teile in die Karte importieren…",
      icon: "plus",
      area: "welt",
      description: "In die Karte der gewählten Rasterzelle",
      locked: project ?? (cellChosen ? null : NO_CELL),
    },
    { command: "list", label: "Liste ein/aus", icon: "list", locked: hasList ? null : NO_LIST },
    { command: "arrangeWerkbank", label: "Anordnung: Werkbank", icon: "werkbank", description: "Die Schau mit dem Inspector daneben", locked: world },
    { command: "arrangeDoppeldeck", label: "Anordnung: Doppeldeck", icon: "doppeldeck", description: deck === "welt" ? "Die Schau über Inspector, Script und Probleme" : "Die Schau über dem Bearbeiten", locked: view },
    { command: "layout", label: "Layout zurücksetzen", icon: "layers", description: `${DECK_NAME[deck]} in der gewählten Anordnung neu aufbauen` },
    { command: "maximize", label: "Schau maximieren", icon: "maximize", description: "Oder wiederherstellen, auch mit Esc", locked: view },
    { command: "floatInspector", label: "Inspector schweben lassen", icon: "float", description: "Am rechten Rand der Schau", locked: world ?? (inspectorFloats ? FLOATS : null) },
    { command: "dockInspector", label: "Inspector andocken", icon: "dock", description: "Dahin, wo die Anordnung ihn hat", locked: world ?? (inspectorFloats ? null : DOCKED) },
    { command: "theme", label: "Thema wechseln", icon: "sun" },
    { command: "density", label: "Dichte wechseln", icon: "density" },
    { command: "terms", label: "Begriffe", icon: "help" },
    { command: "settings", label: "Einstellungen", icon: "settings" },
    { command: "welt", label: "Bereich Welt", icon: "world", area: "welt" },
    { command: "texte", label: "Bereich Texte", icon: "texts", area: "texte" },
    { command: "pokemon", label: "Bereich Pokémon", icon: "pokemon", area: "pokemon" },
    { command: "trainer", label: "Trainer öffnen", icon: "pokemon", area: "pokemon", description: "Die Trainer im Bereich Pokémon" },
    { command: "moves", label: "Attacken öffnen", icon: "pokemon", area: "pokemon", description: "Die Attacken im Bereich Pokémon" },
    { command: "gegenstaende", label: "Bereich Gegenstände", icon: "items", area: "gegenstaende" },
    { command: "technik", label: "Bereich Technik", icon: "tech", area: "technik" },
  ];
  return specs.map(({ command, label, icon, area, shortcut, description, locked }) => ({
    id: `cmd-${command}`,
    label,
    icon,
    area,
    shortcut,
    // A locked command says why instead of what it does.
    description: locked ?? description,
    disabled: locked != null,
    group: "Befehle",
    run: { kind: "command", command },
  }));
}

/** Where a trainer stands ("Route 201, Zweiblattdorf"), else "Ohne Ort"; empty for a locked row. */
function trainerPlaces(row: DataRowDto): string {
  const extra = row.extra as TrainerRowExtra | null;
  if (!extra || !Array.isArray(extra.places)) return "";
  return extra.places.length === 0 ? "Ohne Ort" : [...new Set(extra.places.map((p) => p.name))].join(", ");
}

/** „Begegnungsdatei, verwendet von 2 Orten“, „Begegnungsdatei, ohne Ort“; empty for a locked row. */
function encounterUses(row: DataRowDto): string {
  if (row.extra === null) return "";
  const n = rowUsers(row).length;
  return `Begegnungsdatei, ${n === 0 ? "ohne Ort" : usedByText(n)}`;
}

/**
 * The species, trainers, encounter files, moves and items: their names as the lists show them (an encounter file by the places of
 * `headers`), with types, places, users or the price as the second line.
 */
function dataEntries({ species, trainers = [], encounters = [], moves, items, itemTable }: PaletteData, headers: readonly HeaderSummary[]): PaletteEntry[] {
  const entries: PaletteEntry[] = [];
  const add = (row: DataRowDto, entry: Omit<PaletteEntry, "tech" | "description"> & { description: string }) =>
    entries.push({ ...entry, tech: row.tech, description: entry.description === "" ? undefined : entry.description });
  for (const row of species) {
    add(row, { id: `species-${row.index}`, label: rowLabel("species", row, itemTable), description: speciesTypes(row), area: "pokemon", group: "Pokémon", run: { kind: "goTo", target: { kind: "species", index: row.index } } });
  }
  for (const row of trainers) {
    add(row, { id: `trainer-${row.index}`, label: rowLabel("trainer", row, itemTable), description: trainerPlaces(row), area: "pokemon", group: "Trainer", run: { kind: "goTo", target: { kind: "trainer", index: row.index } } });
  }
  for (const row of encounters) {
    add(row, { id: `encounter-${row.index}`, label: encounterRowLabel(row, headers), description: encounterUses(row), area: "pokemon", group: "Begegnungen", run: { kind: "goTo", target: { kind: "encounter", index: row.index } } });
  }
  for (const row of moves) {
    add(row, { id: `move-${row.index}`, label: rowLabel("move", row, itemTable), description: moveType(row), area: "pokemon", group: "Attacken", run: { kind: "goTo", target: { kind: "move", index: row.index } } });
  }
  for (const row of items) {
    add(row, { id: `item-${row.index}`, label: rowLabel("item", row, itemTable), description: itemPrice(row), area: "gegenstaende", group: "Gegenstände", run: { kind: "goTo", target: { kind: "item", id: row.index } } });
  }
  return entries;
}

/**
 * Every entry of the command palette, grouped in the order Befehle, Orte, Header, Textarchive, Pokémon, Attacken, Gegenstände, Scripts.
 * Ids say what an entry is (`cmd-save`, `place-Zweiblattdorf`, `header-412`, `archive-412`, `species-397`, `move-17`, `item-17`,
 * `script-412`), `tech` is the four-digit number of a member. Without a project only the commands, and the ones that need a project are
 * locked.
 */
export function paletteEntries(source: PaletteSource): PaletteEntry[] {
  const entries = commandEntries(source);
  if (!source.hasProject) return entries;
  for (const place of source.places) {
    // The nameless headers are no place to go to; each of them is found as a header.
    const first = place.headers[0];
    if (place.key === UNNAMED_KEY || first === undefined) continue;
    entries.push({
      id: `place-${place.key}`,
      label: place.name,
      description: `${place.headers.length} ${place.headers.length === 1 ? TERMS.header.label : TERMS.header.plural}`,
      area: "welt",
      group: "Orte",
      run: { kind: "goTo", target: { kind: "header", id: first.header.id } },
    });
  }
  for (const h of source.headers) {
    entries.push({
      id: `header-${h.id}`,
      label: h.location_name || h.internal_name,
      description: h.location_name ? h.internal_name : undefined,
      tech: id4(h.id),
      area: "welt",
      group: "Header",
      run: { kind: "goTo", target: { kind: "header", id: h.id } },
    });
  }
  for (const row of source.archives) {
    entries.push({
      id: `archive-${row.index}`,
      // Like the texts' list: the place the archive is named after, else what it is.
      label: row.place ?? (row.users.length > 0 ? "Ohne Ortsnamen" : TERMS.textArchive.label),
      // R10: the second line starts with what the row is, so typing "textarchiv" finds every archive (an unused one is named so already).
      description: row.users.length > 0 ? `${TERMS.textArchive.label}, ${usedBy(row.users, false)}` : undefined,
      tech: id4(row.index),
      area: "texte",
      group: "Textarchive",
      run: { kind: "goTo", target: { kind: "textArchive", index: row.index } },
    });
  }
  if (source.data !== null) entries.push(...dataEntries(source.data, source.headers));
  for (const file of source.scripts ?? []) {
    // A binary file cannot be opened in the editor.
    if (file.kind === "binary") continue;
    entries.push({
      id: `script-${file.file}`,
      label: file.kind === "levelScript" ? TERMS.levelScript.label : TERMS.scriptFile.label,
      description: file.headers.length > 0 ? usedBy(file.headers) : "Ohne Header",
      tech: id4(file.file),
      icon: "script",
      group: "Scripts",
      run: { kind: "goTo", target: { kind: "script", file: file.file } },
    });
  }
  return entries;
}
