import type { DataDocDto, DataPatch, DataRowDto, LearnEntryDto, PartyFormatDto, PartyMonDto, ProblemDto, ProjectInfo, TrainerRowExtra } from "../../api";
import type { SegmentedOption, SelectOption } from "../../ui/primitives";
import { id4 } from "../../workbench/terms";

// No runtime import of the API or the Pokédex model: `dataModel` (and with it the shortcuts) reads this module.

/** The document of a trainer: properties, party, name, battle texts, the events that start its battle, its Problems rows. */
export type TrainerDoc = Extract<DataDocDto, { kind: "trainer" }>;

/** The kernel's lock reason of the placeholder of the sight script slot (`store.rs` `PLACEHOLDER_REASON`), and what the editor says. */
export const PLACEHOLDER_REASON = "placeholder for the sight script slot";
export const PLACEHOLDER_TEXT = "Platzhalter für das Sicht-Script";

/** The longest name the game copies without an arm9 patch (8 UTF-16 units with the terminator), and the refusal of a longer one. */
export const NAME_MAX = 7;
export const NAME_TOO_LONG = "Höchstens 7 Zeichen; längere Namen brauchen einen arm9-Patch";

/** The most Pokémon a team holds. */
export const PARTY_MAX = 6;

// ---------------------------------------------------------------------------------------------------------------------------- AI

export type AiLevel = "simple" | "smart" | "expert" | "custom";

/** The AI presets of Bauen; `eigene` is active when the value is none of them (it cannot be picked: Tüfteln sets the bits). */
export const AI_LEVELS: readonly { value: AiLevel; label: string; ai: number | null }[] = [
  { value: "simple", label: "einfach", ai: 0x1 },
  { value: "smart", label: "klug", ai: 0x7 },
  { value: "expert", label: "Experte", ai: 0xf },
  { value: "custom", label: "eigene", ai: null },
];

export const AI_OPTIONS: readonly SegmentedOption<AiLevel>[] = AI_LEVELS.map((l) => ({ value: l.value, label: l.label, disabled: l.ai === null }));

/** The preset an AI value is, else `custom`. */
export const aiLevel = (ai: number): AiLevel => AI_LEVELS.find((l) => l.ai === ai)?.value ?? "custom";

/** An AI value in words for an undo label: the preset, else "eigene 0x207". */
export function aiText(ai: number): string {
  const level = AI_LEVELS.find((l) => l.ai === ai);
  return level ? level.label : `eigene 0x${(ai >>> 0).toString(16)}`;
}

/** The AI bits DSPRE names (`AI_COUNT = 11`), in German; the other 21 of the 32 bits stay numbers and are kept. */
export const AI_BITS: readonly string[] = [
  "Grundlagen",
  "Angriffe bewerten",
  "Experte",
  "Statuswerte aufbauen",
  "Riskant",
  "Extreme bevorzugen",
  "Staffette",
  "Doppelkampf-Taktik",
  "KP prüfen",
  "Wetter",
  "Schikane",
];

/** "Bit 0: Grundlagen"; an unnamed bit "Bit 31". */
export const bitLabel = (bit: number): string => (AI_BITS[bit] === undefined ? `Bit ${bit}` : `Bit ${bit}: ${AI_BITS[bit]}`);

/** `value` (an unsigned 32-bit word) with `bit` set or cleared, every other bit kept. */
export function withBit(value: number, bit: number, on: boolean): number {
  const mask = (1 << bit) >>> 0;
  return (on ? (value | mask) : (value & ~mask)) >>> 0;
}

// ---------------------------------------------------------------------------------------------------------------------------- battle type

export type BattleKind = "single" | "double";
export const BATTLE_OPTIONS: readonly SegmentedOption<BattleKind>[] = [
  { value: "single", label: "Einzel" },
  { value: "double", label: "Doppel" },
];

/** A double battle is bit 1 of `battle_type` (vanilla holds 0 and 2); the other bits stay as they are. */
export const isDouble = (battleType: number): boolean => (battleType & 2) !== 0;
export const withDouble = (battleType: number, on: boolean): number => withBit(battleType, 1, on);
export const battleText = (battleType: number): string => (isDouble(battleType) ? "Doppel" : "Einzel");

// ---------------------------------------------------------------------------------------------------------------------------- party

/** HeartGold: bits 0 to 3 of byte 1. */
export const GENDER_OPTIONS: readonly SelectOption<number>[] = [
  { value: 0, label: "Auto" },
  { value: 1, label: "männlich" },
  { value: 2, label: "weiblich" },
];

/** HeartGold: bits 4 to 7 of byte 1. */
export const ABILITY_OPTIONS: readonly SelectOption<number>[] = [
  { value: 0, label: "Auto" },
  { value: 1, label: "Fähigkeit 1" },
  { value: 2, label: "Fähigkeit 2" },
];

export const optionText = (options: readonly SelectOption<number>[], value: number): string => options.find((o) => o.value === value)?.label ?? `Wert ${value}`;

/** The gender of a HeartGold party Pokémon: the DTO's field, else the low nibble of byte 1. */
export const genderOf = (mon: PartyMonDto): number => mon.gender ?? (mon.byte1 & 0x0f);
/** The ability of a HeartGold party Pokémon: the DTO's field, else the high nibble of byte 1. */
export const abilityOf = (mon: PartyMonDto): number => mon.ability ?? (mon.byte1 >> 4);

const isHgss = (family: string) => family === "Hgss";

/**
 * The strength field of a party Pokémon. Platinum reads both bytes as one IV scale (byte 1 the upper byte, almost always 0 in vanilla);
 * HeartGold reads byte 0 as the difficulty and byte 1 as gender and ability.
 */
export function strengthField(family: string): { label: string; bits: 8 | 16; hint: string } {
  return isHgss(family) ? { label: "Stärke", bits: 8, hint: "IV = Wert × 31 / 255" } : { label: "Stärke (IV-Skala)", bits: 16, hint: "IV = Wert × 31 / 255; Byte 1 ist das obere Byte" };
}

export const strengthOf = (mon: PartyMonDto, family: string): number => (isHgss(family) ? mon.iv_or_difficulty : mon.iv_or_difficulty | (mon.byte1 << 8));

/** `mon` with strength `value`: HeartGold keeps byte 1 (gender and ability), Platinum splits the value over both bytes. */
export function withStrength(mon: PartyMonDto, family: string, value: number): PartyMonDto {
  if (isHgss(family)) return { ...mon, iv_or_difficulty: value & 0xff };
  return { ...mon, iv_or_difficulty: value & 0xff, byte1: (value >> 8) & 0xff };
}

/** The species with forms and how many (Pichu's spiky ear is HeartGold's); the form field shows only for them, or for a form already set. */
const FORM_COUNT: Readonly<Record<number, number>> = { 172: 2, 201: 28, 351: 4, 386: 4, 412: 3, 413: 3, 421: 2, 422: 2, 423: 2, 479: 6, 487: 2, 492: 2, 493: 17 };
export const formCount = (species: number): number => FORM_COUNT[species] ?? 1;

/**
 * The moves the game gives a party Pokémon without own moves (pret `TrainerData_BuildParty`, the kernel's `default_moves`): the learnset
 * in file order up to `level`; a known move is skipped, a new one takes the first empty slot, with four full the first one goes.
 */
export function defaultMoves(learnset: readonly LearnEntryDto[], level: number): [number, number, number, number] {
  const moves: [number, number, number, number] = [0, 0, 0, 0];
  for (const entry of learnset) {
    if (entry.level > level) break;
    const id = entry.move_id;
    let added = false;
    for (let slot = 0; slot < 4; slot++) {
      if (moves[slot] === 0) {
        moves[slot] = id;
        added = true;
        break;
      }
      if (moves[slot] === id) {
        added = true;
        break;
      }
    }
    if (!added) {
      moves.shift();
      moves.push(id);
    }
  }
  return moves;
}

/** A new party Pokémon in the trainer's format: strength 0, form 0, item 0 and the given moves where the format has them. */
export function newMon(species: number, level: number, format: PartyFormatDto, family: string, moves?: [number, number, number, number]): PartyMonDto {
  const hgss = isHgss(family);
  return {
    iv_or_difficulty: 0,
    byte1: 0,
    level,
    species,
    form: 0,
    item: format.items ? 0 : null,
    moves: format.moves ? (moves ?? [0, 0, 0, 0]) : null,
    ball_capsule: family === "Dp" ? null : 0,
    gender: hgss ? 0 : null,
    ability: hgss ? 0 : null,
  };
}

/** `party` with the Pokémon at `slot` replaced. */
export const withMon = (party: readonly PartyMonDto[], slot: number, mon: PartyMonDto): PartyMonDto[] => party.map((m, i) => (i === slot ? mon : m));

/** "Wanderer Jens: Platz 1 Level 12 → 14". */
export const slotLabel = (title: string, slot: number, rest: string): string => `${title}: Platz ${slot + 1} ${rest}`;

// ---------------------------------------------------------------------------------------------------------------------------- names

/** The class name from the class bank; a missing one, the kernel's fallback and dashes: "Klasse 130" (the backend's words). */
export function className(classes: readonly string[], cls: number): string {
  const name = classes[cls]?.trim() ?? "";
  if (name === "" || /^class \d+$/.test(name) || /^-+$/.test(name)) return `Klasse ${cls}`;
  return name;
}

/** The classes of the class bank as choices; a class beyond the bank (`current`) is offered as "Klasse 130", like the backend names it. */
export function classOptions(classes: readonly string[], current: number): SelectOption<number>[] {
  const options = classes.map((_, i) => ({ value: i, label: className(classes, i) }));
  return current < classes.length ? options : [...options, { value: current, label: className(classes, current) }];
}

/** A trainer's own name: empty for the kernel's fallback ("trainer 0005"). */
export const ownName = (name: string, index: number): string => (name.trim() === `trainer ${id4(index)}` ? "" : name.trim());

/** "Wanderer Jens", as the game says it before a battle; a trainer without a name "Trainer 0012" (the backend's undo labels do the same). */
export const trainerTitle = (cls: string, name: string, index: number): string => (name.trim() === "" ? `Trainer ${id4(index)}` : `${cls} ${name.trim()}`);

/** The title of a trainer row; a locked row has no class and is named by its name or number. */
export function trainerRowLabel(row: DataRowDto): string {
  const own = ownName(row.name, row.index);
  const extra = row.extra as TrainerRowExtra | null;
  if (!extra || typeof extra.class_name !== "string") return own === "" ? `Trainer ${id4(row.index)}` : own;
  return trainerTitle(extra.class_name, own, row.index);
}

/** The title of a trainer document, for undo labels: from the document as the backend holds it. */
export const docTitle = (doc: TrainerDoc, classes: readonly string[], index: number): string => trainerTitle(className(classes, doc.class), doc.name, index);

/** Why a name cannot be written; null when it can. */
export const nameError = (name: string): string | null => (name.length > NAME_MAX ? NAME_TOO_LONG : null);

/** "5 von 7". */
export const nameCount = (name: string): string => `${name.length} von ${NAME_MAX}`;

/** A species name for cards and labels; without one, the kernel's fallback or dashes: "Art 0520" (as the Pokédex's `tableName`). */
export function speciesName(species: readonly string[], id: number): string {
  const name = species[id]?.trim() ?? "";
  if (name === "" || /^[a-z_]+ \d+$/.test(name) || /^-+$/.test(name)) return `Art ${id4(id)}`;
  return name;
}

/**
 * The text archive of the trainer names (`text_banks.yaml` `trainer_names`, mirrored): Platinum 618, HeartGold 729, Diamond/Pearl 559 for
 * the Western ROMs; null for Japanese and Korean ones, whose numbers are unverified (the trainer list then reads again on any text change).
 */
export function trainerNameArchive(project: ProjectInfo | null): number | null {
  if (project === null || project.language === "ja" || project.language === "ko") return null;
  return ({ Pt: 618, Hgss: 729, Dp: 559 } as Record<string, number>)[project.family] ?? null;
}

// ---------------------------------------------------------------------------------------------------------------------------- creating

/**
 * The shared battle script per family (`trainer_scripts.yaml`, mirrored): its script file, and the trainer ID of its "seen" slot
 * (`seen_script − 2999`), which `create_trainer` fills with a locked placeholder. The kernel prefers the battle file of the game's
 * script table where it has one; the vanilla games agree with these numbers.
 */
export const TRAINER_SCRIPTS: Readonly<Record<string, { battleFile: number; seenSlot: number }>> = {
  Pt: { battleFile: 1114, seenSlot: 929 },
  Hgss: { battleFile: 953, seenSlot: 740 },
};

/** The ID `create_trainer` gives the next trainer: after the last one, past the seen slot (then `placeholder` is written first). */
export function nextTrainer(rows: readonly DataRowDto[], family: string): { id: number; placeholder: number | null } {
  const next = rows.length === 0 ? 0 : Math.max(...rows.map((r) => r.index)) + 1;
  const seen = TRAINER_SCRIPTS[family]?.seenSlot;
  return next === seen ? { id: next + 1, placeholder: next } : { id: next, placeholder: null };
}

const PARTIAL_STEPS: Readonly<Record<string, string>> = {
  "reloading a text archive its compile changed": "Textarchiv neu laden",
  "writing the placeholder": "Platzhalter schreiben",
  "writing the trainer": "Trainer schreiben",
  "setting its name and text offset": "Name und Text-Offset setzen",
  "extending the battle slots": "Kampf-Plätze erweitern",
};

/**
 * A late failure of `create_trainer` (code "partial": "creating trainer 930: battle script file 1114 has the slots up to trainer 930,
 * trainer 929 was written; writing the trainer failed: …") in German, with the trainers written and whether the battle script changed
 * (it got its slots, or "battle script file 1114 stays changed": a failed extension could not put the old script back). Another text
 * stays as it is.
 */
export function partialTrainer(message: string): { text: string; written: number[]; extended: boolean } {
  const m = /^creating trainer (\d+): (.*); (.*?) failed: (.*)$/s.exec(message);
  if (!m) return { text: message, written: [], extended: false };
  const [, id, done, step, reason] = m;
  const written = [...done!.matchAll(/trainer (\d+) was written/g)].map((w) => Number(w[1]));
  const script = /battle script file (\d+) has the slots up to trainer (\d+)/.exec(done!);
  const stuck = /battle script file (\d+) stays changed/.exec(done!);
  const parts: string[] = [];
  if (script) parts.push(`Kampf-Script ${script[1]} hat die Plätze bis Trainer ${script[2]}`);
  if (stuck) parts.push(`Kampf-Script ${stuck[1]} bleibt geändert`);
  if (written.length === 1) parts.push(`Trainer ${written[0]} ist geschrieben`);
  else if (written.length > 1) parts.push(`Trainer ${written.slice(0, -1).join(", ")} und ${written[written.length - 1]} sind geschrieben`);
  const text = `Trainer ${id} nur zum Teil angelegt: ${parts.join(", ")}. Danach fehlgeschlagen: ${PARTIAL_STEPS[step!] ?? step} (${reason}).`;
  return { text, written, extended: script !== null || stuck !== null };
}

// ---------------------------------------------------------------------------------------------------------------------------- texts

/** The battle texts a single trainer needs, by kind (`trainer_message_types.txt`): `label` for the field, `word` for the undo label. */
export const TEXT_KINDS: readonly { kind: number; label: string; word: string }[] = [
  { kind: 0, label: "Vor dem Kampf", word: "vor dem Kampf" },
  { kind: 1, label: "Niederlage", word: "Niederlage" },
  { kind: 2, label: "Danach", word: "danach" },
];

export const textKind = (kind: number): { kind: number; label: string; word: string } => TEXT_KINDS.find((k) => k.kind === kind) ?? { kind, label: `Kampftext Art ${kind}`, word: `Art ${kind}` };

/** The kernel's English warnings in German with a verb; another one stays as it is. */
export const isMarkProblem = (row: Pick<ProblemDto, "message">): boolean => /the text before the battle does not end with \\r/.test(row.message);

export function problemText(row: Pick<ProblemDto, "message">): string {
  if (isMarkProblem(row)) return "Der Text vor dem Kampf sollte auf ⏎ enden, sonst schließt sein Kasten sofort";
  return row.message;
}

// ---------------------------------------------------------------------------------------------------------------------------- list

export type GroupMode = "place" | "class";
export const GROUP_OPTIONS: readonly SegmentedOption<GroupMode>[] = [
  { value: "place", label: "Nach Ort" },
  { value: "class", label: "Nach Klasse" },
];

export interface RowGroup {
  key: string;
  label: string;
  rows: DataRowDto[];
}

const NONE_KEY = "\u0000none";

/**
 * The rows in groups: by the places whose events start the battle (a trainer standing at two places is in both), or by class; groups
 * by name, "Ohne Ort" or "Ohne Klasse" (locked rows, rows without a place) last. Rows keep their order.
 */
export function groupRows(rows: readonly DataRowDto[], mode: GroupMode): RowGroup[] {
  const groups = new Map<string, RowGroup>();
  const add = (key: string, label: string, row: DataRowDto) => {
    const group = groups.get(key) ?? groups.set(key, { key, label, rows: [] }).get(key)!;
    if (group.rows[group.rows.length - 1] !== row) group.rows.push(row);
  };
  for (const row of rows) {
    const extra = row.extra as TrainerRowExtra | null;
    if (mode === "class") {
      if (extra && typeof extra.class_name === "string") add(`c:${extra.class_name}`, extra.class_name, row);
      else add(NONE_KEY, "Ohne Klasse", row);
      continue;
    }
    const places = extra && Array.isArray(extra.places) ? extra.places : [];
    if (places.length === 0) add(NONE_KEY, "Ohne Ort", row);
    for (const place of places) add(`p:${place.name}`, place.name, row);
  }
  const named = [...groups.values()].filter((g) => g.key !== NONE_KEY).sort((a, b) => a.label.localeCompare(b.label, "de"));
  const none = groups.get(NONE_KEY);
  return none ? [...named, none] : named;
}

/** The highest level of a row's team ("Lv. 12"); null without Pokémon or for a locked row. */
export function rowLevel(row: DataRowDto): number | null {
  const extra = row.extra as TrainerRowExtra | null;
  return extra && typeof extra.level_max === "number" && extra.level_max > 0 ? extra.level_max : null;
}

// ---------------------------------------------------------------------------------------------------------------------------- patches

/** `doc` with a trainer patch written, as the backend would hold it, until the document is read again. */
export function applyTrainer(doc: TrainerDoc, patch: DataPatch): TrainerDoc {
  if ("Trainer" in patch) {
    const { field, value } = patch.Trainer;
    if (field === "class" || field === "ai" || field === "battle_type") return { ...doc, [field]: value };
    const n = Number(field.slice(4));
    return { ...doc, items: doc.items.map((item, i) => (i === n ? value : item)) as TrainerDoc["items"] };
  }
  if ("TrainerName" in patch) return { ...doc, name: patch.TrainerName.name };
  if ("TrainerParty" in patch) return { ...doc, party: patch.TrainerParty.party };
  if ("TrainerFormat" in patch) {
    const { moves, items } = patch.TrainerFormat;
    const format = { moves: moves ?? doc.format.moves, items: items ?? doc.format.items };
    // Moves switched on show as empty until the read brings the ones the backend filled in.
    const party = doc.party.map((m) => ({ ...m, moves: format.moves ? (m.moves ?? [0, 0, 0, 0]) : null, item: format.items ? (m.item ?? 0) : null }));
    return { ...doc, format, party };
  }
  if ("TrainerTexts" in patch) {
    let texts = doc.texts;
    for (const t of patch.TrainerTexts.texts) {
      texts = texts.some((x) => x.kind === t.kind)
        ? texts.map((x) => (x.kind === t.kind ? { ...x, text: t.text } : x))
        : [...texts, { kind: t.kind, archive: doc.text_archive ?? 0, message: 0, text: t.text }];
    }
    return { ...doc, texts };
  }
  return doc;
}
