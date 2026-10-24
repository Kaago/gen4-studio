import type { DataRowDto, ItemRowExtra, MoveRowExtra, SpeciesRowExtra } from "../api";
import type { DataSelectKey, PokemonSub } from "../ui/frame/shellState";
import { typeName } from "../ui/primitives/pokemonTypes";

/** The two data areas of the atlas. */
export type DataArea = "pokemon" | "gegenstaende";
/** The parts of the data areas: the four of Pokémon, and the one of Gegenstände. */
export type DataSub = PokemonSub | "items";
/** The data lists the areas show (and the palette lists). */
export type ListKind = "species" | "move" | "item";

/** The list each part shows; null for a part whose list comes later (Trainer, Begegnungen). */
export const SUB_KIND: Record<DataSub, ListKind | null> = { pokedex: "species", trainer: null, encounters: null, moves: "move", items: "item" };
export const SUB_LABEL: Record<DataSub, string> = { pokedex: "Pokédex", trainer: "Trainer", encounters: "Begegnungen", moves: "Attacken", items: "Gegenstände" };
export const SUB_SHORTCUT: Record<PokemonSub, string> = { pokedex: "⌘1", trainer: "⌘2", encounters: "⌘3", moves: "⌘4" };
/** Why a part cannot be opened yet; it shows in the switcher, but locked. */
export const SUB_LOCKED: Partial<Record<DataSub, string>> = { trainer: "kommt mit Phase 3b.3b", encounters: "kommt mit Phase 3b.3b" };
export const isSubLocked = (sub: DataSub): boolean => SUB_LOCKED[sub] !== undefined;

/** Where each part keeps its selection in the shell state. */
export const SELECT_KEY: Record<DataSub, DataSelectKey | null> = { pokedex: "pokedex", trainer: null, encounters: null, moves: "moves", items: "items" };

export const AREA_LABEL: Record<DataArea, string> = { pokemon: "Pokémon", gegenstaende: "Gegenstände" };
export const areaOfSub = (sub: DataSub): DataArea => (sub === "items" ? "gegenstaende" : "pokemon");

/** The field pockets of the bag by their number in the item data (pokeplatinum `POCKET_*`), as the German games name them. */
export const POCKET_NAMES: readonly string[] = ["Items", "Medizin", "Bälle", "TM/VM", "Beeren", "Briefe", "Kampf-Items", "Basis-Items"];
/** "Tasche Medizin"; a pocket number the games do not know keeps its number. */
export const pocketLabel = (pocket: number): string => `Tasche ${POCKET_NAMES[pocket] ?? pocket}`;

/** The move categories by their number in the move data. */
export const CATEGORY_NAMES: readonly string[] = ["Physisch", "Speziell", "Status"];

const id4 = (n: number) => String(n).padStart(4, "0");

/** The kernel's English fallback names ("species 0520") and what the UI calls them. */
const FALLBACK: Record<ListKind, { pattern: RegExp; word: string }> = {
  species: { pattern: /^species \d+$/, word: "Art" },
  move: { pattern: /^move \d+$/, word: "Attacke" },
  item: { pattern: /^item \d+$/, word: "Gegenstand" },
};

/**
 * The name a row shows: the name from the text archives, else what it is with its number ("Art 0520", "Attacke 0469",
 * "Gegenstand 0099"); an item without the item table is a data member without a name ("Daten 0017").
 */
export function rowLabel(kind: ListKind, row: DataRowDto, itemTable: boolean): string {
  if (kind === "item" && !itemTable) return `Daten ${id4(row.index)}`;
  const name = row.name.trim();
  const fallback = FALLBACK[kind];
  if (name === "" || fallback.pattern.test(name)) return `${fallback.word} ${id4(row.index)}`;
  return name;
}

/**
 * The German sentence of a lock: the kernel says "move 0017: 18 bytes, expected 16", the editor "Attacke 0017: 18 Bytes statt 16" (with
 * `word` "Attacke"). A reason in another shape follows the member as the kernel says it.
 */
export function lockSentence(word: string, index: number, locked: string): string {
  const lengths = /: (\d+) bytes, expected (\d+)$/.exec(locked);
  return lengths ? `${word} ${id4(index)}: ${lengths[1]} Bytes statt ${lengths[2]}` : `${word} ${id4(index)}: ${locked}`;
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
  if (filter.types.length > 0) parts.push(`Typ ${filter.types.map(typeName).join(", ")}`);
  if (filter.pocket !== null) parts.push(pocketLabel(filter.pocket));
  if (filter.category !== null) parts.push(`Kategorie ${CATEGORY_NAMES[filter.category] ?? filter.category}`);
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
