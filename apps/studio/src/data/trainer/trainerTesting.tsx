// Shared by the trainer tests: Wanderer Jens's document, the trainer list, names and the Pokémon area mounted with the trainers shown.
// Each test file mocks `../../api` itself (`vi.mock` is hoisted per file) and wires the mocks with `wireApi`.
import { act } from "react";
import { vi } from "vitest";
import * as api from "../../api";
import type { DataDocDto, DataRowDto, EventsDto, NameTablesDto, PlaceDto, RawDto } from "../../api";
import { readShellState, writeShellState } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { PT_ARCHIVES, dataHarness, flush, move, species } from "../dataTesting";
import { PokemonArea, PokemonHand, PokemonList, PokemonView } from "../PokemonArea";

export type TrainerDoc = Extract<DataDocDto, { kind: "trainer" }>;

/** A trainer row as `data_list trainer` gives it. */
export function trainerRow(index: number, name: string, cls: number, class_name: string, level_max: number, places: PlaceDto[], party_count = level_max > 0 ? 1 : 0): DataRowDto {
  return { index, name, tech: String(index).padStart(4, "0"), locked: null, extra: { class: cls, class_name, level_max, party_count, places } };
}

export const PLACEHOLDER_ROW: DataRowDto = { index: 929, name: "", tech: "0929", locked: "placeholder for the sight script slot", extra: null };

/** Trainer 0 (empty), Wanderer Jens and Göre Mia on Route 201, Wanderer Kai nowhere, the placeholder of the sight script slot. */
export const TRAINER_ROWS: DataRowDto[] = [
  trainerRow(0, "", 0, "Klasse 0", 0, []),
  trainerRow(12, "Jens", 6, "Wanderer", 12, [{ header: 5, name: "Route 201" }]),
  trainerRow(13, "Mia", 7, "Göre", 9, [{ header: 5, name: "Route 201" }]),
  trainerRow(40, "Kai", 6, "Wanderer", 20, []),
  PLACEHOLDER_ROW,
];

const named = (length: number, at: Record<number, string>, word: string) => Array.from({ length }, (_, i) => at[i] ?? `${word} ${i}`);

export const TRAINER_NAMES: NameTablesDto = {
  species: named(500, { 0: "----------", 396: "Staralili", 397: "Staravia", 201: "Icognito" }, "Art"),
  moves: named(471, { 17: "Flügelschlag", 33: "Tackle", 45: "Heuler", 98: "Ruckzuckhieb" }, "Attacke"),
  items: named(468, { 0: "----------", 17: "Trank", 26: "Supertrank" }, "Item"),
  abilities: [],
  types: [],
  classes: named(105, { 0: "Pkmn-Trainer", 6: "Wanderer", 7: "Göre" }, "Klasse"),
  forms: [],
  archives: PT_ARCHIVES,
  sounds: [],
};

export const TRAINER_MOVES: DataRowDto[] = [move(17, "Flügelschlag", 2), move(33, "Tackle"), move(45, "Heuler"), move(98, "Ruckzuckhieb")];

export const TRAINER_SPECIES: DataRowDto[] =[species(0, "----------"), species(201, "Icognito", [13, 13]), species(396, "Staralili", [0, 2]), species(397, "Staravia", [0, 2])];

/** Wanderer Jens (trainer 12): one Staralili on level 12, no own moves or items, simple AI, a single battle; placed as object 3 of event file 8. */
export const JENS: TrainerDoc = {
  kind: "trainer",
  class: 6,
  name: "Jens",
  format: { moves: false, items: false },
  ai: 1,
  battle_type: 0,
  items: [0, 0, 0, 0],
  party: [{ iv_or_difficulty: 0, byte1: 0, level: 12, species: 396, form: 0, item: null, moves: null, ball_capsule: 0, gender: null, ability: null }],
  texts: [
    { kind: 0, archive: 617, message: 100, text: "Los geht's!\\r" },
    { kind: 1, archive: 617, message: 101, text: "Verloren!" },
  ],
  texts_locked: null,
  events: [{ source: { kind: "Event", index: 8 }, field: "script", slot: 3, label: "Eventdatei 0008, Objekt 3", source_name: "Eventdatei 0008", nav: { kind: "event", eventFile: 8, eventKind: "object", index: 3 } }],
  name_archive: 618,
  text_archive: 617,
  problems: [],
  placeholder: false,
  locked: null,
  raw_locked: null,
};

export function trainerDoc(over: Partial<TrainerDoc> = {}): TrainerDoc {
  return { ...JENS, ...over };
}

/** The placeholder trainer of the sight script slot: no Pokémon, locked. */
export const PLACEHOLDER_DOC: TrainerDoc = trainerDoc({ class: 0, name: "", party: [], texts: [], events: [], placeholder: true, locked: "placeholder for the sight script slot" });

/** Event file 8: object 3 stands at 12, 7. */
export const EVENTS_8: EventsDto = {
  event_file_id: 8,
  locked: null,
  bg: [],
  warps: [],
  coords: [],
  objects: [0, 1, 2, 3].map((index) => ({ index, local_id: index, graphics_id: 5, movement_type: 0, script: index === 3 ? 3011 : 0, dir: 1, trainer_type: index === 3 ? 1 : 0, sight: 3, hidden_flag: 0, x: 12, z: 7, y: 0 })),
};

export const RAW_TRAINER: RawDto = {
  bytes: [20, 0, 0, 0, 0, 6, 0, 1, ...Array.from({ length: 16 }, () => 0), 0, 0, 12, 0, 0x8c, 0x01, 0, 0],
  fields: [{ name: "class", offset: 5, len: 1, kind: "u8" }],
  path: "files/poketool/trainer/trdata.narc",
  member: 12,
};

/** The API mocks of a test file with the answers of a Platinum project around Wanderer Jens. */
export function wireApi(doc: () => DataDocDto = () => trainerDoc()) {
  vi.mocked(api.dataList).mockReset().mockImplementation(async (kind) => (kind === "trainer" ? TRAINER_ROWS : kind === "species" ? TRAINER_SPECIES : kind === "move" ? TRAINER_MOVES : []));
  vi.mocked(api.nameTables).mockReset().mockResolvedValue(TRAINER_NAMES);
  vi.mocked(api.itemTable).mockReset().mockResolvedValue(null);
  vi.mocked(api.getMonIcon).mockReset().mockResolvedValue(null);
  vi.mocked(api.getItemIcon).mockReset().mockResolvedValue(null);
  vi.mocked(api.getTrainerSprite).mockReset().mockResolvedValue(null);
  vi.mocked(api.getBattleSprite).mockReset().mockResolvedValue(null);
  vi.mocked(api.dataGet).mockReset().mockImplementation(async (kind) => (kind === "species" ? speciesDoc() : doc()));
  vi.mocked(api.dataEdit).mockReset().mockResolvedValue([{ kind: "Trainer", index: 12 }]);
  vi.mocked(api.dataRaw).mockReset().mockResolvedValue(RAW_TRAINER);
  vi.mocked(api.refs).mockReset().mockResolvedValue([]);
  vi.mocked(api.getEvents).mockReset().mockResolvedValue(EVENTS_8);
  vi.mocked(api.createTrainer).mockReset().mockResolvedValue({ id: 930, placeholder: 929, script_extended: true });
  URL.createObjectURL = vi.fn(() => "blob:icon");
  URL.revokeObjectURL = vi.fn();
}

/** Staralili's species document: only its learnset matters here (Tackle and Heuler at 1, Ruckzuckhieb at 5). */
export const SPECIES_DOC = speciesDoc();

function speciesDoc(): DataDocDto {
  const zero = Object.fromEntries(["hp", "attack", "defense", "speed", "sp_attack", "sp_defense"].map((k) => [k, 0]));
  const personal = { ...zero, type1: 0, type2: 2, catch_rate: 0, base_exp: 0, ev_yield_hp: 0, ev_yield_attack: 0, ev_yield_defense: 0, ev_yield_speed: 0, ev_yield_sp_attack: 0, ev_yield_sp_defense: 0, item1: 0, item2: 0, gender_ratio: 0, egg_cycles: 0, base_friendship: 0, growth_rate: 0, egg_group1: 0, egg_group2: 0, ability1: 0, ability2: 0, flee_rate: 0, color: 0, flip: false } as Extract<DataDocDto, { kind: "species" }>["personal"];
  return {
    kind: "species",
    personal,
    saved: personal,
    learnset: [
      { move_id: 33, level: 1 },
      { move_id: 45, level: 1 },
      { move_id: 98, level: 5 },
    ],
    evolution: [],
    tm: [],
    locked: null,
    raw_locked: null,
    locked_asset: null,
    sprite_slots: null,
    egg_moves: null,
  };
}

/** The Pokémon area with the trainers shown, its list, view and hand; `selected` is chosen there before it shows, at `depth`. */
export function trainerHarness() {
  const h = dataHarness((props) => (
    <PokemonArea context={props}>
      <PokemonList open onToggle={() => {}} />
      <section data-view="">
        <PokemonView />
      </section>
      <section data-hand="">
        <PokemonHand />
      </section>
    </PokemonArea>
  ));
  const open = async (selected: number | null = 12, depth: Depth = "bauen", over: Parameters<typeof h.base>[0] = {}) => {
    const state = readShellState();
    writeShellState({ ...state, depth, sub: { pokemon: "trainer" }, selected: selected === null ? {} : { trainer: selected } });
    h.show(h.base({ headers: [{ id: 5, internal_name: "R201", location_index: 0, kanto: false, location_name: "Route 201", matrix_id: 0, area_data_id: 0, event_file_id: 8, text_archive_id: 0, script_file_id: 0, level_script_id: 0 }], ...over }));
    await flush();
    await flush();
    await flush();
  };
  const list = () => h.q<HTMLElement>('section[aria-label="Pokémon"]')!;
  const view = () => h.q<HTMLElement>("[data-view]")!;
  const hand = () => h.q<HTMLElement>("[data-hand]")!;
  const tabs = () => [...hand().querySelectorAll<HTMLElement>("[role=tab]")];
  const showTab = async (label: string) => {
    const tab = tabs().find((t) => t.textContent?.startsWith(label));
    if (!tab) throw new Error(`no tab ${label}`);
    await act(async () => tab.click());
    await flush();
  };
  const field = (label: string) => hand().querySelector<HTMLInputElement>(`input[aria-label="${label}"]`);
  /** A switch or checkbox of the hand by its label. */
  const toggle = (label: string) =>
    hand().querySelector<HTMLElement>(`[role=checkbox][aria-label="${label}"]`) ??
    [...hand().querySelectorAll<HTMLElement>("label")].find((l) => l.textContent === label)?.querySelector<HTMLElement>("[role=switch], [role=checkbox]") ??
    null;
  const click = async (el: HTMLElement | null | undefined) => {
    if (!el) throw new Error("nothing to click");
    await act(async () => el.click());
    await flush();
  };
  /** The options of an open picker (in a dialog outside the area). */
  const pickerOptions = () => [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];
  /** The options of an open select (rendered outside the area). */
  const selectOptions = () => [...document.querySelectorAll<HTMLElement>("[role=option]")].filter((o) => !h.container.contains(o) && !o.closest("[role=dialog]"));
  return { ...h, open, list, view, hand, tabs, showTab, field, toggle, click, pickerOptions, selectOptions };
}
