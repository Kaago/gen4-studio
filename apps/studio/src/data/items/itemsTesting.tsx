// Shared by the items tests: the Potion's document, the item list and table, references and the Gegenstände area mounted with its parts.
// Each test file mocks `../../api` itself (`vi.mock` is hoisted per file) and wires the mocks with `wireApi`.
import { act } from "react";
import { vi } from "vitest";
import * as api from "../../api";
import type { DataDocDto, DataRowDto, FieldValueDto, ItemEntryDto, NameTablesDto, RawDto, RefDto } from "../../api";
import { readShellState, writeShellState } from "../../ui/frame/shellState";
import type { Depth } from "../../ui/frame/shellState";
import { PT_ARCHIVES, dataHarness, flush } from "../dataTesting";
import { ItemsArea, ItemsAreaList, ItemsHand, ItemsView } from "../ItemsArea";
import { ITEM_FIELDS } from "./itemModel";
import type { ItemField } from "./itemModel";

/** itemData/0017 (Potion) of Platinum DE, as the codec test reads it: 300, fling 30, field pocket 1, battle pocket 4, heals 20 HP. */
export const POTION: Partial<Record<ItemField, number>> = {
  price: 300,
  hold_param: 20,
  fling_power: 30,
  natural_gift_type: 31,
  field_pocket: 1,
  battle_pocket: 4,
  field_use_func: 1,
  battle_use_func: 2,
  party_use: 1,
  hp_restore: 1,
  hp_amount: 20,
};

const WIDTH: Partial<Record<ItemField, number>> = { price: 16, natural_gift_type: 5, field_pocket: 4, battle_pocket: 5, crit_stages: 2 };
const SIGNED = new Set<string>(["hp_ev", "attack_ev", "defense_ev", "speed_ev", "sp_attack_ev", "sp_defense_ev", "friendship_low", "friendship_mid", "friendship_high"]);

/** The fields of an item as `data_get` lists them: every named field with its width (one-bit flags, four-bit stages). */
export function fieldsOf(values: Partial<Record<ItemField, number>>): FieldValueDto[] {
  return ITEM_FIELDS.map((name) => {
    const bits = WIDTH[name] ?? (name.endsWith("_stages") ? 4 : /_(heal|flag)$|^(prevent_toss|can_register|guard_spec|revive|revive_all|level_up|evolve|pp_up|pp_max|pp_restore|pp_restore_all|hp_restore)$/.test(name) ? 1 : 8);
    return { name, value: values[name] ?? 0, bits, signed: SIGNED.has(name) };
  });
}

export const DESCRIPTION = "Ein Spray-Heilmittel, das die\\nKP eines Pokémon um 20 Punkte\\nauffrischt.";

type ItemDoc = Extract<DataDocDto, { kind: "item" }>;
export function potionDoc(over: Partial<Record<ItemField, number>> = {}, doc: Partial<ItemDoc> = {}): DataDocDto {
  return { kind: "item", fields: fieldsOf({ ...POTION, ...over }), description: { archive: 391, message: 17, text: DESCRIPTION }, name_archive: 392, shared_with: [], locked: null, raw_locked: null, ...doc };
}

/** 21 other item ids for a shared member (Platinum's 113 to 134 share member 0). */
export const SHARED_IDS = Array.from({ length: 21 }, (_, i) => 113 + i);

const row = (index: number, name: string, price: number, pocket: number, data = index, icon = true): DataRowDto => ({
  index,
  name,
  tech: String(index).padStart(4, "0"),
  locked: null,
  extra: { price, field_pocket: pocket, icon, data_index: data },
});

export const ITEM_ROWS: DataRowDto[] = [
  row(1, "Meisterball", 0, 2),
  row(4, "Pokéball", 200, 2),
  row(17, "Trank", 300, 1),
  row(113, "Fundstück", 0, 0, 0),
  row(114, "Andenken", 0, 0, 0),
];

export const ITEM_NAMES: NameTablesDto = { species: [], moves: [], items: [], abilities: [], types: [], classes: [], forms: [], archives: PT_ARCHIVES, sounds: [] };

/** Item ids 0 to 134: 1 and 4 share icon and palette, 17 has its own, 113 and up use data member 0 and one icon. */
export const ITEM_TABLE: ItemEntryDto[] = Array.from({ length: 135 }, (_, id) => ({
  data: id >= 113 ? 0 : id,
  icon: id === 17 ? 30 : id >= 113 ? 40 : 5,
  palette: id === 17 ? 31 : id >= 113 ? 41 : 6,
  gba: 0,
}));

/** The Potion: trainer 5 carries it, trainer 7's team holds it, Pikachu holds it in the wild. */
export const ITEM_REFS: RefDto[] = [
  { source: { kind: "TrainerProperties", index: 5 }, field: "item", slot: 0, label: "Wanderer Kai, Gegenstand Platz 1", source_name: "Wanderer Kai", nav: { kind: "trainer", index: 5 } },
  { source: { kind: "TrainerProperties", index: 5 }, field: "item", slot: 1, label: "Wanderer Kai, Gegenstand Platz 2", source_name: "Wanderer Kai", nav: { kind: "trainer", index: 5 } },
  { source: { kind: "TrainerParty", index: 7 }, field: "item", slot: 1, label: "Barry, Team Platz 2", source_name: "Barry", nav: { kind: "trainer", index: 7 } },
  { source: { kind: "Personal", index: 25 }, field: "item1", slot: null, label: "Pikachu, Getragener Gegenstand 1", source_name: "Pikachu", nav: { kind: "species", index: 25, tab: "abilities" } },
];

export const RAW_ITEM: RawDto = {
  bytes: [0x2c, 0x01, 0x00, 0x14, 0x00, 0x00, 0x1e, 0x00, 0x9f, 0x20, 0x01, 0x02, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
  fields: [
    { name: "price", offset: 0, len: 2, kind: "u16" },
    { name: "field_pocket", offset: 8, len: 2, kind: "bits", mask: 0x0780 },
    { name: "hp_amount", offset: 27, len: 1, kind: "u8" },
  ],
  path: "files/itemtool/itemdata/pl_item_data.narc",
  member: 17,
};

/** The API mocks of a test file with the answers of a Platinum project around the Potion. */
export function wireApi(doc: () => DataDocDto = () => potionDoc()) {
  vi.mocked(api.dataList).mockReset().mockImplementation(async (kind) => (kind === "item" ? ITEM_ROWS : []));
  vi.mocked(api.nameTables).mockReset().mockResolvedValue(ITEM_NAMES);
  vi.mocked(api.itemTable).mockReset().mockResolvedValue(ITEM_TABLE);
  vi.mocked(api.getMonIcon).mockReset().mockResolvedValue(null);
  vi.mocked(api.getItemIcon).mockReset().mockResolvedValue(new Uint8Array([137, 80, 78, 71]).buffer);
  vi.mocked(api.dataGet).mockReset().mockImplementation(async () => doc());
  vi.mocked(api.dataEdit).mockReset().mockResolvedValue([{ kind: "Item", index: 17 }]);
  vi.mocked(api.dataRaw).mockReset().mockResolvedValue(RAW_ITEM);
  vi.mocked(api.refs).mockReset().mockResolvedValue(ITEM_REFS);
  URL.createObjectURL = vi.fn(() => "blob:icon");
  URL.revokeObjectURL = vi.fn();
}

/** The Gegenstände area with its list, view and hand; `selected` is chosen there before it shows, at `depth`. */
export function itemsHarness() {
  const h = dataHarness((props) => (
    <ItemsArea context={props}>
      <ItemsAreaList open onToggle={() => {}} />
      <section data-view="">
        <ItemsView />
      </section>
      <section data-hand="">
        <ItemsHand />
      </section>
    </ItemsArea>
  ));
  const open = async (selected: number | null = 17, depth: Depth = "bauen", over: Parameters<typeof h.base>[0] = {}) => {
    const state = readShellState();
    writeShellState({ ...state, depth, selected: selected === null ? {} : { items: selected } });
    h.show(h.base(over));
    await flush();
    await flush();
  };
  const pane = () => h.q<HTMLElement>('section[aria-label="Gegenstände"]')!;
  const view = () => h.q<HTMLElement>("[data-view]")!;
  const hand = () => h.q<HTMLElement>("[data-hand]")!;
  const field = (label: string) => hand().querySelector<HTMLInputElement>(`input[aria-label="${label}"]`);
  /** A switch or checkbox of the hand by its label. */
  const toggle = (label: string) =>
    hand().querySelector<HTMLElement>(`[role=checkbox][aria-label="${label}"]`) ??
    [...hand().querySelectorAll<HTMLElement>("label")].find((l) => l.textContent === label)?.querySelector<HTMLElement>("[role=switch], [role=checkbox]") ??
    null;
  /** The options of an open select (rendered outside the area). */
  const options = () => [...document.querySelectorAll<HTMLElement>("[role=option]")].filter((o) => !h.container.contains(o));
  const click = async (el: HTMLElement | null | undefined) => {
    if (!el) throw new Error("nothing to click");
    await act(async () => el.click());
    await flush();
  };
  return { ...h, open, pane, view, hand, field, toggle, options, click };
}
