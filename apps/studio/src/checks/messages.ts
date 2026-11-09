import type { AssetKind, CheckGroup, CheckProblemDto, ChecksProgressDto, SubjectKind } from "../api";
import { kindLabel } from "../assets";
import { t } from "../i18n";
import { depthAtLeast } from "../ui/frame/shellState";
import type { Depth } from "../ui/frame/shellState";
import { id4 } from "../workbench/terms";

/**
 * What a check is, whatever the language: its `group` (what a cheap run covers) and its `severity` (the highest the check reports).
 * Both are the kernel's: the Rust test `commands::checks::messages_ts` holds codes, order, groups and severities against
 * `g4_kernel::checks::Registry::all()`, so this list is complete and in catalogue order. One line per code: the test reads it line by line.
 */
export interface CheckInfo {
  group: CheckGroup;
  severity: "error" | "warning" | "info";
}

/**
 * The words of a check, read from the catalog (`i18n/locales/<lang>/checks.json`) in the language the app speaks now: its title (the
 * head of its group in the Problems panel and its switch in Technik) and its sentence with a way out („Die Tür führt ins Leere. Wähle ein
 * Ziel oder lösche die Tür.“). In a sentence `{n}` is argument n as the kernel gives it, `{n:id}` a number with four digits („Header 0414“),
 * `{n:field}` a field name; `fill` puts them in, i18next does not (it interpolates `{{name}}`, which the sentences do not have).
 * `building` holds Bauen's title and sentence where they would name a header or a matrix (Bauen speaks of places and world maps).
 */
export interface CheckMessage extends CheckInfo {
  title: string;
  text: string;
  /** The sentence when argument 1, a count, is „1“ (the singular: „1 Datei hat sich geändert“). */
  one?: string;
  building?: { title?: string; text: string };
}

export const CHECK_MESSAGES: Readonly<Record<string, CheckInfo>> = {
  "warp.dest-header": { group: "cheap", severity: "error" },
  "warp.dest-warp": { group: "cheap", severity: "error" },
  "warp.off-map": { group: "cheap", severity: "error" },
  "warp.never-fires": { group: "cheap", severity: "info" },
  "event.trainer-missing": { group: "cheap", severity: "error" },
  "event.off-map": { group: "cheap", severity: "error" },
  "event.trainer-blind": { group: "cheap", severity: "warning" },
  "event.trigger-empty": { group: "cheap", severity: "warning" },
  "event.same-spot": { group: "cheap", severity: "warning" },
  "header.ref-range": { group: "cheap", severity: "error" },
  "header.wild-dangling": { group: "cheap", severity: "error" },
  "header.text-split": { group: "cheap", severity: "warning" },
  "matrix.map-range": { group: "cheap", severity: "error" },
  "matrix.header-range": { group: "cheap", severity: "error" },
  "matrix.header-elsewhere": { group: "cheap", severity: "error" },
  "map.model-invalid": { group: "cheap", severity: "error" },
  "map.bdhc-size": { group: "cheap", severity: "error" },
  "map.building-range": { group: "cheap", severity: "error" },
  "map.altitude-walkable": { group: "cheap", severity: "error" },
  "map.step-unblocked": { group: "cheap", severity: "info" },
  "map.buildings-over-limit": { group: "cheap", severity: "warning" },
  "area.tileset-range": { group: "cheap", severity: "error" },
  "place.name-too-long": { group: "cheap", severity: "warning" },
  "place.index-out-of-bank": { group: "cheap", severity: "error" },
  "trainer.no-party": { group: "cheap", severity: "error" },
  "trainer.empty-species": { group: "cheap", severity: "error" },
  "trainer.id-range": { group: "cheap", severity: "error" },
  "trainer.no-moves": { group: "cheap", severity: "warning" },
  "trainer.double-one": { group: "cheap", severity: "warning" },
  "encounter.empty-slot": { group: "cheap", severity: "warning" },
  "encounter.level-zero": { group: "cheap", severity: "warning" },
  "encounter.level-high": { group: "cheap", severity: "warning" },
  "encounter.min-max": { group: "cheap", severity: "info" },
  "encounter.rate-byte": { group: "cheap", severity: "warning" },
  "build.hidden-file": { group: "cheap", severity: "error" },
  "build.member-gap": { group: "cheap", severity: "error" },
  "build.unpacked-not-packed": { group: "cheap", severity: "warning" },
  "build.base-missing": { group: "cheap", severity: "info" },
  "asset.unreadable": { group: "cheap", severity: "error" },
  "event.script-missing": { group: "per_script", severity: "error" },
  "header.level-target": { group: "per_script", severity: "warning" },
  "header.script-kind": { group: "per_script", severity: "error" },
  "script.message-missing": { group: "per_script", severity: "error" },
  "script.unreadable": { group: "per_script", severity: "error" },
  "text.too-wide": { group: "per_archive", severity: "warning" },
  "text.three-lines": { group: "per_archive", severity: "warning" },
  "text.encode": { group: "per_archive", severity: "error" },
  "text.json-broken": { group: "per_archive", severity: "error" },
};

/** Every code, in catalogue order. */
export const CHECK_CODES: readonly string[] = Object.keys(CHECK_MESSAGES);

/** Where a finding is listed in the Problems panel: by its subject; the project's own files (an archive that cannot be read) apart. */
export type CheckArea = "welt" | "scripts" | "texte" | "pokemon" | "projekt";

/** The name of an area of the Problems panel in the language shown (`checks:_words.areas`). */
export const areaLabel = (area: CheckArea): string => t(`checks:_words.areas.${area}`);

/** The names of the areas, each read when it is asked for (a getter: i18next is not ready at module scope, and the language may switch). */
export const AREA_LABELS: Readonly<Record<CheckArea, string>> = {
  get welt() {
    return areaLabel("welt");
  },
  get scripts() {
    return areaLabel("scripts");
  },
  get texte() {
    return areaLabel("texte");
  },
  get pokemon() {
    return areaLabel("pokemon");
  },
  get projekt() {
    return areaLabel("projekt");
  },
};

const AREA_OF: Readonly<Record<SubjectKind, CheckArea>> = {
  header: "welt",
  eventFile: "welt",
  matrix: "welt",
  map: "welt",
  areaData: "welt",
  scriptFile: "scripts",
  textArchive: "texte",
  trainer: "pokemon",
  encounter: "pokemon",
  project: "projekt",
};

/** The area a finding is listed under. */
export const areaOf = (f: Pick<CheckProblemDto, "subject">): CheckArea => AREA_OF[f.subject.kind];

/** The group of the check `code` (what a cheap run covers); null for a code this list does not know. */
export const groupOf = (code: string): CheckGroup | null => CHECK_MESSAGES[code]?.group ?? null;

/** The fields of a catalog entry (`checks.json`: `"<code>": { title, text, one?, building?: { title?, text } }`). */
type Word = "title" | "text" | "one" | "building.title" | "building.text";

/**
 * One word of the check `code` in the language the app speaks now; undefined when the entry has no such field (`one` and `building` are
 * optional). The key is `<code>.<field>`: a code has a dot of its own, and i18next finds the catalog's flat `"warp.dest-header"` entry
 * behind it (`ignoreJSONStructure`, on by default). Read when a message is asked for, never at module scope: i18next starts after the
 * modules load, and a language switch must show in the next call.
 */
function wordOf(code: string, field: Word): string | undefined {
  const text = t(`checks:${code}.${field}` as "checks:warp.dest-header.title", { defaultValue: "" });
  return text === "" ? undefined : text;
}

/** The words of the check `code` from the catalog, with its group and severity; null for a code this list does not know. */
export function checkMessage(code: string): CheckMessage | null {
  const info = CHECK_MESSAGES[code];
  if (!info) return null;
  const one = wordOf(code, "one");
  const buildingTitle = wordOf(code, "building.title");
  const buildingText = wordOf(code, "building.text");
  return {
    ...info,
    title: wordOf(code, "title") ?? code,
    text: wordOf(code, "text") ?? code,
    ...(one !== undefined && { one }),
    ...(buildingText !== undefined && { building: { ...(buildingTitle !== undefined && { title: buildingTitle }), text: buildingText } }),
  };
}

/** The title of the check `code` in the words of `depth`; the code itself when it is not known. */
export const checkTitle = (code: string, depth: Depth): string =>
  (!depthAtLeast(depth, "tuefteln") ? wordOf(code, "building.title") : undefined) ?? wordOf(code, "title") ?? code;

/** Field names of headers, area data and trainers in the words of the editors: the kernel's field name, and its key in `checks:_words.fields`. */
const FIELD_WORDS = {
  matrix_id: "matrixId",
  event_file_id: "eventFileId",
  script_file_id: "scriptFileId",
  level_script_id: "levelScriptId",
  text_archive_id: "textArchiveId",
  area_data_id: "areaDataId",
  wild_pokemon: "wildPokemon",
  map_tileset: "mapTileset",
  buildings_tileset: "buildingsTileset",
  class: "class",
  items: "items",
  species: "species",
  item: "item",
  moves: "moves",
} as const;
const fieldWord = (field: string): string | undefined => {
  const key = (FIELD_WORDS as Readonly<Record<string, (typeof FIELD_WORDS)[keyof typeof FIELD_WORDS]>>)[field];
  return key === undefined ? undefined : t(`checks:_words.fields.${key}`);
};

/** The field and asset words Bauen says instead of a matrix or a header (`checks:_words.building`). */
const buildingFieldWord = (field: string): string | undefined => (field === "matrix_id" ? t("checks:_words.building.matrixId") : undefined);
const BUILDING_ASSET_KEYS = { Matrix: "matrix", HeaderTable: "headerTable", DynamicHeaders: "dynamicHeaders" } as const;
const buildingAssetWord = (kind: AssetKind): string | undefined => {
  const key = (BUILDING_ASSET_KEYS as Partial<Record<AssetKind, (typeof BUILDING_ASSET_KEYS)[keyof typeof BUILDING_ASSET_KEYS]>>)[kind];
  return key === undefined ? undefined : t(`checks:_words.building.${key}`);
};

/** The kernel's names of asset kinds (`AssetKind::label`), for `asset.unreadable`. */
const ASSET_KINDS: Readonly<Record<string, AssetKind>> = {
  map: "Map",
  matrix: "Matrix",
  "area data": "AreaData",
  "event file": "Event",
  "text archive": "TextArchive",
  "map texture": "MapTexture",
  "building texture": "BuildingTexture",
  "building model": "BuildingModel",
  "interior building model": "InteriorBuildingModel",
  "header table": "HeaderTable",
  "dynamic headers": "DynamicHeaders",
  "script file": "Script",
  "encounter file": "Encounter",
  "personal data": "Personal",
  learnset: "Learnset",
  "evolution data": "Evolution",
  move: "Move",
  item: "Item",
  "trainer properties": "TrainerProperties",
  "trainer party": "TrainerParty",
  "trainer text table": "TrainerTextTable",
  "trainer text offsets": "TrainerTextOffsets",
  "item icons": "ItemIcons",
  "menu icons": "MonIcons",
  trainer: "Trainer",
  "battle sprites": "PokemonBattleSprites",
  "form battle sprites": "OtherPokemonBattleSprites",
  "trainer graphics": "TrainerGraphics",
  "egg moves": "EggMoves",
};

/** „Eventdatei 0411“, or „Das Archiv Pokémon-Daten“ for a whole archive (no index). */
function assetName(kind: string, index: string, depth: Depth): string {
  const known = ASSET_KINDS[kind];
  const word = known ? ((!depthAtLeast(depth, "tuefteln") ? buildingAssetWord(known) : undefined) ?? kindLabel(known)) : kind;
  return index === "" ? t("checks:_words.wholeArchive", { word }) : `${word} ${id4(Number(index))}`;
}

/** Puts the arguments into `text` (see `CheckMessage`). */
function fill(text: string, args: readonly string[], depth: Depth): string {
  return text.replace(/\{(\d+)(?::(id|field))?\}/g, (_, n: string, format?: string) => {
    const value = args[Number(n)] ?? "";
    if (format === "id" && /^\d+$/.test(value)) return id4(Number(value));
    if (format === "field") return (!depthAtLeast(depth, "tuefteln") ? buildingFieldWord(value) : undefined) ?? fieldWord(value) ?? value;
    return value;
  });
}

/**
 * The sentence of a finding with its way out, in the words of `depth` and the language of the app; the kernel's (English) sentence for
 * a code this list does not know.
 */
export function findingText(f: Pick<CheckProblemDto, "code" | "key" | "args" | "message">, depth: Depth): string {
  if (!CHECK_MESSAGES[f.code]) return f.message;
  const text = (!depthAtLeast(depth, "tuefteln") ? wordOf(f.code, "building.text") : undefined) ?? (f.args[1] === "1" ? wordOf(f.code, "one") : undefined) ?? wordOf(f.code, "text");
  if (text === undefined) return f.message;
  if (f.code === "asset.unreadable") {
    const [kind = "", index = "", why = ""] = f.args;
    const name = assetName(kind, index, depth);
    if (f.key.startsWith("locked:")) {
      return t("checks:_words.locked", { name, why });
    }
    return fill(text, f.args, depth).replace("{asset}", name);
  }
  return fill(text, f.args, depth);
}

/** Where a finding is, as the kernel names it („Header 0012 · Matrix“); Bauen says „Ort 0012 · Weltkarte“. */
export function findingLabel(f: Pick<CheckProblemDto, "label">, depth: Depth): string {
  if (depthAtLeast(depth, "tuefteln")) return f.label;
  const place = t("checks:_words.building.header");
  const worldMap = t("checks:_words.building.matrix");
  return f.label.replace(/\bHeader\b/g, place).replace(/\bMatrix\b/g, worldMap);
}

/** The passes a progress names, by the kernel's label: the key of each in `checks:_words.progress`. */
const PROGRESS_KEYS = { cheap: "cheap", load: "load", scripts: "scripts", texts: "texts" } as const satisfies Record<ChecksProgressDto["label"], string>;

/** „Prüft … Texte 312/724“: how far a running pass got; „Prüft …“ before it says. */
export function progressText(progress: ChecksProgressDto | null | undefined): string {
  if (!progress) return t("checks:_words.checking");
  const key = (PROGRESS_KEYS as Readonly<Record<string, (typeof PROGRESS_KEYS)[keyof typeof PROGRESS_KEYS]>>)[progress.label];
  const what = key === undefined ? progress.label : t(`checks:_words.progress.${key}`);
  return t("checks:_words.checkingPass", { what, done: progress.done, total: progress.total });
}

/** The project's ignore file; a report names it as `skipped` code when it could not be read. */
const IGNORE_FILE = "g4-checks.json";

const COMMON_SCRIPTS = /^(\d+) script files that no header uses \(common scripts\) are not checked/;
const ONE_FILE = /^(level )?script file (\d+) is not checked: (.+)$/;

/**
 * A check that did not run, or left part of its work undone, as a hint line („Nachricht fehlt: 28 Common-Scripts ohne Header nicht geprüft.“):
 * sentences in the language shown with the way out, the kernel's reason only as a detail in brackets.
 */
export function skippedText({ code, why }: { code: string; why: string }, depth: Depth): string {
  // Not a check: the ignore file could not be read when the project opened (ruling R15).
  if (code === IGNORE_FILE) return t("checks:_words.skipped.ignoreFile", { file: IGNORE_FILE, why });
  const title = checkTitle(code, depth);
  if (why === "scripts not adopted") return t("checks:_words.skipped.notAdopted", { title });
  const common = COMMON_SCRIPTS.exec(why);
  if (common) {
    const count = Number(common[1]);
    return !depthAtLeast(depth, "tuefteln") ? t("checks:_words.skipped.commonBuilding", { title, count }) : t("checks:_words.skipped.commonTech", { title, count });
  }
  const file = ONE_FILE.exec(why);
  if (file) {
    const kind = file[1] ? t("checks:_words.skipped.levelScript") : t("checks:_words.skipped.scriptFile");
    return t("checks:_words.skipped.oneFile", { title, kind, id: id4(Number(file[2])), why: file[3] });
  }
  // Mostly a file that cannot be read (the header table, an archive): its finding „Datei nicht lesbar“ names it.
  return t("checks:_words.skipped.other", { title, why, unreadable: checkTitle("asset.unreadable", depth) });
}
