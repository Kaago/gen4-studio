// Shared by the encounter tests: Route 201's file (Platinum), a HeartGold file, the file list, names and the Pokémon area mounted with
// the Begegnungen shown. Each test file mocks `../../api` itself (`vi.mock` is hoisted per file) and wires the mocks with `wireApi`.
import { act } from "react";
import { vi } from "vitest";
import * as api from "../../api";
import type { DataDocDto, DataRowDto, HeaderSummary, NameTablesDto, ProjectInfo, RawDto, SlotDto } from "../../api";
import { readShellState, writeShellState } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { PT_ARCHIVES, dataHarness, flush, species } from "../dataTesting";
import { PokemonArea, PokemonHand, PokemonList, PokemonView } from "../PokemonArea";
import type { EncounterDoc } from "./encounterModel";

const GRASS = [20, 20, 10, 10, 10, 10, 5, 5, 4, 4, 1, 1];

/** A slot as the kernel gives it: `level` is min and max (grass), `share` the grass share of its place. */
export function slot(index: number, species: number, level: number, over: Partial<SlotDto> = {}): SlotDto {
  return { index, species, min: level, max: level, species_morning: null, species_night: null, share: GRASS[index] ?? 0, replaced_by: [], replaces: [], ...over };
}

const water = (shares: number[]): SlotDto[] => shares.map((share, i) => ({ ...slot(i, 0, 0), share }));

/** Route 201 (file 140, header 342): Staralili and Bidiza in the grass at rate 30, Enton on the water; Tag/Nacht replace slots 3 and 4. */
export const ROUTE_201: EncounterDoc = {
  kind: "encounter",
  methods: [
    {
      method: "grass",
      rate: 30,
      slots: [
        slot(0, 396, 2, { replaced_by: [{ list: "swarm", entry: 0, when: "during a swarm", in_file: true }] }),
        slot(1, 399, 2),
        slot(2, 396, 3, {
          replaced_by: [
            { list: "day", entry: 0, when: "from 10 to 19 o'clock", in_file: true },
            { list: "night", entry: 0, when: "from 20 to 3 o'clock", in_file: true },
          ],
        }),
        slot(3, 399, 3),
        ...Array.from({ length: 7 }, (_, i) => slot(4 + i, 396, 4)),
        slot(11, 0, 0),
      ],
    },
    { method: "surf", rate: 10, slots: [{ ...slot(0, 54, 20), max: 30, share: 60 }, ...water([30, 5, 4, 1]).slice(0).map((s, i) => ({ ...s, index: i + 1 }))] },
    { method: "old_rod", rate: 0, slots: water([60, 30, 5, 4, 1]) },
    { method: "good_rod", rate: 0, slots: water([40, 40, 15, 4, 1]) },
    { method: "super_rod", rate: 0, slots: water([40, 40, 15, 4, 1]) },
  ],
  replacements: [
    { list: "swarm", label: "Schwarm", when: "during a swarm", slots: [slot(0, 403, 2, { share: 20, replaces: [{ method: "grass", slot: 0 }] }), slot(1, 403, 2, { share: 20, replaces: [{ method: "grass", slot: 1 }] })] },
    { list: "day", label: "Tag", when: "from 10 to 19 o'clock", slots: [slot(0, 396, 3, { share: 10, replaces: [{ method: "grass", slot: 2 }] }), slot(1, 399, 3, { share: 10, replaces: [{ method: "grass", slot: 3 }] })] },
    { list: "night", label: "Nacht", when: "from 20 to 3 o'clock", slots: [slot(0, 0, 3, { share: 10, replaces: [{ method: "grass", slot: 2 }] }), slot(1, 399, 3, { share: 10, replaces: [{ method: "grass", slot: 3 }] })] },
  ],
  extras: { form_rates: [0, 0, 0, 0, 0], unown_table: 0 },
  warnings: [{ method: "grass", slot: 2, text: "night entry 0 is empty: from 20 to 3 o'clock, grass slot 2 gives a glitch Pokémon" }],
  problems: [
    {
      severity: "warning",
      file: "encounters/0140",
      line: 3,
      col: 1,
      message: "night entry 0 is empty: from 20 to 3 o'clock, grass slot 2 gives a glitch Pokémon",
      source: "encounter",
    },
  ],
  users: [{ header: 342, name: "Route 201" }],
  effective: [
    { when: "morning", species: [{ species: 396, share: 75 }, { species: 399, share: 24 }] },
    { when: "day", species: [{ species: 396, share: 75 }, { species: 399, share: 24 }] },
    { when: "night", species: [{ species: 396, share: 65 }, { species: 399, share: 24 }] },
  ],
  locked: null,
  raw_locked: null,
};

export function encounterDoc(over: Partial<EncounterDoc> = {}): EncounterDoc {
  return { ...ROUTE_201, ...over };
}

/** File 141, used by Route 202 (header 400) and Route 203 (header 401). */
export const SHARED_FILE: EncounterDoc = encounterDoc({
  warnings: [],
  problems: [],
  users: [
    { header: 400, name: "Route 202" },
    { header: 401, name: "Route 203" },
  ],
});

const hgGrass = (i: number, morning: number, day: number, night: number, level: number): SlotDto => ({ ...slot(i, day, level), species_morning: morning, species_night: night });

/** A HeartGold file: Taubsi in the morning, Rattfratz by day, Hoothoot at night; Rock Smash after Surfen. */
export const HG_FILE: EncounterDoc = {
  ...ROUTE_201,
  methods: [
    { method: "grass", rate: 25, slots: Array.from({ length: 12 }, (_, i) => hgGrass(i, 16, 19, 163, 2 + (i % 3))) },
    { method: "surf", rate: 0, slots: water([60, 30, 5, 4, 1]) },
    { method: "rock_smash", rate: 0, slots: water([80, 20]) },
    { method: "old_rod", rate: 0, slots: water([40, 30, 15, 10, 5]) },
    { method: "good_rod", rate: 0, slots: water([40, 30, 15, 10, 5]) },
    { method: "super_rod", rate: 0, slots: water([40, 30, 15, 10, 5]) },
  ],
  replacements: [
    { list: "radio_hoenn", label: "Radio Hoenn", when: "with the Hoenn sound on the radio", slots: [slot(0, 0, 2, { share: 20 }), slot(1, 0, 2, { share: 20 })] },
    { list: "land_swarm", label: "Schwarm im Gras", when: "during a swarm on one of the 20 swarm headers", slots: [slot(0, 0, 2, { share: 40 })] },
  ],
  extras: { form_rates: null, unown_table: null },
  warnings: [],
  problems: [],
  users: [{ header: 30, name: "Route 29" }],
  effective: [
    { when: "morning", species: [{ species: 16, share: 100 }] },
    { when: "day", species: [{ species: 19, share: 100 }] },
    { when: "night", species: [{ species: 163, share: 100 }] },
  ],
};

/** A row of `data_list encounter`: the headers that use the file. */
export const fileRow = (index: number, users: number[]): DataRowDto => ({ index, name: "", tech: String(index).padStart(4, "0"), locked: null, extra: { users } });

/** File 0 unused, 140 on Route 201, 141 on Routes 202 and 203, 182 unused. */
export const FILE_ROWS: DataRowDto[] = [fileRow(0, []), fileRow(140, [342]), fileRow(141, [400, 401]), fileRow(182, [])];

const header = (id: number, location_name: string): HeaderSummary => ({ id, internal_name: `R${id}`, location_index: 0, kanto: false, location_name, matrix_id: 0, area_data_id: 0, event_file_id: id, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });
export const HEADERS: HeaderSummary[] = [header(342, "Route 201"), header(400, "Route 202"), header(401, "Route 203"), header(30, "Route 29")];

const named = (length: number, at: Record<number, string>, word: string) => Array.from({ length }, (_, i) => at[i] ?? `${word} ${i}`);

export const ENCOUNTER_NAMES: NameTablesDto = {
  species: named(500, { 0: "----------", 16: "Taubsi", 19: "Rattfratz", 54: "Enton", 163: "Hoothoot", 396: "Staralili", 399: "Bidiza", 403: "Sheinux" }, "Art"),
  moves: [],
  items: [],
  abilities: [],
  types: [],
  classes: [],
  forms: [],
  archives: PT_ARCHIVES,
};

export const ENCOUNTER_SPECIES: DataRowDto[] = [
  species(0, "----------"),
  species(16, "Taubsi", [0, 2]),
  species(19, "Rattfratz", [0, 0]),
  species(54, "Enton", [11, 11]),
  species(163, "Hoothoot", [0, 2]),
  species(396, "Staralili", [0, 2]),
  species(399, "Bidiza", [0, 0]),
  species(403, "Sheinux", [12, 12]),
];

export const RAW_FILE: RawDto = { bytes: [30, 0, 0, 0, 2, 0, 0, 0], fields: [{ name: "grass_rate", offset: 0, len: 4, kind: "u32" }], path: "files/fielddata/encountdata/pl_enc_data.narc", member: 140 };

export const projectHg: ProjectInfo = { root: "/hg", game_code: "IPKD", family: "Hgss", language: "de", header_count: 3 };

/** The API mocks of a test file with the answers of a Platinum project around Route 201. */
export function wireApi(doc: (index: number) => DataDocDto = () => encounterDoc()) {
  vi.mocked(api.dataList).mockReset().mockImplementation(async (kind) => (kind === "encounter" ? FILE_ROWS : kind === "species" ? ENCOUNTER_SPECIES : []));
  vi.mocked(api.nameTables).mockReset().mockResolvedValue(ENCOUNTER_NAMES);
  vi.mocked(api.itemTable).mockReset().mockResolvedValue(null);
  vi.mocked(api.getMonIcon).mockReset().mockResolvedValue(null);
  vi.mocked(api.dataGet).mockReset().mockImplementation(async (_kind, index) => doc(index));
  vi.mocked(api.dataEdit).mockReset().mockResolvedValue([{ kind: "Encounter", index: 140 }]);
  vi.mocked(api.dataRaw).mockReset().mockResolvedValue(RAW_FILE);
  vi.mocked(api.refs).mockReset().mockResolvedValue([]);
  vi.mocked(api.createEncounterFile).mockReset().mockResolvedValue(183);
  vi.mocked(api.splitEncounterFile).mockReset().mockResolvedValue({ index: 183, changed: [{ kind: "HeaderTable", index: 0 }] });
  URL.createObjectURL = vi.fn(() => "blob:icon");
  URL.revokeObjectURL = vi.fn();
}

/** The Pokémon area with the Begegnungen shown, its list, view and hand; `selected` is chosen there before it shows, at `depth`. */
export function encounterHarness() {
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
  const open = async (selected: number | null = 140, depth: Depth = "bauen", over: Parameters<typeof h.base>[0] = {}) => {
    const state = readShellState();
    writeShellState({ ...state, depth, sub: { pokemon: "encounters" }, selected: selected === null ? {} : { encounters: selected } });
    h.show(h.base({ headers: HEADERS, ...over }));
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
  const click = async (el: HTMLElement | null | undefined) => {
    if (!el) throw new Error("nothing to click");
    await act(async () => el.click());
    await flush();
  };
  /** A button anywhere in the document by its accessible name or text. */
  const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>("button")].find((b) => (b.getAttribute("aria-label") ?? b.textContent ?? "").trim() === label);
  /** The options of an open picker (in a dialog outside the area). */
  const pickerOptions = () => [...document.querySelectorAll<HTMLElement>("[role=dialog] [role=option]")];
  const dialog = () => document.querySelector<HTMLElement>("[role=dialog]");
  return { ...h, open, list, view, hand, tabs, showTab, field, click, button, pickerOptions, dialog };
}
