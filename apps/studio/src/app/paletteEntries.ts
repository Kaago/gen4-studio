import type { DataRowDto, HeaderSummary, ScriptFileDto, TrainerRowExtra } from "../api";
import { t } from "../i18n";
import { itemPrice, moveType, rowLabel, speciesTypes } from "../data/dataModel";
import { encounterRowLabel, rowUsers, usedByText } from "../data/encounters/encounterModel";
import type { PickerItem } from "../ui/primitives";
import type { IconName } from "../ui/primitives/icons";
import type { PaletteGroup } from "../ui/frame/Palette";
import type { DeckArea } from "../workbench/layout";
import type { NavTarget } from "../workbench/navigation";
import { UNNAMED_KEY } from "../workbench/places";
import type { Place } from "../workbench/places";
import { id4, termLabel, termPlural } from "../workbench/terms";
import type { ArchiveRow } from "../workbench/text/textModel";

/** What a command of the palette does; App carries it out. */
export type PaletteCommand =
  | "open"
  | "newGame"
  | "save"
  | "build"
  | "play"
  | "area"
  | "room"
  | "clearWorld"
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
  | "importParts"
  | "previewFrom3d"
  | "previewFromMap"
  | "gitSetup"
  | "publish"
  | "commitAll"
  | "flags"
  | "geschichten"
  | "threads";

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
  /** The deck „Layout zurücksetzen“ puts back: the shown one, or the world's on an area without a deck. */
  deck: DeckArea;
  /** Whether the shown area has a dock (`deck` is then the shown one): only there is a view to maximize or arrange. */
  docked: boolean;
  /** Whether the world's inspector floats (read when the palette opens): it can then be docked, otherwise floated. */
  inspectorFloats: boolean;
  /** Whether a matrix cell is pinned in the inspector (the import of parts works on its map). */
  cellChosen: boolean;
  /** Whether a place is shown in the world (else the 3D view has no scene): the 3D view's picture needs one. */
  placeShown: boolean;
  /** A wizard of the world is open: no new game is started beside it. */
  wizard: boolean;
  /** The open project lies in no Git repository (its status read): „Git einrichten…“ is offered. Not given: not offered. */
  gitSetup?: boolean;
  /** Why „Veröffentlichen…“ waits (no found base ROM, a broken releases list); null or not given: it can open. */
  publishLocked?: string | null;
  /** The project's own repository is there: „Stand sichern…“ is offered. Not given: not offered. */
  commitAll?: boolean;
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

/**
 * Why the 3D view has no picture to take: no place is shown (also the reason in the tab menu of „3D“ and the toast, see App). A catalog key:
 * `t(NO_PLACE)` reads it in the language shown.
 */
export const NO_PLACE = "app:palette.locks.noPlace";

/** "Verwendet von Header 0412, 0413" (lower case "verwendet" after a comma): the headers that use an archive or a script file. */
const usedBy = (ids: readonly number[], first = true) =>
  t(first ? "app:palette.usedBy" : "app:palette.usedByAfterComma", { term: termLabel("header"), ids: ids.map(id4).join(", ") });

type CommandSpec = { command: PaletteCommand; label: string; icon: IconName; area?: PickerItem["area"]; shortcut?: string; description?: string; locked?: string | null };

function commandEntries({ hasProject, commands }: PaletteSource): PaletteEntry[] {
  const { busy, hasList, inApp, deck, docked, inspectorFloats, cellChosen, placeShown, wizard, gitSetup = false, publishLocked = null, commitAll = false } = commands;
  // The reasons a command waits, read when the palette builds its entries (in the language shown).
  const NO_PROJECT = t("app:palette.locks.noProject");
  const BUSY = t("app:palette.locks.busy");
  const NO_LIST = t("app:palette.locks.noList");
  const NO_CELL = t("app:palette.locks.noCell");
  const NO_SCENE = t("app:palette.locks.noScene");
  const WIZARD_OPEN = t("app:palette.locks.wizardOpen");
  const WORLD_ONLY = t("app:palette.locks.worldOnly");
  const NO_VIEW = t("app:palette.locks.noView");
  const FLOATS = t("app:palette.locks.floats");
  const DOCKED = t("app:palette.locks.docked");
  // Why a command that works on the open project cannot run now (null: it can).
  const project = !hasProject ? NO_PROJECT : busy ? BUSY : null;
  // Only the world's dock has the Werkbank and an inspector; every dock has the Doppeldeck.
  const world = docked && deck === "welt" ? null : WORLD_ONLY;
  const view = docked ? null : NO_VIEW;
  const specs: CommandSpec[] = [
    { command: "open", label: t("app:palette.open"), icon: "open", shortcut: "⌘O", locked: busy ? BUSY : null },
    // Creating a project changes nothing of the open one; it is opened only when the user asks.
    { command: "newGame", label: t("app:palette.newGame"), icon: "plus", locked: wizard ? WIZARD_OPEN : busy ? BUSY : null },
    { command: "save", label: t("app:palette.save"), icon: "save", shortcut: "⌘S", locked: project },
    { command: "build", label: t("app:palette.build"), icon: "build", locked: project },
    { command: "play", label: t("app:palette.play"), icon: "play", shortcut: inApp ? "⌘R" : undefined, locked: project },
    { command: "area", label: t("app:palette.area"), icon: "plus", locked: hasProject ? null : NO_PROJECT },
    { command: "room", label: t("app:palette.room"), icon: "room", locked: hasProject ? null : NO_PROJECT },
    { command: "clearWorld", label: t("app:palette.clearWorld"), icon: "world", description: t("app:palette.clearWorldHint"), locked: hasProject ? null : NO_PROJECT },
    {
      command: "importParts",
      label: t("app:palette.importParts"),
      icon: "plus",
      area: "welt",
      description: t("app:palette.importPartsHint"),
      locked: project ?? (cellChosen ? null : NO_CELL),
    },
    // Phase 4.0 §3.5: the project's own preview picture from the scene; it changes no data of the project (no busy lock, as the wizards).
    // In the world the command brings the 3D view to the front itself (`uncover`, as from the tab menu), so it locks only outside it and while no place is shown (no scene then).
    {
      command: "previewFrom3d",
      label: t("app:palette.previewFrom3d"),
      icon: "world",
      area: "welt",
      description: t("app:palette.previewFrom3dHint"),
      locked: hasProject ? (world !== null ? NO_SCENE : placeShown ? null : t(NO_PLACE)) : NO_PROJECT,
    },
    {
      command: "previewFromMap",
      label: t("app:palette.previewFromMap"),
      icon: "map",
      area: "welt",
      description: t("app:palette.previewFromMapHint"),
      locked: hasProject ? (cellChosen ? null : NO_CELL) : NO_PROJECT,
    },
    { command: "list", label: t("app:palette.list"), icon: "list", locked: hasList ? null : NO_LIST },
    { command: "arrangeWerkbank", label: t("app:palette.arrangeWerkbank"), icon: "werkbank", description: t("app:palette.arrangeWerkbankHint"), locked: world },
    { command: "arrangeDoppeldeck", label: t("app:palette.arrangeDoppeldeck"), icon: "doppeldeck", description: deck === "welt" ? t("app:palette.arrangeDoppeldeckWorld") : t("app:palette.arrangeDoppeldeckOther"), locked: view },
    { command: "layout", label: t("app:palette.layout"), icon: "layers", description: t("app:palette.layoutHint", { area: t(`app:palette.deck.${deck}`) }) },
    { command: "maximize", label: t("app:palette.maximize"), icon: "maximize", description: t("app:palette.maximizeHint"), locked: view },
    { command: "floatInspector", label: t("app:palette.floatInspector"), icon: "float", description: t("app:palette.floatInspectorHint"), locked: world ?? (inspectorFloats ? FLOATS : null) },
    { command: "dockInspector", label: t("app:palette.dockInspector"), icon: "dock", description: t("app:palette.dockInspectorHint"), locked: world ?? (inspectorFloats ? null : DOCKED) },
    { command: "theme", label: t("app:palette.theme"), icon: "sun" },
    { command: "density", label: t("app:palette.density"), icon: "density" },
    { command: "terms", label: t("app:palette.terms"), icon: "help" },
    { command: "settings", label: t("app:palette.settings"), icon: "settings" },
    { command: "welt", label: t("app:palette.areaWelt"), icon: "world", area: "welt" },
    { command: "geschichten", label: t("app:palette.areaGeschichten"), icon: "stories", area: "geschichten" },
    { command: "texte", label: t("app:palette.areaTexte"), icon: "texts", area: "texte" },
    { command: "pokemon", label: t("app:palette.areaPokemon"), icon: "pokemon", area: "pokemon" },
    { command: "trainer", label: t("app:palette.trainer"), icon: "pokemon", area: "pokemon", description: t("app:palette.trainerHint") },
    { command: "moves", label: t("app:palette.moves"), icon: "pokemon", area: "pokemon", description: t("app:palette.movesHint") },
    { command: "gegenstaende", label: t("app:palette.areaGegenstaende"), icon: "items", area: "gegenstaende" },
    { command: "technik", label: t("app:palette.areaTechnik"), icon: "tech", area: "technik" },
    // Only for a project in no repository (never a repository inside one); it commits the saved state, so it waits like saving.
    ...(hasProject && gitSetup ? [{ command: "gitSetup", label: t("app:palette.gitSetup"), icon: "git", locked: busy ? BUSY : null } satisfies CommandSpec] : []),
    // The wizard builds and writes the release on the session's build path: it waits for a found base ROM, a save or build, and another wizard.
    { command: "publish", label: t("app:palette.publish"), icon: "publish", locked: !hasProject ? NO_PROJECT : (publishLocked ?? (busy ? BUSY : wizard ? WIZARD_OPEN : null)) },
    // Only with the project's own repository; it commits the saved state, so it waits like saving.
    ...(hasProject && commitAll ? [{ command: "commitAll", label: t("app:palette.commitAll"), icon: "git", locked: busy ? BUSY : null } satisfies CommandSpec] : []),
    // Geschichten › Fäden (3d.6) and › Flags und Variablen (3d.1, moved from Technik with 3d.6).
    { command: "threads", label: t("app:palette.threads"), icon: "thread", area: "geschichten", locked: hasProject ? null : NO_PROJECT },
    { command: "flags", label: t("app:palette.flags"), icon: "flag", area: "geschichten", locked: hasProject ? null : NO_PROJECT },
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
  return extra.places.length === 0 ? t("app:palette.noPlace") : [...new Set(extra.places.map((p) => p.name))].join(", ");
}

/** „Begegnungsdatei, verwendet von 2 Orten“, „Begegnungsdatei, ohne Ort“; empty for a locked row. */
function encounterUses(row: DataRowDto): string {
  if (row.extra === null) return "";
  const n = rowUsers(row).length;
  return t("app:palette.encounterUses", { uses: n === 0 ? t("app:palette.noPlaceAfterComma") : usedByText(n) });
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
      description: `${place.headers.length} ${place.headers.length === 1 ? termLabel("header") : termPlural("header")}`,
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
      label: row.place ?? (row.users.length > 0 ? t("app:palette.noPlaceName") : termLabel("textArchive")),
      // R10: the second line starts with what the row is, so typing "textarchiv" finds every archive (an unused one is named so already).
      description: row.users.length > 0 ? `${termLabel("textArchive")}, ${usedBy(row.users, false)}` : undefined,
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
      label: file.kind === "levelScript" ? termLabel("levelScript") : termLabel("scriptFile"),
      description: file.headers.length > 0 ? usedBy(file.headers) : t("app:palette.noHeader"),
      tech: id4(file.file),
      icon: "script",
      group: "Scripts",
      run: { kind: "goTo", target: { kind: "script", file: file.file } },
    });
  }
  return entries;
}
