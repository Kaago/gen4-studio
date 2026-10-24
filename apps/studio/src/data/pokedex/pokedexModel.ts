import { fieldLabels } from "../../api";
import type { AssetId, DataRowDto, LearnEntryDto, NameTablesDto, PersonalDto, PersonalField, SpeciesRowExtra, SpeciesTab } from "../../api";
import type { SelectOption } from "../../ui/primitives";
import { typeName } from "../../ui/primitives/pokemonTypes";
import { id4 } from "../../workbench/terms";

/** The six base stats in the order the games list them (summary screen). */
export type StatField = "hp" | "attack" | "defense" | "sp_attack" | "sp_defense" | "speed";

/** The base stats with their label and their place in a row's `extra.stats` (HP, Attack, Defense, Speed, Special Attack, Special Defense). */
export const STATS: readonly { field: StatField; label: string; row: number }[] = [
  { field: "hp", label: fieldLabels.species.hp, row: 0 },
  { field: "attack", label: fieldLabels.species.attack, row: 1 },
  { field: "defense", label: fieldLabels.species.defense, row: 2 },
  { field: "sp_attack", label: fieldLabels.species.sp_attack, row: 4 },
  { field: "sp_defense", label: fieldLabels.species.sp_defense, row: 5 },
  { field: "speed", label: fieldLabels.species.speed, row: 3 },
];

/** The highest species of the National Dex; members after it are the eggs (494, 495) and the forms (496 to 507). */
const LAST_SPECIES = 493;
const FIRST_FORM = 496;
const LAST_FORM = 507;
const isForm = (index: number) => index >= FIRST_FORM && index <= LAST_FORM;
/** A species of its own: not member 0 (the placeholder), not an egg, not a form; members after the forms (a hack's) count as species. */
const isSpecies = (index: number) => index !== 0 && (index <= LAST_SPECIES || index > LAST_FORM);

/** What an evolution can lead to (and a species in the party can be): a species of its own, no placeholder, egg or form member. */
export const isEvolutionTarget = isSpecies;

const statsOf = (row: DataRowDto) => {
  const extra = row.extra as SpeciesRowExtra | null;
  return extra && Array.isArray(extra.stats) ? extra.stats : null;
};

/** The average of each base stat over every species (without member 0, the eggs, the forms and locked rows); 0 without any. */
export function statAverages(rows: readonly DataRowDto[]): Record<StatField, number> {
  const sums = [0, 0, 0, 0, 0, 0];
  let n = 0;
  for (const row of rows) {
    const stats = statsOf(row);
    if (!isSpecies(row.index) || stats === null) continue;
    stats.forEach((v, i) => (sums[i]! += v));
    n++;
  }
  return Object.fromEntries(STATS.map((s) => [s.field, n === 0 ? 0 : sums[s.row]! / n])) as Record<StatField, number>;
}

/** The sum of the six base stats. */
export const statSum = (stats: Pick<PersonalDto, StatField>): number => STATS.reduce((sum, s) => sum + stats[s.field], 0);

/** What the Pokédex list holds: "493 Arten, 12 Formen" (the eggs and member 0 are not counted). */
export function speciesCount(rows: readonly DataRowDto[]): string {
  const species = rows.filter((r) => isSpecies(r.index)).length;
  const forms = rows.filter((r) => isForm(r.index)).length;
  const parts = [`${species} ${species === 1 ? "Art" : "Arten"}`];
  if (forms > 0) parts.push(`${forms} ${forms === 1 ? "Form" : "Formen"}`);
  return parts.join(", ");
}

/** The EV yield: six two-bit fields in one word (KP in bits 0 and 1, then Angriff, …); bits 12 to 15 are not part of it. */
export const EV_YIELD: readonly { field: PersonalField; stat: StatField; mask: number }[] = (
  ["hp", "attack", "defense", "speed", "sp_attack", "sp_defense"] as const
).map((stat, i) => ({ field: `ev_yield_${stat}` as PersonalField, stat, mask: 0b11 << (2 * i) }));

/** The six yields of an EV word. */
export const evYieldFromWord = (word: number): number[] => EV_YIELD.map((e, i) => (word & e.mask) >>> (2 * i));
/** The EV word of six yields (each kept to its two bits). */
export const evYieldWord = (values: readonly number[]): number => EV_YIELD.reduce((word, e, i) => word | ((((values[i] ?? 0) & 0b11) << (2 * i)) & e.mask), 0);

/** The gender ratios the games use, in words; another byte shows as "Wert 100". */
export const GENDER: readonly SelectOption<number>[] = [
  { value: 0, label: "nur männlich" },
  { value: 31, label: "87,5 % männlich" },
  { value: 63, label: "75 % männlich" },
  { value: 127, label: "50 % männlich" },
  { value: 191, label: "75 % weiblich" },
  { value: 225, label: "87,5 % weiblich" },
  { value: 254, label: "nur weiblich" },
  { value: 255, label: "geschlechtslos" },
];
export const genderText = (ratio: number): string => GENDER.find((g) => g.value === ratio)?.label ?? `Wert ${ratio}`;

/** A whole number with German thousands points ("3.840"). */
const grouped = (n: number) => String(n).replace(/\B(?=(\d{3})+(?!\d))/g, ".");

/** The steps to hatch an egg: 256 per cycle. */
export const hatchText = (cycles: number): string => `${cycles} ${cycles === 1 ? "Zyklus" : "Zyklen"}, ${grouped(cycles * 256)} Schritte`;

/** The six growth rates by their number in the personal data (dspre-report §1.4). */
export const GROWTH_RATES: readonly SelectOption<number>[] = ["Mittelschnell", "Unregelmäßig", "Schwankend", "Mittellangsam", "Schnell", "Langsam"].map(
  (label, value) => ({ value, label }),
);

/** The fifteen egg groups (1 to 15). */
export const EGG_GROUPS: readonly SelectOption<number>[] = [
  "Monster",
  "Wasser 1",
  "Käfer",
  "Flug",
  "Feld",
  "Fee",
  "Pflanze",
  "Humanotyp",
  "Wasser 3",
  "Mineral",
  "Amorph",
  "Wasser 2",
  "Ditto",
  "Drache",
  "Unbekannt",
].map((label, i) => ({ value: i + 1, label }));

/** The body colours of the Pokédex search (bits 0 to 6 of the colour byte). */
export const COLORS: readonly SelectOption<number>[] = ["Rot", "Blau", "Gelb", "Grün", "Schwarz", "Braun", "Lila", "Grau", "Weiß", "Rosa"].map((label, value) => ({ value, label }));

/** What the parameter of an evolution method means (dspre-report §1.4); `number` for a method the games do not know. */
export type ParamKind = "none" | "level" | "item" | "move" | "species" | "beauty" | "number";

/** The evolution methods 0 to 26 of the fourth generation (pokeplatinum `EVO_*`, DSPRE `EvolutionFile.cs`) and their parameter. */
export const EVO_METHODS: readonly (SelectOption<number> & { param: ParamKind })[] = (
  [
    ["Keine", "none"],
    ["Freundschaft", "none"],
    ["Freundschaft am Tag", "none"],
    ["Freundschaft in der Nacht", "none"],
    ["Level", "level"],
    ["Tausch", "none"],
    ["Tausch mit getragenem Gegenstand", "item"],
    ["Gegenstand anwenden", "item"],
    ["Level, Angriff größer als Verteidigung", "level"],
    ["Level, Angriff gleich Verteidigung", "level"],
    ["Level, Angriff kleiner als Verteidigung", "level"],
    ["Level, Persönlichkeitswert niedrig", "level"],
    ["Level, Persönlichkeitswert hoch", "level"],
    ["Level, Ninjask-Zweig", "level"],
    ["Level, Ninjatom bei freiem Platz", "level"],
    ["Schönheit", "beauty"],
    ["Gegenstand anwenden, männlich", "item"],
    ["Gegenstand anwenden, weiblich", "item"],
    ["Level am Tag mit getragenem Gegenstand", "item"],
    ["Level in der Nacht mit getragenem Gegenstand", "item"],
    ["Level mit Attacke", "move"],
    ["Level mit Art im Team", "species"],
    ["Level, männlich", "level"],
    ["Level, weiblich", "level"],
    ["Level am Kraterberg", "none"],
    ["Level im Ewigwald", "none"],
    ["Level auf Route 217", "none"],
  ] as const
).map(([label, param], value) => ({ value, label, param }));

export const paramKind = (method: number): ParamKind => EVO_METHODS[method]?.param ?? "number";

/** What the parameter field of a kind is called ("Level, Entwicklung 1"). */
export const PARAM_LABEL: Record<ParamKind, string> = {
  none: "Parameter",
  level: "Level",
  item: "Gegenstand",
  move: "Attacke",
  species: "Art",
  beauty: "Schönheit",
  number: "Parameter",
};

/** The parts of a species document by asset kind, as a lock names them. */
const PART_NAME: Partial<Record<AssetId["kind"], string>> = { Personal: "Pokémon-Daten", Learnset: "Lernset", Evolution: "Entwicklungen" };

/**
 * The German sentence of a lock: the kernel says "personal 0397: 36 bytes, expected 44", the editor "Pokémon-Daten 0397: 36 Bytes statt
 * 44". A reason in another shape follows the part as the kernel says it.
 */
export function lockText(locked: string, asset: AssetId | null): string {
  const part = asset === null ? "Pokémon-Daten" : (PART_NAME[asset.kind] ?? asset.kind);
  const where = asset === null ? part : `${part} ${String(asset.index).padStart(4, "0")}`;
  const lengths = /: (\d+) bytes, expected (\d+)$/.exec(locked);
  return lengths ? `${where}: ${lengths[1]} Bytes statt ${lengths[2]}` : `${where}: ${locked}`;
}

/** TM01 to TM92, then VM01 to VM08 (the number `n` of a `Tm` patch). */
export const TM_COUNT = 92;
export const MACHINE_COUNT = 100;
export const machineLabel = (n: number): string => (n < TM_COUNT ? `TM${String(n + 1).padStart(2, "0")}` : `VM${String(n - TM_COUNT + 1).padStart(2, "0")}`);

/** The game reads at most 20 level-up moves (pokeplatinum `MAX_LEARNSET_ENTRIES`). */
export const MAX_LEARNSET = 20;

/** A learnset entry by what it is, not where it stands: its move and which entry of that move it is (a list may teach a move twice). */
export interface LearnRef {
  move_id: number;
  nth: number;
}

/** The identity of every entry, in the list's order. */
export function learnRefs(entries: readonly LearnEntryDto[]): LearnRef[] {
  const seen = new Map<number, number>();
  return entries.map((e) => {
    const nth = seen.get(e.move_id) ?? 0;
    seen.set(e.move_id, nth + 1);
    return { move_id: e.move_id, nth };
  });
}

/** The row key of an entry: it follows the entry when a new level sorts it elsewhere. */
export const learnKey = (ref: LearnRef): string => `m${ref.move_id}-${ref.nth}`;

/** Where entry `ref` stands in `entries`; -1 when it is gone. */
export function findLearn(entries: readonly LearnEntryDto[], ref: LearnRef): number {
  let nth = 0;
  for (let i = 0; i < entries.length; i++) {
    if (entries[i]!.move_id !== ref.move_id) continue;
    if (nth === ref.nth) return i;
    nth++;
  }
  return -1;
}

/**
 * Which parts of a species have the wrong length (their fields are locked). The kernel names the first locked part; a lock that names
 * none is taken as one of the personal data.
 */
export function lockedParts(doc: { locked: string | null; locked_asset: AssetId | null }): { personal: boolean; learnset: boolean; evolution: boolean } {
  const kind = doc.locked === null ? null : (doc.locked_asset?.kind ?? "Personal");
  return { personal: kind === "Personal", learnset: kind === "Learnset", evolution: kind === "Evolution" };
}

/** The learnset by level; moves of the same level keep their order (the order the game teaches them in). */
export const sortLearnset = (entries: readonly LearnEntryDto[]): LearnEntryDto[] =>
  entries
    .map((e, i) => [e, i] as const)
    .sort(([a, i], [b, j]) => a.level - b.level || i - j)
    .map(([e]) => e);

/** The tabs of the editor (the ids `NavTarget` names) with their labels; `raw` only while tinkering. */
export const TABS: readonly { id: SpeciesTab; label: string }[] = [
  { id: "values", label: "Werte" },
  { id: "abilities", label: "Fähigkeiten und Typen" },
  { id: "learnset", label: "Lernset" },
  { id: "evolution", label: "Entwicklung" },
  { id: "machines", label: "TM/VM" },
  { id: "raw", label: "Rohdaten" },
];

/** The part of the species each tab edits: its dot comes from that asset's unsaved changes (Rohdaten shows all three). */
export const TAB_ASSETS: Record<SpeciesTab, readonly AssetId["kind"][]> = {
  values: ["Personal"],
  abilities: ["Personal"],
  learnset: ["Learnset"],
  evolution: ["Evolution"],
  machines: ["Personal"],
  raw: ["Personal", "Learnset", "Evolution"],
};


/**
 * The name of entry `id` of a name table: what the text archive says, else `word` with the number ("Attacke 0469"), also for the kernel's
 * English fallbacks ("move 0469") and a placeholder of dashes.
 */
export function tableName(table: readonly string[], id: number, word: string): string {
  const name = table[id]?.trim() ?? "";
  if (name === "" || /^[a-z_]+ \d+$/.test(name) || /^-+$/.test(name)) return `${word} ${id4(id)}`;
  return name;
}

/** An ability by number; 0 is none. */
export const abilityName = (abilities: readonly string[], id: number): string => (id === 0 ? "Keine Fähigkeit" : tableName(abilities, id, "Fähigkeit"));
/** An item by item id; 0 is none. */
export const itemName = (items: readonly string[], id: number): string => (id === 0 ? "Kein Gegenstand" : tableName(items, id, "Gegenstand"));
/** A move by number. */
export const moveName = (moves: readonly string[], id: number): string => tableName(moves, id, "Attacke");

const optionLabel = (options: readonly SelectOption<number>[], value: number) => options.find((o) => o.value === value)?.label ?? `Wert ${value}`;

/** A value of the personal data in words for an undo label ("Normal", "Mittellangsam", "Bedroher"); numbers stay numbers. */
export function personalText(field: PersonalField, value: number, names: Pick<NameTablesDto, "abilities" | "items">): string {
  switch (field) {
    case "type1":
    case "type2":
      return typeName(value);
    case "growth_rate":
      return optionLabel(GROWTH_RATES, value);
    case "gender_ratio":
      return genderText(value);
    case "egg_group1":
    case "egg_group2":
      return optionLabel(EGG_GROUPS, value);
    case "color":
      return optionLabel(COLORS, value);
    case "ability1":
    case "ability2":
      return abilityName(names.abilities, value);
    case "item1":
    case "item2":
      return itemName(names.items, value);
    default:
      return String(value);
  }
}

/** The undo label of a personal field: "Staravia: Angriff 75 → 85", "Staravia: Typ 2 Flug → Feuer". */
export const personalLabel = (name: string, field: PersonalField, old: number, value: number, names: Pick<NameTablesDto, "abilities" | "items">): string =>
  `${name}: ${fieldLabels.species[field]} ${personalText(field, old, names)} → ${personalText(field, value, names)}`;
