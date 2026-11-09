import type { DataRowDto, HeaderSummary, ItemRowExtra, MoveRowExtra, SpeciesRowExtra } from "../api";
import { t } from "../i18n";
import type { DataSelectKey, PokemonSub } from "../ui/frame/shellState";
import { typeName } from "../ui/primitives/pokemonTypes";
import { encounterRowLabel } from "./encounters/encounterModel";
import { PLACEHOLDER_REASON, placeholderText, trainerRowLabel } from "./trainer/trainerModel";

/** The two data areas of the atlas. */
export type DataArea = "pokemon" | "gegenstaende";
/** The parts of the data areas: the four of Pokémon, and the one of Gegenstände. */
export type DataSub = PokemonSub | "items";
/** The data lists the areas show (and the palette lists). */
export type ListKind = "species" | "move" | "item" | "trainer" | "encounter";

/** The list each part shows. */
export const SUB_KIND: Record<DataSub, ListKind | null> = { pokedex: "species", trainer: "trainer", encounters: "encounter", moves: "move", items: "item" };
/** The name of a part in the switcher and as the title of its empty state, in the language shown. */
export const subLabel = (sub: DataSub): string => t(`data:dataModel.sub.${sub}`);
export const SUB_SHORTCUT: Record<PokemonSub, string> = { pokedex: "⌘1", trainer: "⌘2", encounters: "⌘3", moves: "⌘4" };
/** Why a part cannot be opened yet; it shows in the switcher, but locked (none since the Begegnungen came with 3b.3b). */
export const SUB_LOCKED: Partial<Record<DataSub, string>> = {};
export const isSubLocked = (sub: DataSub): boolean => SUB_LOCKED[sub] !== undefined;

/** Where each part keeps its selection in the shell state. */
export const SELECT_KEY: Record<DataSub, DataSelectKey | null> = { pokedex: "pokedex", trainer: "trainer", encounters: "encounters", moves: "moves", items: "items" };

/** The title of an area's list column, in the language shown. */
export const areaLabel = (area: DataArea): string => t(`data:dataModel.area.${area}`);
export const areaOfSub = (sub: DataSub): DataArea => (sub === "items" ? "gegenstaende" : "pokemon");

/** The field pockets of the bag by their number in the item data (pokeplatinum `POCKET_*`), as the German games name them. */
export const POCKET_IDS = ["items", "medicine", "balls", "tmHm", "berries", "mail", "battleItems", "keyItems"] as const;
/** The name of a pocket by its number, in the language shown; a pocket number the games do not know has none. */
export const pocketName = (pocket: number): string | undefined => {
  const id = POCKET_IDS[pocket];
  return id === undefined ? undefined : t(`data:dataModel.pockets.${id}`);
};
/** The names of all pockets, by their number. */
export const pocketNames = (): string[] => POCKET_IDS.map((id) => t(`data:dataModel.pockets.${id}`));
/** "Tasche Medizin"; a pocket number the games do not know keeps its number. */
export const pocketLabel = (pocket: number): string => t("data:dataModel.pocket", { name: pocketName(pocket) ?? pocket });

/** The move categories by their number in the move data. */
export const CATEGORY_IDS = ["physical", "special", "status"] as const;
/** The name of a move category by its number, in the language shown; a number the games do not know has none. */
export const categoryNameOf = (category: number): string | undefined => {
  const id = CATEGORY_IDS[category];
  return id === undefined ? undefined : t(`data:dataModel.categories.${id}`);
};
/** The names of all move categories, by their number. */
export const categoryNames = (): string[] => CATEGORY_IDS.map((id) => t(`data:dataModel.categories.${id}`));

const id4 = (n: number) => String(n).padStart(4, "0");

/** The kernel's English fallback names ("species 0520") and what the UI calls them. */
const FALLBACK: Record<ListKind, { pattern: RegExp }> = {
  species: { pattern: /^species \d+$/ },
  move: { pattern: /^move \d+$/ },
  item: { pattern: /^item \d+$/ },
  trainer: { pattern: /^trainer \d+$/ },
  encounter: { pattern: /^encounter \d+$/ },
};

/**
 * The name a row shows: the name from the text archives, else what it is with its number ("Art 0520", "Attacke 0469",
 * "Gegenstand 0099"); an item without the item table is a data member without a name ("Daten 0017"). An encounter file is named by the
 * places of the `headers` that use it ("Route 201"), else "Begegnungen 0012".
 */
export function rowLabel(kind: ListKind, row: DataRowDto, itemTable: boolean, headers: readonly HeaderSummary[] = []): string {
  if (kind === "item" && !itemTable) return t("data:dataModel.dataMember", { id: id4(row.index) });
  // "Wanderer Jens": class and name, as the game says it before a battle.
  if (kind === "trainer") return trainerRowLabel(row);
  if (kind === "encounter") return encounterRowLabel(row, headers);
  const name = row.name.trim();
  const fallback = FALLBACK[kind];
  if (name === "" || fallback.pattern.test(name)) return `${t(`data:dataModel.fallbackWord.${kind}`)} ${id4(row.index)}`;
  return name;
}

/**
 * The German sentence of a lock: the kernel says "move 0017: 18 bytes, expected 16", the editor "Attacke 0017: 18 Bytes statt 16" (with
 * `word` "Attacke"). A reason in another shape follows the member as the kernel says it.
 */
export function lockSentence(word: string, index: number, locked: string): string {
  const lengths = /: (\d+) bytes, expected (\d+)$/.exec(locked);
  return lengths
    ? t("data:dataModel.lockLengths", { word, id: id4(index), actual: lengths[1], expected: lengths[2] })
    : t("data:dataModel.lockOther", { word, id: id4(index), reason: locked });
}

/** The German words of the parts the kernel names in a lock reason ("item 0017" is a data member, which several item ids may share). */
const LOCK_PART: Readonly<Record<string, "personalData" | "learnset" | "evolutionData" | "move" | "item" | "encounter" | "trainer">> = {
  "personal data": "personalData",
  learnset: "learnset",
  "evolution data": "evolutionData",
  move: "move",
  item: "item",
  encounter: "encounter",
  trainer: "trainer",
};

/**
 * The kernel's lock reason in German, by the part it names: "item 0000: 36 bytes, expected 34" → "Daten 0000: 36 Bytes statt 34". A
 * reason in another shape stays as it is.
 */
export function lockReason(locked: string): string {
  if (locked === PLACEHOLDER_REASON) return placeholderText();
  // A trainer is a pair of members: "trainer 0012: party 20 bytes, expected 16".
  const pair = /^trainer (\d{4,}): (party|properties) (\d+) bytes, expected (\d+)$/.exec(locked);
  if (pair) return t("data:dataModel.lockTrainerPair", { id: pair[1], part: pair[2] === "party" ? t("data:dataModel.partyPart") : t("data:dataModel.propertiesPart"), actual: pair[3], expected: pair[4] });
  const m = /^(.+) (\d{4,}): (\d+) bytes, expected (\d+)$/.exec(locked);
  if (!m) return locked;
  const part = LOCK_PART[m[1]!];
  return t("data:dataModel.lockLengths", { word: part === undefined ? m[1] : t(`data:dataModel.lockParts.${part}`), id: m[2], actual: m[3], expected: m[4] });
}

/** The types of a species row in words ("Normal, Flug"; one type once); empty for a locked row. */
export function speciesTypes(row: DataRowDto): string {
  const extra = row.extra as SpeciesRowExtra | null;
  if (!extra || !Array.isArray(extra.types)) return "";
  const [a, b] = extra.types;
  return a === b ? typeName(a) : `${typeName(a)}, ${typeName(b)}`;
}

/** The type of a move row in words; empty for a locked row. */
export function moveType(row: DataRowDto): string {
  const extra = row.extra as MoveRowExtra | null;
  return extra && typeof extra.type === "number" ? typeName(extra.type) : "";
}

/** The price of an item row ("300 ₽"); empty for a locked row. */
export function itemPrice(row: DataRowDto): string {
  const extra = row.extra as ItemRowExtra | null;
  return extra && typeof extra.price === "number" ? `${extra.price} ₽` : "";
}

/** The field pocket of an item row; null for a locked row. */
export function itemPocket(row: DataRowDto): number | null {
  const extra = row.extra as ItemRowExtra | null;
  return extra && typeof extra.field_pocket === "number" ? extra.field_pocket : null;
}

/** What a list is narrowed to besides the search: types (species, moves; any of them), a pocket (items), a category (moves). */
export interface ListFilter {
  types: readonly number[];
  pocket: number | null;
  category: number | null;
}
export const NO_FILTER: ListFilter = { types: [], pocket: null, category: null };
export const isFiltered = (filter: ListFilter): boolean => filter.types.length > 0 || filter.pocket !== null || filter.category !== null;

/** The filter in words ("Tasche Medizin", "Typ Feuer, Flug", "Kategorie Status"), for the filter line; null without a filter. */
export function filterText(filter: ListFilter): string | null {
  const parts: string[] = [];
  if (filter.types.length > 0) parts.push(t("data:dataModel.filterTypes", { types: filter.types.map(typeName).join(", ") }));
  if (filter.pocket !== null) parts.push(pocketLabel(filter.pocket));
  if (filter.category !== null) parts.push(t("data:dataModel.filterCategory", { name: categoryNameOf(filter.category) ?? filter.category }));
  return parts.length === 0 ? null : parts.join(", ");
}

function rowTypes(kind: ListKind, row: DataRowDto): number[] {
  if (kind === "species") return [...((row.extra as SpeciesRowExtra | null)?.types ?? [])];
  if (kind === "move") {
    const type = (row.extra as MoveRowExtra | null)?.type;
    return type === undefined ? [] : [type];
  }
  return [];
}

/**
 * Whether a row matches the search: a number finds the member of that number and the numbers that start with it ("03" finds
 * 0300 to 0399), any text finds the names that contain it (any case).
 */
export function matchesQuery(row: DataRowDto, label: string, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (q === "") return true;
  if (/^\d+$/.test(q) && (Number(q) === row.index || row.tech.startsWith(q))) return true;
  return label.toLowerCase().includes(q);
}

/** The rows that match the search and the filter, in their order. `labels` are the names the rows show (`rowLabel`). */
export function filterRows(kind: ListKind, rows: readonly DataRowDto[], labels: readonly string[], query: string, filter: ListFilter): DataRowDto[] {
  return rows.filter((row, i) => {
    if (!matchesQuery(row, labels[i] ?? "", query)) return false;
    if (filter.types.length > 0 && !rowTypes(kind, row).some((t) => filter.types.includes(t))) return false;
    if (filter.pocket !== null && (kind !== "item" || itemPocket(row) !== filter.pocket)) return false;
    if (filter.category !== null && (kind !== "move" || (row.extra as MoveRowExtra | null)?.category !== filter.category)) return false;
    return true;
  });
}
