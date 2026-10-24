// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataGet, dataList, getItemIcon, itemTable, refs } from "../../api";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { ITEM_ROWS, SHARED_IDS, itemsHarness, potionDoc, wireApi } from "./itemsTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  getMonIcon: vi.fn(),
  getItemIcon: vi.fn(),
  dataGet: vi.fn(),
  dataEdit: vi.fn(),
  dataRaw: vi.fn(),
  refs: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ItemView", () => {
  let h: ReturnType<typeof itemsHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = itemsHarness();
  });
  afterEach(() => h.unmount());

  const card = () => h.view().querySelector<HTMLElement>('section[aria-label="Karte"]')!;
  const uses = () => h.view().querySelector<HTMLElement>('section[aria-label="Verwendet in"]')!;

  it("shows_the_card_with_icon_pocket_battle_use_and_prices", async () => {
    await h.open();
    expect(h.view().querySelector("section[data-item]")?.getAttribute("data-item")).toBe("17");
    expect(dataGet).toHaveBeenCalledWith("item", 17);
    expect(card().querySelector("h2")?.textContent).toBe("TrankArchiv 0392");
    expect(card().textContent).toContain("Nr. 0017");
    expect(card().textContent).toContain("Tasche Medizin");
    expect(card().textContent).toContain("Im Kampf nutzbar");
    expect(card().textContent).toContain("300 ₽");
    expect(card().textContent).toContain("Verkauf");
    expect(card().textContent).toContain("150 ₽");
    expect(card().textContent).toContain("30 Schleuder-Stärke");
    expect(card().querySelector("img[data-sprite=picture]")?.getAttribute("alt")).toBe("Trank");
    expect(getItemIcon).toHaveBeenCalledWith(17);
  });

  it("shows_the_description_on_the_ds_screen_and_leads_to_its_text", async () => {
    await h.open();
    const description = h.view().querySelector<HTMLElement>('section[aria-label="Beschreibung"]')!;
    expect(description.textContent).toContain("Ein Spray-Heilmittel, das die");
    await act(async () => [...description.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.includes("Beschreibung bearbeiten"))!.click());
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "textArchive", index: 391, message: 17 });
  });

  it("lists_where_the_item_is_used_by_trainers_teams_and_wild_pokemon", async () => {
    await h.open();
    expect(refs).toHaveBeenCalledWith({ kind: "item", id: 17 });
    expect(uses().textContent).toContain("1 Trainer-Item, 1 Team-Item, 1 wild getragen, 0 Entwicklungen");
    const chips = [...uses().querySelectorAll<HTMLButtonElement>("button")];
    const trainer = chips.find((b) => b.textContent?.includes("Wanderer Kai"))!;
    expect(trainer.disabled).toBe(true);
    expect(chips.find((b) => b.textContent?.includes("Barry"))?.disabled).toBe(true);
    await act(async () => chips.find((b) => b.textContent?.includes("Pikachu"))!.click());
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "species", index: 25, tab: "abilities" });
    expect(uses().textContent).toContain("Trainer öffnen kommt mit Phase 3b.3b.");
  });

  it("leads_from_the_name_to_its_text", async () => {
    await h.open();
    await act(async () => card().querySelector<HTMLButtonElement>("h2 button")!.click());
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "textArchive", index: 392, message: 17 });
  });

  it("calls_an_item_usable_in_battle_when_it_has_a_battle_pocket", async () => {
    // X items have battle function 0 and still show in the battle bag.
    vi.mocked(dataGet).mockResolvedValue(potionDoc({ battle_pocket: 2, battle_use_func: 0 }));
    await h.open();
    expect(card().textContent).toContain("Im Kampf nutzbar");
    h.unmount();
    h = itemsHarness();
    vi.mocked(dataGet).mockResolvedValue(potionDoc({ battle_pocket: 0, battle_use_func: 2 }));
    await h.open();
    expect(card().textContent).not.toContain("Im Kampf nutzbar");
  });

  it("says_that_the_data_is_shared", async () => {
    vi.mocked(dataGet).mockResolvedValue(potionDoc({}, { shared_with: SHARED_IDS }));
    await h.open();
    expect(card().textContent).toContain("Teilt Daten mit 21 Gegenständen");
  });

  it("reads_the_item_again_when_its_data_member_changed", async () => {
    await h.open(113);
    vi.mocked(dataGet).mockClear();
    vi.mocked(dataGet).mockResolvedValue(potionDoc({ price: 999 }));
    // Item 113 uses data member 0: an edit of member 0 (through any item id) concerns it, one of member 17 does not.
    await act(async () => h.bus.emit({ topic: "dataChanged", kind: "item", index: 17 }));
    await flush();
    expect(dataGet).not.toHaveBeenCalledWith("item", 113);
    await act(async () => h.bus.emit({ topic: "dataChanged", kind: "item", index: 0 }));
    await flush();
    expect(dataGet).toHaveBeenCalledWith("item", 113);
    expect(card().textContent).toContain("999 ₽");
  });

  it("does_not_look_for_uses_without_the_item_table", async () => {
    vi.mocked(itemTable).mockResolvedValue(null);
    vi.mocked(dataGet).mockResolvedValue(potionDoc({}, { name_archive: null }));
    vi.mocked(dataList).mockResolvedValue([{ ...ITEM_ROWS[2]!, name: "", extra: { price: 300, field_pocket: 1, icon: false, data_index: 17 } }]);
    await h.open();
    expect(card().querySelector("h2")?.textContent).toBe("Daten 0017");
    expect(refs).not.toHaveBeenCalled();
    expect(uses().textContent).toContain("Ohne Item-Tabelle lassen sich die Verwendungen nicht suchen.");
  });

  it("shows_why_a_locked_item_has_no_values", async () => {
    vi.mocked(dataGet).mockResolvedValue(potionDoc({}, { fields: [], locked: "item 0017: 36 bytes, expected 34", raw_locked: Array.from({ length: 36 }, () => 0) }));
    await h.open();
    expect(card().textContent).toContain("Gegenstand 0017: 36 Bytes statt 34. Seine Werte sind nicht lesbar.");
    expect(card().textContent).not.toContain("₽");
  });
});
