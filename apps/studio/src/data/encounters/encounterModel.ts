import type { DataDocDto, DataPatch, DataRowDto, EncounterMethod, EncounterRowExtra, HeaderSummary, MethodDto, ReplacedByDto, SlotDto } from "../../api";
import { t } from "../../i18n";
import type { TableColumn } from "../../ui/primitives";
import { id4, termLabel } from "../../workbench/terms";

/**
 * The words of the Begegnungen: tabs, columns, labels and the kernel's English texts in German. Nothing here computes the game's rules:
 * shares, replacements, the species per time of day and the warnings come from the backend's document (`data_get encounter`).
 */

export type EncounterDoc = Extract<DataDocDto, { kind: "encounter" }>;

/** The kernel's time keys of the grass list. */
export type TimeKey = "morning" | "day" | "night";

/** A tab of the hand: a method of the file, or the replacement lists. */
export type EncounterTab = EncounterMethod | "replacements";

type MethodName = "grass" | "surf" | "rockSmash" | "oldRod" | "goodRod" | "superRod";

/** The catalog key of each method's word, by the kernel's name for it. */
const METHOD_NAME: Readonly<Record<EncounterMethod, MethodName>> = { grass: "grass", surf: "surf", rock_smash: "rockSmash", old_rod: "oldRod", good_rod: "goodRod", super_rod: "superRod" };

const methodName = (name: MethodName): string => t(`data:encounterModel.methods.${name}`);

/** "Gras", "Profi-Angel"; a method the editor does not know keeps its key. */
export const methodWord = (method: string): string => {
  const name = METHOD_NAME[method as EncounterMethod];
  return name === undefined ? method : methodName(name);
};

const PT_METHODS: readonly EncounterMethod[] = ["grass", "surf", "old_rod", "good_rod", "super_rod"];
/** HeartGold and SoulSilver have Rock Smash (Fels) after Surfen; Platinum's block for it is unused. */
const HGSS_METHODS: readonly EncounterMethod[] = ["grass", "surf", "rock_smash", "old_rod", "good_rod", "super_rod"];

/** The tabs of the hand per family, in game order, and „Ersatz“ (the replacement lists) last. */
export function methodTabs(family: string): { id: EncounterTab; label: string }[] {
  const methods = family === "Hgss" ? HGSS_METHODS : PT_METHODS;
  return [...methods.map((m) => ({ id: m as EncounterTab, label: methodName(METHOD_NAME[m]) })), { id: "replacements", label: t("data:encounterModel.replacements") }];
}

/** The time columns of the grass list: HeartGold has one species column per time of day (`species` is the day one). */
export const TIME_COLUMNS: readonly { when: TimeKey; field: "species_morning" | "species" | "species_night"; readonly label: string; readonly word: string }[] = [
  timeColumn("morning", "species_morning"),
  timeColumn("day", "species"),
  timeColumn("night", "species_night"),
];

/** One time of day; `label` and `word` are read when asked for (in the language shown then), never when this module loads. */
function timeColumn(when: TimeKey, field: "species_morning" | "species" | "species_night") {
  return {
    when,
    field,
    get label() {
      return t(`data:encounterModel.times.${when}.label`);
    },
    get word() {
      return t(`data:encounterModel.times.${when}.word`);
    },
  };
}

/** „Morgens“, „Tags“, „Nachts“ for the kernel's time keys. */
export const timeLabel = (when: string): string => TIME_COLUMNS.find((c) => c.when === when)?.label ?? when;

/** The hours of a time of day, as the games split them (4–9, 10–19, 20–3). */
export const TIME_HOURS: Readonly<Record<TimeKey, string>> = {
  get morning() {
    return t("data:encounterModel.hours.morning");
  },
  get day() {
    return t("data:encounterModel.hours.day");
  },
  get night() {
    return t("data:encounterModel.hours.night");
  },
};

/**
 * The columns of a method's table: Platz · Pokémon · Level · Anteil · Ersetzt durch. HeartGold's grass has three species columns
 * (Morgens, Tags, Nachts) and one level column for all three; grass has one level, water a range (min–max).
 */
export function slotColumns(family: string, method: EncounterMethod): TableColumn[] {
  const species: TableColumn[] =
    family === "Hgss" && method === "grass" ? TIME_COLUMNS.map((c) => ({ key: c.field, label: c.label })) : [{ key: "species", label: t("data:encounterModel.columns.pokemon") }];
  return [
    { key: "slot", label: t("data:encounterModel.columns.slot"), width: 64 },
    ...species,
    { key: "level", label: method === "grass" ? t("data:encounterModel.columns.level") : t("data:encounterModel.columns.levelRange"), width: method === "grass" ? 96 : 168 },
    { key: "share", label: t("data:encounterModel.columns.share"), width: 72, align: "end" },
    { key: "replaced", label: t("data:encounterModel.columns.replaced") },
  ];
}

/** „Platz 1“ for slot 0 (the kernel counts from 0, the editor from 1). */
export const slotText = (slot: number): string => t("data:encounterModel.slot", { n: slot + 1 });

/** „20 %“. */
export const shareText = (share: number): string => t("data:percent", { value: share });

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
const LIST_NAME = {
  swarm: "swarm",
  day: "day",
  night: "night",
  radar: "radar",
  dual_ruby: "dualRuby",
  dual_sapphire: "dualSapphire",
  dual_emerald: "dualEmerald",
  dual_firered: "dualFireRed",
  dual_leafgreen: "dualLeafGreen",
  radio: "radio",
  radio_hoenn: "radioHoenn",
  radio_sinnoh: "radioSinnoh",
  land_swarm: "landSwarm",
  surf_swarm: "surfSwarm",
  night_fish: "nightFish",
  fish_swarm: "fishSwarm",
  trophy_garden: "trophyGarden",
  great_marsh: "greatMarsh",
} as const;

/** „Tag“, „Schwarm im Gras“, „Dual-Slot Rubin“; an unknown list keeps its key. */
export const listWord = (list: string): string => {
  const name = (LIST_NAME as Readonly<Record<string, (typeof LIST_NAME)[keyof typeof LIST_NAME]>>)[list];
  return name === undefined ? list : t(`data:encounterModel.lists.${name}`);
};

const GBA = { Ruby: "ruby", Sapphire: "sapphire", Emerald: "emerald", FireRed: "fireRed", LeafGreen: "leafGreen" } as const;

/** The kernel's English phrases for when an entry applies, to the key of their words in the catalog. */
const WHEN = {
  "during a swarm": "swarm",
  "with the Poké Radar": "radar",
  "in the Trophy Garden": "trophyGarden",
  "in the Great Marsh during a Safari Game": "greatMarsh",
  "with the Hoenn sound on the radio": "radioHoenn",
  "with the Sinnoh sound on the radio": "radioSinnoh",
  "during a swarm on one of the 20 swarm headers": "swarmHeaders",
  "at night": "night",
} as const;

/** When the game uses a replacement entry, in German („von 10 bis 19 Uhr“, „mit Rubin im GBA-Slot“); unknown words stay English. */
export function whenText(when: string): string {
  const hours = /^from (\d+) to (\d+) o'clock$/.exec(when);
  if (hours) return t("data:encounterModel.when.hours", { from: hours[1], to: hours[2] });
  const gba = /^with (\w+) in the Game Boy Advance slot$/.exec(when);
  if (gba) {
    const game = (GBA as Readonly<Record<string, (typeof GBA)[keyof typeof GBA]>>)[gba[1]!];
    return t("data:encounterModel.when.gba", { game: game === undefined ? gba[1] : t(`data:encounterModel.games.${game}`) });
  }
  const known = (WHEN as Readonly<Record<string, (typeof WHEN)[keyof typeof WHEN]>>)[when];
  return known === undefined ? when : t(`data:encounterModel.when.${known}`);
}

/** „Tag 1 (von 10 bis 19 Uhr)“: one replacement entry that can take a slot's place. */
export const replacedByEntry = (r: ReplacedByDto): string => `${listWord(r.list)} ${r.entry + 1} (${whenText(r.when)})`;

/** The „Ersetzt durch“ cell: every entry that can take the slot's place; empty when none can. */
export const replacedByText = (list: readonly ReplacedByDto[]): string => list.map(replacedByEntry).join(", ");

/** „Gras Platz 3, 4“: the slots a replacement entry takes the place of. */
export function replacesText(targets: readonly { method: string; slot: number }[]): string {
  const byMethod = new Map<string, number[]>();
  for (const t of targets) byMethod.set(t.method, [...(byMethod.get(t.method) ?? []), t.slot + 1]);
  return [...byMethod].map(([m, slots]) => t("data:encounterModel.replaces", { method: methodWord(m), slots: slots.join(", ") })).join("; ");
}

// ------------------------------------------------------------------------------------------------------------------------- warnings

const NAME = "(grass|surf|rock smash|old rod|good rod|super rod)";
const NAME_WORD: Readonly<Record<string, MethodName>> = { grass: "grass", surf: "surf", "rock smash": "rockSmash", "old rod": "oldRod", "good rod": "goodRod", "super rod": "superRod" };

/**
 * A rule warning of the kernel (English, 0-based numbers) in German with 1-based places: „Gras Platz 3: Staralili auf Level 0“. `species`
 * names a species number (without names: „Art 0396“). A text in another shape stays as the kernel says it.
 */
export function warningText(text: string, species: (id: number) => string = (id) => t("data:speciesFallback", { id: id4(id) })): string {
  const name = (key: string) => (NAME_WORD[key] === undefined ? key : methodName(NAME_WORD[key]));
  let m = new RegExp(`^${NAME} rate (\\d+) acts like 0: the method never runs$`).exec(text);
  if (m) return t("data:encounterModel.warnings.rateZero", { name: name(m[1]!), rate: m[2] });
  m = new RegExp(`^${NAME} rate (\\d+) acts like (\\d+)$`).exec(text);
  if (m) return t("data:encounterModel.warnings.rateLike", { name: name(m[1]!), rate: m[2], like: m[3] });
  m = new RegExp(`^${NAME} slot (\\d+): species (\\d+) at level 0$`).exec(text);
  if (m) return t("data:encounterModel.warnings.levelZero", { name: name(m[1]!), slot: slotText(Number(m[2])), species: species(Number(m[3])) });
  m = new RegExp(`^${NAME} slot (\\d+): level (\\d+) is above 100$`).exec(text);
  if (m) return t("data:encounterModel.warnings.levelAbove100", { name: name(m[1]!), slot: slotText(Number(m[2])), level: m[3] });
  m = new RegExp(`^${NAME} slot (\\d+): min level (\\d+) is above max level (\\d+), the game swaps them$`).exec(text);
  if (m) return t("data:encounterModel.warnings.minAboveMax", { name: name(m[1]!), slot: slotText(Number(m[2])), min: m[3], max: m[4] });
  m = new RegExp(`^${NAME} slot (\\d+): the (morning|day|night) species is empty: (.+) it gives a glitch Pokémon$`).exec(text);
  if (m) return t("data:encounterModel.warnings.timeEmpty", { name: name(m[1]!), slot: slotText(Number(m[2])), which: t(`data:encounterModel.warnings.species.${m[3] as TimeKey}`), when: whenText(m[4]!) });
  m = new RegExp(`^(\\w+) entry (\\d+) is empty: (.+), ${NAME} slot (\\d+) gives a glitch Pokémon$`).exec(text);
  if (m) return t("data:encounterModel.warnings.entryEmpty", { list: listWord(m[1]!), entry: Number(m[2]) + 1, when: whenText(m[3]!), name: name(m[4]!), slot: slotText(Number(m[5])) });
  return text;
}

// ------------------------------------------------------------------------------------------------------------------------- the list

/** The name of a header as the editor shows a place: its location, else its internal name, else „Header 0012“. */
export function headerPlace(headers: readonly HeaderSummary[], id: number): string {
  const h = headers.find((x) => x.id === id);
  return h?.location_name.trim() || h?.internal_name.trim() || `${termLabel("header")} ${id4(id)}`;
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
  if (places.length === 0) return `${t("data:dataModel.fallbackWord.encounter")} ${id4(row.index)}`;
  const shown = places.slice(0, LABEL_PLACES).join(", ");
  return places.length > LABEL_PLACES ? `${shown} +${places.length - LABEL_PLACES}` : shown;
}

/** „verwendet von 1 Ort“, „verwendet von 3 Orten“ (headers that use the file). */
export const usedByText = (n: number): string => t("data:encounterModel.usedBy", { count: n });

/**
 * A late failure of `split_encounter_file` (code "partial": "encounter file 183 was written as a copy of 141; switching header 401 to it
 * failed: …") in German, with the copy that stays; null for another text.
 */
export function partialSplit(message: string): { copy: number; text: string } | null {
  const m = /^encounter file (\d+) was written as a copy of (\d+); switching header (\d+) to it failed: (.*)$/s.exec(message);
  if (!m) return null;
  const [, copy, file, header, reason] = m;
  return { copy: Number(copy), text: t("data:encounterModel.partialSplit", { copy, file, header, reason }) };
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
export const UNOWN_OPTIONS: readonly { value: number; readonly label: string }[] = (["almostAll0", "almostAll1", "onlyF", "onlyR", "onlyI", "onlyN", "onlyE", "onlyD", "marks"] as const).map((name, value) => ({
  value,
  // Read when asked for (in the language shown then), never when this module loads.
  get label() {
    return t(`data:encounterModel.unown.${name}`);
  },
}));
export const unownText = (value: number): string => UNOWN_OPTIONS.find((o) => o.value === value)?.label ?? t("data:value", { value });

/** The two form rates the game reads: [0] Schalellos, [1] Gastrodon; 0 is West, anything else Ost (the games write 100). */
export const FORM_RATES: readonly { field: "form_rate_0" | "form_rate_1"; index: 0 | 1; readonly label: string }[] = [
  {
    field: "form_rate_0",
    index: 0,
    get label() {
      return t("data:encounterModel.formRates.shellos");
    },
  },
  {
    field: "form_rate_1",
    index: 1,
    get label() {
      return t("data:encounterModel.formRates.gastrodon");
    },
  },
];
/** What „Ost“ writes (the games' own value). */
export const EAST = 100;
export const formText = (rate: number): string => (rate === 0 ? t("data:encounterModel.west") : t("data:encounterModel.east"));

// ------------------------------------------------------------------------------------------------------------------------- labels

/** „Route 201: Gras-Rate 30 → 100“. */
export const rateLabel = (title: string, method: string, old: number, next: number): string => t("data:encounterModel.rateLabel", { title, method: methodWord(method), from: old, to: next });

/** „Route 201: Gras Platz 1 Bidiza → Staralili“, „Route 201: Surfen Platz 2 Mindestlevel 20 → 30“. */
export const slotLabel = (title: string, method: string, slot: number, rest: string): string => `${title}: ${methodWord(method)} ${slotText(slot)} ${rest}`;

/** „Route 201: Tag Eintrag 1 Bidiza → Staralili“. */
export const entryLabel = (title: string, list: string, entry: number, rest: string): string => t("data:encounterModel.entryLabel", { title, list: listWord(list), entry: entry + 1, rest });

/** „Route 201: Gras Platz 2 ↔ 7“: two slots trade species and levels. */
export const swapLabel = (title: string, method: string, a: number, b: number): string => t("data:encounterModel.swapLabel", { title, method: methodWord(method), a: a + 1, b: b + 1 });

/** The word of a slot field in a label. */
export const FIELD_WORD: Readonly<Record<string, string>> = {
  get level() {
    return t("data:encounterModel.fieldWords.level");
  },
  get min() {
    return t("data:encounterModel.fieldWords.min");
  },
  get max() {
    return t("data:encounterModel.fieldWords.max");
  },
  species: "",
  get species_morning() {
    return t("data:encounterModel.times.morning.word");
  },
  get species_night() {
    return t("data:encounterModel.times.night.word");
  },
};

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
