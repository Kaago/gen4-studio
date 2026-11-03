import type { DataDocDto, DataPatch, DataRowDto, EncounterMethod, EncounterRowExtra, HeaderSummary, MethodDto, ReplacedByDto, SlotDto } from "../../api";
import type { TableColumn } from "../../ui/primitives";
import { id4 } from "../../workbench/terms";

/**
 * The words of the Begegnungen: tabs, columns, labels and the kernel's English texts in German. Nothing here computes the game's rules:
 * shares, replacements, the species per time of day and the warnings come from the backend's document (`data_get encounter`).
 */

export type EncounterDoc = Extract<DataDocDto, { kind: "encounter" }>;

/** The kernel's time keys of the grass list. */
export type TimeKey = "morning" | "day" | "night";

/** A tab of the hand: a method of the file, or the replacement lists. */
export type EncounterTab = EncounterMethod | "replacements";

const METHOD_WORD: Readonly<Record<EncounterMethod, string>> = {
  grass: "Gras",
  surf: "Surfen",
  rock_smash: "Fels",
  old_rod: "Alte Angel",
  good_rod: "Profi-Angel",
  super_rod: "Super-Angel",
};

/** "Gras", "Profi-Angel"; a method the editor does not know keeps its key. */
export const methodWord = (method: string): string => METHOD_WORD[method as EncounterMethod] ?? method;

const PT_METHODS: readonly EncounterMethod[] = ["grass", "surf", "old_rod", "good_rod", "super_rod"];
/** HeartGold and SoulSilver have Rock Smash (Fels) after Surfen; Platinum's block for it is unused. */
const HGSS_METHODS: readonly EncounterMethod[] = ["grass", "surf", "rock_smash", "old_rod", "good_rod", "super_rod"];

/** The tabs of the hand per family, in game order, and „Ersatz“ (the replacement lists) last. */
export function methodTabs(family: string): { id: EncounterTab; label: string }[] {
  const methods = family === "Hgss" ? HGSS_METHODS : PT_METHODS;
  return [...methods.map((m) => ({ id: m as EncounterTab, label: METHOD_WORD[m] })), { id: "replacements", label: "Ersatz" }];
}

/** The time columns of the grass list: HeartGold has one species column per time of day (`species` is the day one). */
export const TIME_COLUMNS: readonly { when: TimeKey; field: "species_morning" | "species" | "species_night"; label: string; word: string }[] = [
  { when: "morning", field: "species_morning", label: "Morgens", word: "morgens" },
  { when: "day", field: "species", label: "Tags", word: "tags" },
  { when: "night", field: "species_night", label: "Nachts", word: "nachts" },
];

/** „Morgens“, „Tags“, „Nachts“ for the kernel's time keys. */
export const timeLabel = (when: string): string => TIME_COLUMNS.find((c) => c.when === when)?.label ?? when;

/** The hours of a time of day, as the games split them (4–9, 10–19, 20–3). */
export const TIME_HOURS: Readonly<Record<TimeKey, string>> = { morning: "4–9 Uhr", day: "10–19 Uhr", night: "20–3 Uhr" };

/**
 * The columns of a method's table: Platz · Pokémon · Level · Anteil · Ersetzt durch. HeartGold's grass has three species columns
 * (Morgens, Tags, Nachts) and one level column for all three; grass has one level, water a range (min–max).
 */
export function slotColumns(family: string, method: EncounterMethod): TableColumn[] {
  const species: TableColumn[] =
    family === "Hgss" && method === "grass" ? TIME_COLUMNS.map((c) => ({ key: c.field, label: c.label })) : [{ key: "species", label: "Pokémon" }];
  return [
    { key: "slot", label: "Platz", width: 64 },
    ...species,
    { key: "level", label: method === "grass" ? "Level" : "Level (min–max)", width: method === "grass" ? 96 : 168 },
    { key: "share", label: "Anteil", width: 72, align: "end" },
    { key: "replaced", label: "Ersetzt durch" },
  ];
}

/** „Platz 1“ for slot 0 (the kernel counts from 0, the editor from 1). */
export const slotText = (slot: number): string => `Platz ${slot + 1}`;

/** „20 %“. */
export const shareText = (share: number): string => `${share} %`;

/** Whether a slot holds nothing (species and levels 0, in every column): valid, shown muted. */
export function isEmptySlot(slot: SlotDto): boolean {
  return slot.species === 0 && slot.min === 0 && slot.max === 0 && (slot.species_morning ?? 0) === 0 && (slot.species_night ?? 0) === 0;
}

/** How many slots of a method hold something (the tab's count). */
export const usedSlots = (method: MethodDto): number => method.slots.filter((s) => !isEmptySlot(s)).length;

/** How many entries of the replacement lists name a species (the „Ersatz“ tab's count). */
export const usedEntries = (doc: EncounterDoc): number => doc.replacements.reduce((n, l) => n + l.slots.filter((s) => s.species !== 0).length, 0);

/**
 * The German names of the lists the kernel names (as `fieldLabels.encounter` has them; no runtime import of the api here), also those not
 * stored in the file (Trophy Garden, Great Marsh).
 */
const LIST_WORD: Readonly<Record<string, string>> = {
  swarm: "Schwarm",
  day: "Tag",
  night: "Nacht",
  radar: "Poké-Radar",
  dual_ruby: "Dual-Slot Rubin",
  dual_sapphire: "Dual-Slot Saphir",
  dual_emerald: "Dual-Slot Smaragd",
  dual_firered: "Dual-Slot Feuerrot",
  dual_leafgreen: "Dual-Slot Blattgrün",
  radio: "Radio",
  radio_hoenn: "Radio Hoenn",
  radio_sinnoh: "Radio Sinnoh",
  land_swarm: "Schwarm im Gras",
  surf_swarm: "Schwarm beim Surfen",
  night_fish: "Angeln nachts",
  fish_swarm: "Schwarm beim Angeln",
  trophy_garden: "Trophäengarten",
  great_marsh: "Großmoor",
};

/** „Tag“, „Schwarm im Gras“, „Dual-Slot Rubin“; an unknown list keeps its key. */
export const listWord = (list: string): string => LIST_WORD[list] ?? list;

const GBA: Readonly<Record<string, string>> = { Ruby: "Rubin", Sapphire: "Saphir", Emerald: "Smaragd", FireRed: "Feuerrot", LeafGreen: "Blattgrün" };

const WHEN: Readonly<Record<string, string>> = {
  "during a swarm": "bei einem Schwarm",
  "with the Poké Radar": "mit dem Poké-Radar",
  "in the Trophy Garden": "im Trophäengarten",
  "in the Great Marsh during a Safari Game": "im Großmoor beim Safari-Spiel",
  "with the Hoenn sound on the radio": "mit Hoenn-Klängen im Radio",
  "with the Sinnoh sound on the radio": "mit Sinnoh-Klängen im Radio",
  "during a swarm on one of the 20 swarm headers": "bei einem Schwarm (nur an den 20 Schwarm-Orten)",
  "at night": "nachts",
};

/** When the game uses a replacement entry, in German („von 10 bis 19 Uhr“, „mit Rubin im GBA-Slot“); unknown words stay English. */
export function whenText(when: string): string {
  const hours = /^from (\d+) to (\d+) o'clock$/.exec(when);
  if (hours) return `von ${hours[1]} bis ${hours[2]} Uhr`;
  const gba = /^with (\w+) in the Game Boy Advance slot$/.exec(when);
  if (gba) return `mit ${GBA[gba[1]!] ?? gba[1]} im GBA-Slot`;
  return WHEN[when] ?? when;
}

/** „Tag 1 (von 10 bis 19 Uhr)“: one replacement entry that can take a slot's place. */
export const replacedByEntry = (r: ReplacedByDto): string => `${listWord(r.list)} ${r.entry + 1} (${whenText(r.when)})`;

/** The „Ersetzt durch“ cell: every entry that can take the slot's place; empty when none can. */
export const replacedByText = (list: readonly ReplacedByDto[]): string => list.map(replacedByEntry).join(", ");

/** „Gras Platz 3, 4“: the slots a replacement entry takes the place of. */
export function replacesText(targets: readonly { method: string; slot: number }[]): string {
  const byMethod = new Map<string, number[]>();
  for (const t of targets) byMethod.set(t.method, [...(byMethod.get(t.method) ?? []), t.slot + 1]);
  return [...byMethod].map(([m, slots]) => `${methodWord(m)} Platz ${slots.join(", ")}`).join("; ");
}

// ------------------------------------------------------------------------------------------------------------------------- warnings

const NAME = "(grass|surf|rock smash|old rod|good rod|super rod)";
const NAME_WORD: Readonly<Record<string, string>> = { grass: "Gras", surf: "Surfen", "rock smash": "Fels", "old rod": "Alte Angel", "good rod": "Profi-Angel", "super rod": "Super-Angel" };
const TIME_SPECIES: Readonly<Record<string, string>> = { morning: "Die Art für morgens", day: "Die Art für tags", night: "Die Art für nachts" };

/**
 * A rule warning of the kernel (English, 0-based numbers) in German with 1-based places: „Gras Platz 3: Staralili auf Level 0“. `species`
 * names a species number (without names: „Art 0396“). A text in another shape stays as the kernel says it.
 */
export function warningText(text: string, species: (id: number) => string = (id) => `Art ${id4(id)}`): string {
  const name = (key: string) => NAME_WORD[key] ?? key;
  let m = new RegExp(`^${NAME} rate (\\d+) acts like 0: the method never runs$`).exec(text);
  if (m) return `${name(m[1]!)}-Rate ${m[2]} wirkt wie 0: diese Methode löst nie aus (das Spiel liest nur das untere Byte)`;
  m = new RegExp(`^${NAME} rate (\\d+) acts like (\\d+)$`).exec(text);
  if (m) return `${name(m[1]!)}-Rate ${m[2]} wirkt wie ${m[3]} (das Spiel liest nur das untere Byte)`;
  m = new RegExp(`^${NAME} slot (\\d+): species (\\d+) at level 0$`).exec(text);
  if (m) return `${name(m[1]!)} ${slotText(Number(m[2]))}: ${species(Number(m[3]))} auf Level 0`;
  m = new RegExp(`^${NAME} slot (\\d+): level (\\d+) is above 100$`).exec(text);
  if (m) return `${name(m[1]!)} ${slotText(Number(m[2]))}: Level ${m[3]} liegt über 100`;
  m = new RegExp(`^${NAME} slot (\\d+): min level (\\d+) is above max level (\\d+), the game swaps them$`).exec(text);
  if (m) return `${name(m[1]!)} ${slotText(Number(m[2]))}: Mindestlevel ${m[3]} liegt über Höchstlevel ${m[4]}, das Spiel tauscht sie`;
  m = new RegExp(`^${NAME} slot (\\d+): the (morning|day|night) species is empty: (.+) it gives a glitch Pokémon$`).exec(text);
  if (m) return `${name(m[1]!)} ${slotText(Number(m[2]))}: ${TIME_SPECIES[m[3]!]} ist leer, ${whenText(m[4]!)} erscheint dort ein Glitch-Pokémon`;
  m = new RegExp(`^(\\w+) entry (\\d+) is empty: (.+), ${NAME} slot (\\d+) gives a glitch Pokémon$`).exec(text);
  if (m) return `${listWord(m[1]!)} Eintrag ${Number(m[2]) + 1} ist leer: ${whenText(m[3]!)} erscheint auf ${name(m[4]!)} ${slotText(Number(m[5]))} ein Glitch-Pokémon`;
  return text;
}

// ------------------------------------------------------------------------------------------------------------------------- the list

/** The name of a header as the editor shows a place: its location, else its internal name, else „Header 0012“. */
export function headerPlace(headers: readonly HeaderSummary[], id: number): string {
  const h = headers.find((x) => x.id === id);
  return h?.location_name.trim() || h?.internal_name.trim() || `Header ${id4(id)}`;
}

/** The headers whose `wild_pokemon` is the file of `row`; none for a locked row (its users are not read). */
export function rowUsers(row: DataRowDto): readonly number[] {
  const extra = row.extra as EncounterRowExtra | null;
  return extra && Array.isArray(extra.users) ? extra.users : [];
}

/** The places that use the file of `row`, each name once, in header order. */
export function rowPlaces(row: DataRowDto, headers: readonly HeaderSummary[]): string[] {
  return [...new Set(rowUsers(row).map((id) => headerPlace(headers, id)))];
}

/** How many places a label names before it says how many more there are. */
const LABEL_PLACES = 3;

/** An encounter file by the places that use it („Route 201“, „Route 203, Erzelingen +2“), else „Begegnungen 0012“. */
export function encounterRowLabel(row: DataRowDto, headers: readonly HeaderSummary[]): string {
  const places = rowPlaces(row, headers);
  if (places.length === 0) return `Begegnungen ${id4(row.index)}`;
  const shown = places.slice(0, LABEL_PLACES).join(", ");
  return places.length > LABEL_PLACES ? `${shown} +${places.length - LABEL_PLACES}` : shown;
}

/** „verwendet von 1 Ort“, „verwendet von 3 Orten“ (headers that use the file). */
export const usedByText = (n: number): string => `verwendet von ${n} ${n === 1 ? "Ort" : "Orten"}`;

/**
 * A late failure of `split_encounter_file` (code "partial": "encounter file 183 was written as a copy of 141; switching header 401 to it
 * failed: …") in German, with the copy that stays; null for another text.
 */
export function partialSplit(message: string): { copy: number; text: string } | null {
  const m = /^encounter file (\d+) was written as a copy of (\d+); switching header (\d+) to it failed: (.*)$/s.exec(message);
  if (!m) return null;
  const [, copy, file, header, reason] = m;
  return { copy: Number(copy), text: `Begegnungsdatei ${copy} ist als Kopie von ${file} angelegt und bleibt. Danach fehlgeschlagen: Header ${header} auf sie umhängen (${reason}).` };
}

/** The files with a place (in file order), and the group „Ohne Ort“ (unused and locked files); the tombstones (3d.4) only with `showDeleted`. */
export function groupFiles(rows: readonly DataRowDto[], showDeleted = false): { placed: DataRowDto[]; unplaced: DataRowDto[] } {
  const placed: DataRowDto[] = [];
  const unplaced: DataRowDto[] = [];
  for (const row of rows) if (showDeleted || row.deleted !== true) (rowUsers(row).length > 0 ? placed : unplaced).push(row);
  return { placed, unplaced };
}

// ------------------------------------------------------------------------------------------------------------------------- Platinum extras

/** The Unown tables of Platinum (0 to 8; DSPRE `UnownTableNames`). */
export const UNOWN_OPTIONS: readonly { value: number; label: string }[] = [
  { value: 0, label: "Fast alle Formen (0)" },
  { value: 1, label: "Fast alle Formen (1)" },
  { value: 2, label: "Nur F" },
  { value: 3, label: "Nur R" },
  { value: 4, label: "Nur I" },
  { value: 5, label: "Nur N" },
  { value: 6, label: "Nur E" },
  { value: 7, label: "Nur D" },
  { value: 8, label: "! und ?" },
];
export const unownText = (value: number): string => UNOWN_OPTIONS.find((o) => o.value === value)?.label ?? `Wert ${value}`;

/** The two form rates the game reads: [0] Schalellos, [1] Gastrodon; 0 is West, anything else Ost (the games write 100). */
export const FORM_RATES: readonly { field: "form_rate_0" | "form_rate_1"; index: 0 | 1; label: string }[] = [
  { field: "form_rate_0", index: 0, label: "Schalellos" },
  { field: "form_rate_1", index: 1, label: "Gastrodon" },
];
/** What „Ost“ writes (the games' own value). */
export const EAST = 100;
export const formText = (rate: number): string => (rate === 0 ? "West" : "Ost");

// ------------------------------------------------------------------------------------------------------------------------- labels

/** „Route 201: Gras-Rate 30 → 100“. */
export const rateLabel = (title: string, method: string, old: number, next: number): string => `${title}: ${methodWord(method)}-Rate ${old} → ${next}`;

/** „Route 201: Gras Platz 1 Bidiza → Staralili“, „Route 201: Surfen Platz 2 Mindestlevel 20 → 30“. */
export const slotLabel = (title: string, method: string, slot: number, rest: string): string => `${title}: ${methodWord(method)} ${slotText(slot)} ${rest}`;

/** „Route 201: Tag Eintrag 1 Bidiza → Staralili“. */
export const entryLabel = (title: string, list: string, entry: number, rest: string): string => `${title}: ${listWord(list)} Eintrag ${entry + 1} ${rest}`;

/** „Route 201: Gras Platz 2 ↔ 7“: two slots trade species and levels. */
export const swapLabel = (title: string, method: string, a: number, b: number): string => `${title}: ${methodWord(method)} Platz ${a + 1} ↔ ${b + 1}`;

/** The word of a slot field in a label. */
export const FIELD_WORD: Readonly<Record<string, string>> = { level: "Level", min: "Mindestlevel", max: "Höchstlevel", species: "", species_morning: "morgens", species_night: "nachts" };

// ------------------------------------------------------------------------------------------------------------------------- the document as edited

const setSlot = (slots: readonly SlotDto[], index: number, change: (s: SlotDto) => SlotDto): SlotDto[] => slots.map((s) => (s.index === index ? change(s) : s));

/** A slot with `field` written (water levels pulled along as the backend does: min over max raises max, max under min lowers min). */
function writeSlot(slot: SlotDto, field: string, value: number): SlotDto {
  switch (field) {
    case "species":
    case "species_morning":
    case "species_night":
      return { ...slot, [field]: value };
    case "level":
      return { ...slot, min: value, max: value };
    case "min":
      return { ...slot, min: value, max: Math.max(slot.max, value) };
    case "max":
      return { ...slot, max: value, min: Math.min(slot.min, value) };
    default:
      return slot;
  }
}

/**
 * `doc` with `patch` written, for what the editor shows until the document is read again (`DocSpec.apply`). Only the written values: the
 * shares, warnings and who waits stay as last read until the backend's answer comes.
 */
export function applyEncounter(doc: EncounterDoc, patch: DataPatch): EncounterDoc {
  if ("Encounter" in patch) {
    const { method, slot, field, value } = patch.Encounter;
    if (doc.methods.some((m) => m.method === method)) {
      return {
        ...doc,
        methods: doc.methods.map((m) => {
          if (m.method !== method) return m;
          if (field === "rate") return { ...m, rate: value };
          return slot === null || slot === undefined ? m : { ...m, slots: setSlot(m.slots, slot, (s) => writeSlot(s, field, value)) };
        }),
      };
    }
    return {
      ...doc,
      replacements: doc.replacements.map((l) => (l.list !== method ? l : { ...l, slots: setSlot(l.slots, slot ?? 0, (s) => writeSlot(s, field, value)) })),
    };
  }
  if ("EncounterSwap" in patch) {
    // Species and levels trade places; the share, the replacements and the slot number belong to the place and stay.
    const { method, a, b } = patch.EncounterSwap;
    const traded = (s: SlotDto) => ({ species: s.species, min: s.min, max: s.max, species_morning: s.species_morning, species_night: s.species_night });
    return {
      ...doc,
      methods: doc.methods.map((m) => {
        const sa = m.slots.find((s) => s.index === a);
        const sb = m.slots.find((s) => s.index === b);
        if (m.method !== method || sa === undefined || sb === undefined || a === b) return m;
        return { ...m, slots: m.slots.map((s) => (s.index === a ? { ...s, ...traded(sb) } : s.index === b ? { ...s, ...traded(sa) } : s)) };
      }),
    };
  }
  if ("EncounterExtra" in patch) {
    const { field, value } = patch.EncounterExtra;
    if (field === "unown_table") return { ...doc, extras: { ...doc.extras, unown_table: value } };
    const n = /^form_rate_([0-4])$/.exec(field);
    const rates = doc.extras.form_rates;
    if (!n || rates === null) return doc;
    return { ...doc, extras: { ...doc.extras, form_rates: rates.map((r, i) => (i === Number(n[1]) ? value : r)) as typeof rates } };
  }
  return doc;
}
