// Shared by the moves tests: Wing Attack's document, the move list, references and the Pokémon area mounted with the moves showing.
// Each test file mocks `../../api` itself (`vi.mock` is hoisted per file) and wires the mocks with `wireApi`.
import { act } from "react";
import { vi } from "vitest";
import * as api from "../../api";
import type { DataDocDto, DataRowDto, FieldValueDto, MoveRowExtra, NameTablesDto, RawDto, RefDto } from "../../api";
import { readShellState, writeShellState } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { dataHarness, flush, move } from "../dataTesting";
import { PokemonArea, PokemonHand, PokemonList, PokemonView } from "../PokemonArea";

/** moveData/0017 (Wing Attack) of Platinum: flags 0x73 (contact, protect, mirror move, king's rock and bit 6). */
export const WING_ATTACK: Record<string, number> = {
  effect: 0,
  category: 0,
  power: 60,
  move_type: 2,
  accuracy: 100,
  pp: 35,
  effect_chance: 0,
  target: 0,
  priority: 0,
  flags: 0x73,
  contest: 5,
  contest_type: 0,
};

const WIDTH: Record<string, [number, boolean]> = { effect: [16, false], target: [16, false], priority: [8, true] };

export const fieldsOf = (values: Record<string, number>): FieldValueDto[] =>
  Object.entries(values).map(([name, value]) => ({ name, value, bits: WIDTH[name]?.[0] ?? 8, signed: WIDTH[name]?.[1] ?? false }));

export const DESCRIPTION = "Trifft den Gegner mit weit\\nausgebreiteten Flügeln.";

export function wingAttackDoc(over: Record<string, number> = {}, doc: Partial<Extract<DataDocDto, { kind: "move" }>> = {}): DataDocDto {
  return { kind: "move", fields: fieldsOf({ ...WING_ATTACK, ...over }), description: { archive: 646, message: 17, text: DESCRIPTION }, locked: null, raw_locked: null, ...doc };
}

const row = (index: number, name: string, extra: MoveRowExtra): DataRowDto => ({ ...move(index, name), extra });

export const MOVE_ROWS: DataRowDto[] = [
  row(16, "Windstoß", { type: 2, category: 1, power: 40, accuracy: 100, pp: 35 }),
  row(17, "Flügelschlag", { type: 2, category: 0, power: 60, accuracy: 100, pp: 35 }),
  row(33, "Tackle", { type: 0, category: 0, power: 35, accuracy: 95, pp: 35 }),
  row(45, "Heuler", { type: 0, category: 2, power: 0, accuracy: 100, pp: 40 }),
  row(297, "Federtanz", { type: 2, category: 2, power: 0, accuracy: 100, pp: 15 }),
  { ...move(469, "move 0469"), extra: { type: 0, category: 0, power: 0, accuracy: 0, pp: 0 } },
  { index: 470, name: "move 0470", tech: "0470", locked: "move 0470: 18 bytes, expected 16", extra: null },
];

export const MOVE_NAMES: NameTablesDto = { species: [], moves: MOVE_ROWS.map((r) => r.name), items: [], abilities: [], types: [], classes: [], forms: [] };

/** Wing Attack: Starly and Staravia learn it, trainer 44 uses it twice, nobody evolves by it. */
export const MOVE_REFS: RefDto[] = [
  { source: { kind: "Learnset", index: 396 }, field: "move", slot: 3, label: "Staralili, Lernset Platz 4", nav: { kind: "species", index: 396, tab: "learnset" } },
  { source: { kind: "Learnset", index: 397 }, field: "move", slot: 3, label: "Staravia, Lernset Platz 4", nav: { kind: "species", index: 397, tab: "learnset" } },
  { source: { kind: "TrainerParty", index: 44 }, field: "move", slot: 0, label: "Wanderer Kai, Team Platz 1", nav: { kind: "trainer", index: 44 } },
  { source: { kind: "TrainerParty", index: 44 }, field: "move", slot: 1, label: "Wanderer Kai, Team Platz 2", nav: { kind: "trainer", index: 44 } },
];

export const RAW_MOVE: RawDto = {
  bytes: [0x00, 0x00, 0x00, 0x3c, 0x02, 0x64, 0x23, 0x00, 0x00, 0x00, 0x00, 0x73, 0x05, 0x00, 0x00, 0x00],
  fields: [
    { name: "effect", offset: 0, len: 2, kind: "u16" },
    { name: "power", offset: 3, len: 1, kind: "u8" },
    { name: "flags", offset: 11, len: 1, kind: "u8" },
  ],
  path: "files/poketool/waza/pl_waza_tbl.narc",
  member: 17,
};

/** The API mocks of a test file with the answers of a Platinum project around Wing Attack. */
export function wireApi(doc: () => DataDocDto = () => wingAttackDoc()) {
  vi.mocked(api.dataList).mockReset().mockImplementation(async (kind) => (kind === "move" ? MOVE_ROWS : []));
  vi.mocked(api.nameTables).mockReset().mockResolvedValue(MOVE_NAMES);
  vi.mocked(api.itemTable).mockReset().mockResolvedValue(null);
  vi.mocked(api.getMonIcon).mockReset().mockResolvedValue(null);
  vi.mocked(api.dataGet).mockReset().mockImplementation(async () => doc());
  vi.mocked(api.dataEdit).mockReset().mockResolvedValue([{ kind: "Move", index: 17 }]);
  vi.mocked(api.dataRaw).mockReset().mockResolvedValue(RAW_MOVE);
  vi.mocked(api.refs).mockReset().mockResolvedValue(MOVE_REFS);
}

/** The Pokémon area with its list, view and hand, showing the moves; `selected` is chosen there before it shows, at `depth`. */
export function movesHarness() {
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
  const open = async (selected: number | null = 17, depth: Depth = "bauen", over: Parameters<typeof h.base>[0] = {}) => {
    const state = readShellState();
    writeShellState({ ...state, depth, sub: { pokemon: "moves" }, selected: selected === null ? {} : { moves: selected } });
    h.show(h.base(over));
    await flush();
    await flush();
  };
  const view = () => h.q<HTMLElement>("[data-view]")!;
  const hand = () => h.q<HTMLElement>("[data-hand]")!;
  const field = (label: string) => hand().querySelector<HTMLInputElement>(`input[aria-label="${label}"]`);
  const checkbox = (label: string) => hand().querySelector<HTMLElement>(`[role=checkbox][aria-label="${label}"]`) ?? [...hand().querySelectorAll<HTMLElement>("label")].find((l) => l.textContent === label)?.querySelector<HTMLElement>("[role=checkbox]") ?? null;
  /** The options of an open select (rendered outside the area). */
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")].filter((o) => !h.container.contains(o));
  const click = async (el: HTMLElement | null | undefined) => {
    if (!el) throw new Error("nothing to click");
    await act(async () => el.click());
    await flush();
  };
  return { ...h, open, view, hand, field, checkbox, options, click };
}
