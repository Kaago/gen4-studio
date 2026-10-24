// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataList, getItemIcon, itemTable } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { ITEM_ROWS, itemsHarness, wireApi } from "./itemsTesting";

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

describe("ItemsList", () => {
  let h: ReturnType<typeof itemsHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = itemsHarness();
  });
  afterEach(() => h.unmount());

  const shown = () => [...h.pane().querySelectorAll("[data-item]")].map((r) => Number(r.getAttribute("data-item")));
  const row = (id: number) => h.pane().querySelector<HTMLElement>(`[data-item="${id}"]`);
  const chip = (label: string) => [...h.pane().querySelectorAll<HTMLButtonElement>('[role=group][aria-label="Taschen"] button[aria-pressed]')].find((b) => b.textContent?.startsWith(label));

  it("lists_the_items_by_id_with_icon_name_and_the_price_on_the_right", async () => {
    await h.open(null);
    expect(h.pane().querySelector("header")?.textContent).toContain("5 Gegenstände");
    expect(shown()).toEqual([1, 4, 17, 113, 114]);
    expect(row(17)?.getAttribute("aria-label")).toBe("Trank, 0017");
    expect(row(17)?.textContent).toContain("300 ₽");
    expect(row(17)?.querySelector("[data-sprite=picture]")).not.toBeNull();
  });

  it("asks_for_each_icon_and_palette_once", async () => {
    await h.open(null);
    // 1 and 4 share icon 5 and palette 6; 113 and 114 share icon 40 and palette 41; 17 has its own.
    expect(vi.mocked(getItemIcon).mock.calls.map((c) => c[0]).sort((a, b) => a - b)).toEqual([1, 17, 113]);
  });

  it("names_data_members_without_the_item_table_and_says_why", async () => {
    vi.mocked(itemTable).mockResolvedValue(null);
    vi.mocked(dataList).mockResolvedValue([{ ...ITEM_ROWS[2]!, name: "", extra: { price: 300, field_pocket: 1, icon: false, data_index: 17 } }]);
    await h.open(null);
    expect(row(17)?.textContent).toContain("Daten 0017");
    expect(row(17)?.querySelector("[data-sprite=picture]")).toBeNull();
    expect(getItemIcon).not.toHaveBeenCalled();
    const notice = [...h.pane().querySelectorAll<HTMLElement>("[role=status]")].find((n) => n.textContent?.includes("Item-Tabelle"));
    expect(notice?.textContent).toBe("Item-Tabelle nicht lesbar: Namen und Symbole fehlen");
  });

  it("shows_no_notice_with_the_item_table", async () => {
    await h.open(null);
    expect(h.pane().textContent).not.toContain("Item-Tabelle nicht lesbar");
  });

  it("filters_by_pocket_with_chips_and_their_counts", async () => {
    await h.open(null);
    expect(chip("Medizin")?.textContent).toBe("Medizin1");
    expect(chip("Bälle")?.textContent).toBe("Bälle2");
    expect(chip("Basis-Items")).toBeDefined();
    await act(async () => chip("Bälle")!.click());
    await flush();
    expect(shown()).toEqual([1, 4]);
    expect(h.pane().textContent).toContain("Tasche Bälle: 2");
    // Another pocket replaces the first: one pocket at a time.
    await act(async () => chip("Medizin")!.click());
    await flush();
    expect(shown()).toEqual([17]);
    await act(async () => chip("Medizin")!.click());
    await flush();
    expect(shown()).toHaveLength(5);
  });

  it("finds_an_item_by_name_or_number", async () => {
    await h.open(null);
    const search = h.pane().querySelector<HTMLInputElement>('input[aria-label="Gegenstände durchsuchen"]')!;
    typeInto(search, "trank");
    expect(shown()).toEqual([17]);
    typeInto(search, "113");
    expect(shown()).toEqual([113]);
  });

  it("marks_a_locked_item_and_names_its_data_member_as_the_reason", async () => {
    // Item 114 uses data member 0, which has the wrong length.
    vi.mocked(dataList).mockResolvedValue(ITEM_ROWS.map((r) => (r.index === 114 ? { ...r, locked: "item 0000: 36 bytes, expected 34", extra: null } : r)));
    await h.open(null);
    expect(row(114)?.textContent).toContain("Gesperrt");
    expect(row(114)?.getAttribute("aria-label")).toBe("Andenken, 0114, gesperrt: Daten 0000: 36 Bytes statt 34");
    expect(row(17)?.textContent).not.toContain("Gesperrt");
    expect(row(17)?.getAttribute("aria-label")).toBe("Trank, 0017");
  });

  it("marks_every_item_of_an_unsaved_data_member", async () => {
    await h.open(null, "bauen", { dirty: [{ kind: "Item", index: 0 }] });
    expect(row(113)?.querySelector('[aria-label="ungespeichert"]')).not.toBeNull();
    expect(row(114)?.querySelector('[aria-label="ungespeichert"]')).not.toBeNull();
    expect(row(17)?.querySelector('[aria-label="ungespeichert"]')).toBeNull();
  });
});
