// Shared by the Pokédex tests: Staravia's document, names, references and the Pokémon area mounted with the Pokédex selected.
// Each test file mocks `../../api` itself (`vi.mock` is hoisted per file) and wires the mocks with `wireApi`.
import { act } from "react";
import { vi } from "vitest";
import * as api from "../../api";
import type { DataDocDto, DataRowDto, EvoEntryDto, LearnEntryDto, MoveRowExtra, NameTablesDto, PersonalDto, RawDto, RefDto } from "../../api";
import { readShellState, writeShellState } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { dataHarness, flush, move, species } from "../dataTesting";
import { PokemonArea, PokemonHand, PokemonList, PokemonView } from "../PokemonArea";

export const STARAVIA: PersonalDto = {
  hp: 55,
  attack: 75,
  defense: 50,
  speed: 80,
  sp_attack: 40,
  sp_defense: 40,
  type1: 0,
  type2: 2,
  catch_rate: 120,
  base_exp: 119,
  ev_yield_hp: 0,
  ev_yield_attack: 2,
  ev_yield_defense: 0,
  ev_yield_speed: 0,
  ev_yield_sp_attack: 0,
  ev_yield_sp_defense: 0,
  item1: 0,
  item2: 0,
  gender_ratio: 127,
  egg_cycles: 15,
  base_friendship: 70,
  growth_rate: 3,
  egg_group1: 4,
  egg_group2: 4,
  ability1: 22,
  ability2: 0,
  flee_rate: 0,
  color: 5,
  flip: false,
};

export const LEARNSET: LearnEntryDto[] = [
  { move_id: 33, level: 1 },
  { move_id: 45, level: 1 },
  { move_id: 98, level: 5 },
  { move_id: 17, level: 9 },
];

const empty: EvoEntryDto = { method: 0, param: 0, target: 0 };
/** Staravia becomes Staraptor at level 34. */
export const EVOLUTION: EvoEntryDto[] = [{ method: 4, param: 34, target: 398 }, empty, empty, empty, empty, empty, empty];

/** TM40 (Aero-Ass) and HM02 (Fliegen) on. */
export const TM = Array.from({ length: 100 }, (_, n) => n === 39 || n === 93);

export function staraviaDoc(over: Partial<Extract<DataDocDto, { kind: "species" }>> = {}): DataDocDto {
  return { kind: "species", personal: STARAVIA, saved: STARAVIA, learnset: LEARNSET, evolution: EVOLUTION, tm: TM, locked: null, raw_locked: null, locked_asset: null, ...over };
}

const named = (length: number, at: Record<number, string>, word: string) => Array.from({ length }, (_, i) => at[i] ?? `${word} ${i}`);

/** 124 abilities (Bedroher is 22), the moves the learnset names, a few items. */
export const POKEDEX_NAMES: NameTablesDto = {
  species: [],
  moves: named(471, { 17: "Flügelschlag", 33: "Tackle", 45: "Heuler", 98: "Ruckzuckhieb", 332: "Aero-Ass", 19: "Fliegen" }, "Attacke"),
  items: named(468, { 0: "----------", 1: "Meisterball", 17: "Trank", 229: "Ewigstein" }, "Item"),
  abilities: named(124, { 0: "-------", 22: "Bedroher", 51: "Adlerauge" }, "Fähigkeit"),
  types: [],
  classes: [],
  forms: [],
};

export const POKEDEX_SPECIES: DataRowDto[] = [
  species(0, "----------"),
  species(1, "Bisasam", [12, 3]),
  { ...species(396, "Staralili", [0, 2]), extra: { types: [0, 2], number: 396, stats: [40, 55, 30, 60, 30, 30] } },
  { ...species(397, "Staravia", [0, 2]), extra: { types: [0, 2], number: 397, stats: [55, 75, 50, 80, 40, 40] } },
  species(398, "Staraptor", [0, 2]),
  species(503, "Hitze-Rotom", [13, 9]),
  species(520, "species 0520"),
];

export const POKEDEX_MOVES: DataRowDto[] = [
  { ...move(17, "Flügelschlag", 2), extra: { type: 2, category: 0, power: 60, accuracy: 100, pp: 35 } satisfies MoveRowExtra },
  move(33, "Tackle", 0),
  { ...move(45, "Heuler", 0), extra: { type: 0, category: 2, power: 0, accuracy: 100, pp: 40 } satisfies MoveRowExtra },
  move(98, "Ruckzuckhieb", 0),
  move(332, "Aero-Ass", 2),
];

/** Staravia waits in the grass of encounter file 12, which header 5 (Route 209) uses, and in the party of trainer 44. */
export const REFS: RefDto[] = [
  { source: { kind: "Encounter", index: 12 }, field: "grass", slot: 2, label: "Begegnungen 0012, Gras Platz 3", nav: { kind: "header", id: 5 } },
  { source: { kind: "Encounter", index: 12 }, field: "grass", slot: 5, label: "Begegnungen 0012, Gras Platz 6", nav: { kind: "header", id: 5 } },
  { source: { kind: "TrainerParty", index: 44 }, field: "species", slot: 0, label: "Wanderer Kai, Team Platz 1", nav: { kind: "trainer", index: 44 } },
  { source: { kind: "Evolution", index: 396 }, field: "target", slot: 0, label: "Staralili, Entwicklung 1 Ziel", nav: { kind: "species", index: 396, tab: "evolution" } },
];

export const RAW_PERSONAL: RawDto = {
  bytes: [55, 75, 50, 80, 40, 40, 0, 2, 120, 119, 8, 0, ...Array.from({ length: 32 }, () => 0)],
  fields: [
    { name: "hp", offset: 0, len: 1, kind: "u8" },
    { name: "attack", offset: 1, len: 1, kind: "u8" },
  ],
  path: "files/poketool/personal/pl_personal.narc",
  member: 397,
};

/** The API mocks of a test file with the answers of a Platinum project around Staravia. */
export function wireApi(doc: () => DataDocDto = () => staraviaDoc()) {
  vi.mocked(api.dataList).mockReset().mockImplementation(async (kind) => (kind === "species" ? POKEDEX_SPECIES : kind === "move" ? POKEDEX_MOVES : []));
  vi.mocked(api.nameTables).mockReset().mockResolvedValue(POKEDEX_NAMES);
  vi.mocked(api.itemTable).mockReset().mockResolvedValue(null);
  vi.mocked(api.getMonIcon).mockReset().mockResolvedValue(null);
  vi.mocked(api.getItemIcon).mockReset().mockResolvedValue(null);
  vi.mocked(api.dataGet).mockReset().mockImplementation(async () => doc());
  vi.mocked(api.dataEdit).mockReset().mockResolvedValue([{ kind: "Personal", index: 397 }]);
  vi.mocked(api.dataRaw).mockReset().mockResolvedValue(RAW_PERSONAL);
  vi.mocked(api.refs).mockReset().mockResolvedValue(REFS);
  vi.mocked(api.machineTable).mockReset().mockResolvedValue(Array.from({ length: 100 }, (_, n) => (n === 39 ? 332 : n === 93 ? 19 : 0)));
  URL.createObjectURL = vi.fn(() => "blob:icon");
  URL.revokeObjectURL = vi.fn();
}

/** The Pokémon area with its list, view and hand; `selected` is chosen in the Pokédex before it shows, at `depth`. */
export function pokedexHarness() {
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
  const open = async (selected: number | null = 397, depth: Depth = "bauen", over: Parameters<typeof h.base>[0] = {}) => {
    writeShellState({ ...readShellState(), depth, selected: selected === null ? {} : { pokedex: selected } });
    h.show(h.base({ headers: [{ id: 5, internal_name: "R209", location_name: "Route 209", matrix_id: 0, area_data_id: 0, event_file_id: 0, text_archive_id: 0, script_file_id: 0, level_script_id: 0 }], ...over }));
    await flush();
    await flush();
  };
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
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];
  const click = async (el: HTMLElement | null | undefined) => {
    if (!el) throw new Error("nothing to click");
    await act(async () => el.click());
    await flush();
  };
  return { ...h, open, view, hand, tabs, showTab, field, options, click };
}
