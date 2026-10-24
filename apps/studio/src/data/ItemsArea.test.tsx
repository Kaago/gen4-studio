// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataList, getItemIcon, itemTable, nameTables } from "../api";
import { readShellState } from "../ui/frame/shellState";
import { ITEMS, ITEM_TABLE, NAMES, dataHarness, flush, item } from "./dataTesting";
import { resetIcons } from "./icons";
import { ItemsArea, ItemsAreaList, ItemsHand, ItemsStatus, ItemsView } from "./ItemsArea";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  getMonIcon: vi.fn(),
  getItemIcon: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ItemsArea", () => {
  let h: ReturnType<typeof dataHarness>;

  beforeEach(() => {
    resetIcons();
    vi.mocked(dataList).mockReset().mockImplementation(async (kind) => (kind === "item" ? ITEMS : []));
    vi.mocked(nameTables).mockReset().mockResolvedValue(NAMES);
    vi.mocked(itemTable).mockReset().mockResolvedValue(ITEM_TABLE);
    vi.mocked(getItemIcon).mockReset().mockResolvedValue(new Uint8Array([137, 80, 78, 71]).buffer);
    URL.createObjectURL = vi.fn(() => "blob:icon");
    URL.revokeObjectURL = vi.fn();
    Element.prototype.scrollIntoView = () => {};
    h = dataHarness((props) => (
      <ItemsArea context={props}>
        <ItemsAreaList open onToggle={() => {}} />
        <section data-view="">
          <ItemsView />
        </section>
        <section data-hand="">
          <ItemsHand />
        </section>
        <p data-status="">
          <ItemsStatus />
        </p>
      </ItemsArea>
    ));
  });
  afterEach(() => h.unmount());

  const rows = () => [...h.container.querySelectorAll("[data-item]")].map((r) => Number(r.getAttribute("data-item")));

  it("lists_the_items_by_id_with_name_number_and_icon_and_has_no_switcher", async () => {
    h.show(h.base());
    await flush();
    expect(dataList).toHaveBeenCalledWith("item");
    expect(rows()).toEqual([1, 4, 17]);
    expect(h.q('[data-item="17"]')?.textContent).toContain("Trank");
    expect(h.q('[data-item="17"]')?.textContent).toContain("0017");
    expect(h.q('[data-item="17"] [data-sprite=picture]')).not.toBeNull();
    expect(h.q('section[aria-label="Gegenstände"] header button[aria-haspopup="menu"]')).toBeNull();
    // Items 1 and 4 share icon and palette: one request for both.
    expect(vi.mocked(getItemIcon).mock.calls.map((c) => c[0]).sort((a, b) => a - b)).toEqual([1, 17]);
  });

  it("names_data_members_without_the_item_table_and_shows_no_icons", async () => {
    vi.mocked(itemTable).mockResolvedValue(null);
    vi.mocked(dataList).mockResolvedValue([item(17, "", 1, false)]);
    h.show(h.base());
    await flush();
    expect(h.q('[data-item="17"]')?.textContent).toContain("Daten 0017");
    expect(h.q('[data-item="17"] [data-sprite=picture]')).toBeNull();
    expect(getItemIcon).not.toHaveBeenCalled();
  });

  it("waits_for_the_item_table_before_it_names_the_rows", async () => {
    let answer: (table: typeof ITEM_TABLE) => void = () => {};
    vi.mocked(itemTable).mockImplementation(() => new Promise((resolve) => (answer = resolve)));
    h.show(h.base());
    await flush();
    // The rows are there, the table is not: no "Daten 0017" before "Trank".
    expect(rows()).toEqual([]);
    expect(h.container.textContent).not.toContain("Daten");
    expect(h.container.textContent).toContain("Lädt");
    await act(async () => answer(ITEM_TABLE));
    await flush();
    expect(h.q('[data-item="17"]')?.textContent).toContain("Trank");
  });

  it("puts_the_pocket_into_the_breadcrumb_and_a_click_on_it_shows_that_pocket_only", async () => {
    h.show(h.base());
    await flush();
    expect(h.trail()).toEqual(["Gegenstände"]);
    await act(async () => h.q<HTMLElement>('[data-item="4"]')!.click());
    expect(h.trail()).toEqual(["Gegenstände", "Tasche Bälle", "Pokéball"]);
    expect(h.q("[data-status]")?.textContent).toBe("Pokéball, 0004");
    expect(readShellState().selected).toEqual({ items: 4 });
    await act(async () => h.lastTrail()[1]!.onClick!());
    expect(rows()).toEqual([1, 4]);
    expect(h.container.textContent).toContain("Tasche Bälle");
    await act(async () => h.button("Filter lösen")!.click());
    expect(rows()).toEqual([1, 4, 17]);
  });

  it("says_what_to_do_when_no_item_is_chosen", async () => {
    h.show(h.base());
    await flush();
    expect(h.q("[data-view]")?.textContent).toContain("Kein Gegenstand gewählt.");
  });

  it("opens_the_item_a_link_asks_for", async () => {
    h.show(h.base());
    await flush();
    h.show(h.base({ target: { sub: "items", index: 17 } }));
    await flush();
    expect(h.q('[data-item="17"]')?.getAttribute("aria-selected")).toBe("true");
    expect(h.trail()).toEqual(["Gegenstände", "Tasche Medizin", "Trank"]);
  });
});
