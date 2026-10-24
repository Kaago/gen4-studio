// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataEdit, dataGet, dataRaw } from "../../api";
import type { AssetId } from "../../api";
import type { EditEvent } from "../../bus/editBus";
import { openSelect, pressKey, typeInto } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { SHARED_IDS, itemsHarness, potionDoc, wireApi } from "./itemsTesting";

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

describe("ItemHand", () => {
  let h: ReturnType<typeof itemsHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = itemsHarness();
  });
  afterEach(() => h.unmount());

  const columns = () => [...h.hand().querySelectorAll<HTMLElement>("section[aria-label]")].map((s) => s.getAttribute("aria-label"));
  const commit = async (label: string, text: string) => {
    const input = h.field(label)!;
    typeInto(input, text);
    pressKey(input, "Enter");
    await flush();
  };
  const patches = () => vi.mocked(dataEdit).mock.calls.map((c) => [c[2], c[3]]);
  /**
   * A backend whose writes land only when `release` is called, one at a time: edits queue up behind the first, and every read in between
   * answers the stored state, without the write still on its way (a stale read for the editor's newer values).
   */
  const holdEdits = () => {
    const stored: Record<string, number> = {};
    const waiting: (() => void)[] = [];
    vi.mocked(dataGet).mockImplementation(async () => potionDoc(stored));
    vi.mocked(dataEdit).mockImplementation(
      (_kind, _index, patch) =>
        new Promise<AssetId[]>((resolve) =>
          waiting.push(() => {
            if ("Item" in patch) stored[patch.Item.field] = patch.Item.value;
            resolve([{ kind: "Item", index: 17 }]);
          }),
        ),
    );
    return async () => {
      while (waiting.length > 0) {
        await act(async () => waiting.shift()!());
        await flush();
      }
    };
  };

  it("has_the_columns_werte_effekte_funktionen_and_the_raw_data_only_while_tinkering", async () => {
    await h.open(17, "bauen");
    expect(h.hand().querySelector('[aria-label="Trank bearbeiten"]')?.getAttribute("data-item")).toBe("17");
    expect(columns()).toEqual(["Werte", "Effekte", "Funktionen"]);
    expect(h.field("Preis")?.value).toBe("300");
    expect(h.field("KP-Menge")).toBeNull();
    expect(dataRaw).not.toHaveBeenCalled();
    h.unmount();
    h = itemsHarness();
    await h.open(17, "tuefteln");
    expect(columns()).toEqual(["Werte", "Effekte", "Funktionen", "Rohdaten"]);
    expect(h.field("KP-Menge")?.value).toBe("20");
    expect(h.toggle("Füllt KP auf")?.getAttribute("aria-checked")).toBe("true");
    expect(dataRaw).toHaveBeenCalledWith("item", 17);
    expect(h.hand().textContent).toContain("files/itemtool/itemdata/pl_item_data.narc");
  });

  it("commits_a_changed_price_as_one_labelled_edit_and_tells_the_others", async () => {
    await h.open();
    const heard: EditEvent[] = [];
    h.bus.subscribe("dataChanged", (e) => heard.push(e));
    vi.mocked(dataGet).mockClear();
    await commit("Preis", "250");
    expect(dataEdit).toHaveBeenCalledWith("item", 17, { Item: { field: "price", value: 250 } }, "Trank: Preis 300 → 250");
    expect(heard).toEqual([{ topic: "dataChanged", kind: "item", index: 17 }]);
    expect(h.calls.onEdited).toHaveBeenCalled();
    expect(dataGet).toHaveBeenCalledWith("item", 17);
  });

  it("refuses_a_price_beyond_16_bits", async () => {
    await h.open();
    await commit("Preis", "65536");
    expect(dataEdit).not.toHaveBeenCalled();
  });

  it("warns_that_shared_data_counts_for_the_other_items_and_names_them_in_the_label", async () => {
    vi.mocked(dataGet).mockResolvedValue(potionDoc({}, { shared_with: SHARED_IDS }));
    await h.open();
    const warning = [...h.hand().querySelectorAll<HTMLElement>("[role=status]")].find((d) => d.textContent?.includes("Preis und Werte"));
    expect(warning?.textContent).toContain("Preis und Werte gelten auch für 21 andere Gegenstände");
    // The list: the other items, by name where the list has one.
    expect(warning?.textContent).toContain("Fundstück");
    expect(warning?.textContent).toContain("Gegenstand 0133");
    await commit("Preis", "250");
    expect(dataEdit).toHaveBeenCalledWith("item", 17, { Item: { field: "price", value: 250 } }, "Trank: Preis 300 → 250, gilt für 22 Gegenstände");
    await h.click([...warning!.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.includes("Fundstück")));
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "item", id: 113 });
  });

  it("builds_two_quick_battle_pocket_edits_on_each_other_while_reads_miss_the_write_on_its_way", async () => {
    // Battle pocket 4. The first edit's re-read answers before the second write landed: the second edit must still keep the first bit.
    const release = holdEdits();
    await h.open();
    await h.click(h.toggle("Pokébälle"));
    await h.click(h.toggle("Kampf-Items"));
    // Both switches show on before the backend answered.
    expect(h.toggle("Pokébälle")?.getAttribute("aria-checked")).toBe("true");
    expect(h.toggle("Kampf-Items")?.getAttribute("aria-checked")).toBe("true");
    await release();
    expect(patches()).toEqual([
      [{ Item: { field: "battle_pocket", value: 0x05 } }, "Trank: Kampftasche Pokébälle aus → an"],
      [{ Item: { field: "battle_pocket", value: 0x07 } }, "Trank: Kampftasche Kampf-Items aus → an"],
    ]);
    expect(h.toggle("Pokébälle")?.getAttribute("aria-checked")).toBe("true");
    expect(h.toggle("Kampf-Items")?.getAttribute("aria-checked")).toBe("true");
  });

  it("names_the_old_value_of_a_second_quick_price_edit_after_the_first", async () => {
    const release = holdEdits();
    await h.open();
    await commit("Preis", "250");
    await commit("Preis", "240");
    await release();
    expect(patches()).toEqual([
      [{ Item: { field: "price", value: 250 } }, "Trank: Preis 300 → 250"],
      [{ Item: { field: "price", value: 240 } }, "Trank: Preis 250 → 240"],
    ]);
  });

  it("chooses_the_pocket_and_the_functions_and_switches_the_bag_flags", async () => {
    await h.open();
    openSelect(h.hand().querySelector<HTMLElement>('[aria-label="Tasche"]')!);
    await flush();
    expect(h.options().map((o) => o.textContent)).toEqual(["Items", "Medizin", "Bälle", "TM/VM", "Beeren", "Briefe", "Kampf-Items", "Basis-Items"]);
    await h.click(h.options().find((o) => o.textContent === "Bälle"));
    expect(dataEdit).toHaveBeenLastCalledWith("item", 17, { Item: { field: "field_pocket", value: 2 } }, "Trank: Tasche Medizin → Bälle");
    openSelect(h.hand().querySelector<HTMLElement>('[aria-label="Kampffunktion"]')!);
    await flush();
    await h.click(h.options().find((o) => o.textContent === "Flucht"));
    expect(dataEdit).toHaveBeenLastCalledWith("item", 17, { Item: { field: "battle_use_func", value: 3 } }, "Trank: Kampffunktion Heilmittel → Flucht");
    await h.click(h.toggle("Nicht wegwerfbar"));
    expect(dataEdit).toHaveBeenLastCalledWith("item", 17, { Item: { field: "prevent_toss", value: 1 } }, "Trank: Nicht wegwerfbar aus → an");
    await h.click(h.toggle("Wirkung im Team"));
    expect(dataEdit).toHaveBeenLastCalledWith("item", 17, { Item: { field: "party_use", value: 0 } }, "Trank: Wirkung im Team an → aus");
  });

  it("edits_a_party_parameter_while_tinkering", async () => {
    await h.open(17, "tuefteln");
    await h.click(h.toggle("Füllt KP auf"));
    expect(dataEdit).toHaveBeenLastCalledWith("item", 17, { Item: { field: "hp_restore", value: 0 } }, "Trank: Füllt KP auf an → aus");
    await commit("KP-Menge", "50");
    expect(dataEdit).toHaveBeenLastCalledWith("item", 17, { Item: { field: "hp_amount", value: 50 } }, "Trank: KP-Menge 20 → 50");
    await commit("EV-Änderung KP", "-10");
    expect(dataEdit).toHaveBeenLastCalledWith("item", 17, { Item: { field: "hp_ev", value: -10 } }, "Trank: EV-Änderung KP 0 → -10");
  });

  it("shows_a_refusal_of_the_backend", async () => {
    vi.mocked(dataEdit).mockRejectedValue({ code: "error", message: "Gegenstand nicht schreibbar", asset: null });
    await h.open();
    await commit("Preis", "250");
    expect(h.hand().textContent).toContain("Gegenstand nicht schreibbar");
  });

  it("shows_why_a_locked_item_has_no_fields", async () => {
    vi.mocked(dataGet).mockResolvedValue(potionDoc({}, { fields: [], locked: "item 0017: 36 bytes, expected 34", raw_locked: Array.from({ length: 36 }, () => 0) }));
    await h.open();
    expect(h.hand().textContent).toContain("Gegenstand 0017: 36 Bytes statt 34");
    expect(h.field("Preis")).toBeNull();
  });
});
