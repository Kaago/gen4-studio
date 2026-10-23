import type { HeaderSummary, ScriptFileDto } from "../api";
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
  | "layout"
  | "theme"
  | "density"
  | "terms"
  | "settings"
  | "welt"
  | "texte"
  | "technik";

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
}

export interface PaletteSource {
  places: readonly Place[];
  headers: readonly HeaderSummary[];
  archives: readonly ArchiveRow[];
  /** The script files; null when the scripts are not adopted (or not read yet). */
  scripts: readonly ScriptFileDto[] | null;
  hasProject: boolean;
  commands: CommandState;
}

const NO_PROJECT = "Erst ein Projekt öffnen";
const BUSY = "Erst Speichern oder Bauen abwarten";
const NO_LIST = "Dieser Bereich hat keine Liste";
const DECK_NAME: Record<DeckArea, string> = { welt: "Welt", texte: "Texte" };

/** "Verwendet von Header 0412, 0413" (lower case "verwendet" after a comma): the headers that use an archive or a script file. */
const usedBy = (ids: readonly number[], first = true) => `${first ? "Verwendet" : "verwendet"} von ${TERMS.header.label} ${ids.map(id4).join(", ")}`;

type CommandSpec = { command: PaletteCommand; label: string; icon: IconName; area?: PickerItem["area"]; shortcut?: string; description?: string; locked?: string | null };

function commandEntries({ hasProject, commands }: PaletteSource): PaletteEntry[] {
  const { busy, hasList, inApp, deck } = commands;
  // Why a command that works on the open project cannot run now (null: it can).
  const project = !hasProject ? NO_PROJECT : busy ? BUSY : null;
  const specs: CommandSpec[] = [
    { command: "open", label: "Projekt öffnen", icon: "open", shortcut: "⌘O", locked: busy ? BUSY : null },
    { command: "save", label: "Speichern", icon: "save", shortcut: "⌘S", locked: project },
    { command: "build", label: "Bauen", icon: "build", locked: project },
    { command: "play", label: "Hier spielen", icon: "play", shortcut: inApp ? "⌘R" : undefined, locked: project },
    { command: "area", label: "Gebiet anlegen…", icon: "plus", locked: hasProject ? null : NO_PROJECT },
    { command: "room", label: "Raum anlegen…", icon: "room", locked: hasProject ? null : NO_PROJECT },
    { command: "list", label: "Liste ein/aus", icon: "list", locked: hasList ? null : NO_LIST },
    { command: "layout", label: "Layout zurücksetzen", icon: "layers", description: `Schau und Hand von ${DECK_NAME[deck]} wie beim ersten Start` },
    { command: "theme", label: "Thema wechseln", icon: "sun" },
    { command: "density", label: "Dichte wechseln", icon: "density" },
    { command: "terms", label: "Begriffe", icon: "help" },
    { command: "settings", label: "Einstellungen", icon: "settings" },
    { command: "welt", label: "Bereich Welt", icon: "world", area: "welt" },
    { command: "texte", label: "Bereich Texte", icon: "texts", area: "texte" },
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

/**
 * Every entry of the command palette, grouped in the order Befehle, Orte, Header, Textarchive, Scripts. Ids say what an
 * entry is (`cmd-save`, `place-Zweiblattdorf`, `header-412`, `archive-412`, `script-412`), `tech` is the four-digit
 * number of a member. Without a project only the commands, and the ones that need a project are locked.
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
