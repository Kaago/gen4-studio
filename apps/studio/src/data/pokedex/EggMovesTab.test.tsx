// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataEdit, dataGet } from "../../api";
import type { AssetId, EggMovesDto } from "../../api";
import type { EditEvent } from "../../bus/editBus";
import { typeInSearch } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { pokedexHarness, staraviaDoc, wireApi } from "./pokedexTesting";

vi.mock("../../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  getMonIcon: vi.fn(),
  getBattleSprite: vi.fn(),
  getItemIcon: vi.fn(),
  dataGet: vi.fn(),
  dataEdit: vi.fn(),
  dataRaw: vi.fn(),
  refs: vi.fn(),
  machineTable: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** HeartGold: Flügelschlag, Heuler, Ruckzuckhieb from the game's NARC, no budget. */
const HGSS: EggMovesDto = { moves: [17, 45, 98], source: "narc", budget: null, locked: null };
/** Platinum's overlay table, every word in use. */
const FULL: EggMovesDto = { moves: [17, 45, 98], source: "overlay", budget: { used: 1910, total: 1910 }, locked: null };

describe("EggMovesTab", () => {
  let h: ReturnType<typeof pokedexHarness>;

  beforeEach(() => {
    resetIcons();
    Element.prototype.scrollIntoView = () => {};
    h = pokedexHarness();
  });
  afterEach(() => h.unmount());

  const show = async (egg_moves: EggMovesDto | null, dirty: AssetId[] = []) => {
    wireApi(() => staraviaDoc({ egg_moves }));
    vi.mocked(dataEdit).mockResolvedValue([{ kind: "EggMoves", index: 0 }]);
    await h.open(397, "bauen", { dirty });
    await h.showTab("Ei-Attacken");
  };
  const tab = () => h.hand().querySelector<HTMLElement>('[data-tab="eggMoves"]')!;
  const rows = () => [...tab().querySelectorAll<HTMLElement>("[data-egg]")];
  const add = () => tab().querySelector<HTMLButtonElement>("[data-add-egg]");

  it("rows_show_moves_with_names", async () => {
    await show(HGSS);
    expect(rows().map((r) => r.getAttribute("data-move"))).toEqual(["17", "45", "98"]);
    expect(rows().map((r) => r.querySelector("button")?.textContent)).toEqual(["Flügelschlag", "Heuler", "Ruckzuckhieb"]);
    expect(tab().textContent).toContain("3 Ei-Attacken");
    // HeartGold's table has no budget.
    expect(tab().querySelector("[data-egg-budget]")).toBeNull();
    expect(add()?.getAttribute("aria-disabled")).toBeNull();
    // The tab names how many the species has.
    expect(h.tabs().find((t) => t.textContent?.startsWith("Ei-Attacken"))?.textContent).toBe("Ei-Attacken3");
  });

  it("changing_a_move_calls_data_edit_with_the_whole_list", async () => {
    await show(HGSS);
    const heard: EditEvent[] = [];
    h.bus.subscribe("dataChanged", (e) => heard.push(e));
    vi.mocked(dataGet).mockClear();
    await h.click(tab().querySelector<HTMLElement>('button[aria-label="Attacke in Zeile 2 wählen, jetzt Heuler"]'));
    await h.click(h.options().find((o) => o.textContent?.includes("Aero-Ass")));
    expect(dataEdit).toHaveBeenCalledWith("species", 397, { EggMoves: { moves: [17, 332, 98] } }, "Staravia: Ei-Attacken getauscht");
    // The table belongs to every species: the edit is announced for any of them, and the editor reads the species again.
    expect(heard).toContainEqual({ topic: "dataChanged", kind: "species", index: null });
    expect(dataGet).toHaveBeenCalledWith("species", 397);
  });

  it("removes_and_adds_a_move_with_the_whole_list", async () => {
    await show(HGSS);
    await h.click(tab().querySelector<HTMLElement>('button[aria-label="Heuler entfernen"]'));
    expect(dataEdit).toHaveBeenLastCalledWith("species", 397, { EggMoves: { moves: [17, 98] } }, "Staravia: Ei-Attacken 3 → 2");
    await h.click(add());
    typeInSearch(document.querySelector<HTMLInputElement>('input[aria-label="Suchen"]')!, "Aero");
    await h.click(h.options()[0]);
    expect(dataEdit).toHaveBeenLastCalledWith("species", 397, { EggMoves: { moves: [17, 45, 98, 332] } }, "Staravia: Ei-Attacken 3 → 4");
  });

  it("add_is_locked_when_the_platinum_budget_is_full", async () => {
    await show(FULL);
    expect(tab().querySelector("[data-egg-budget]")?.textContent).toBe("3 Ei-Attacken · Tabelle 1 910 von 1 910 Einträgen");
    expect(add()?.getAttribute("aria-disabled")).toBe("true");
    expect(tab().textContent).toContain("Tabelle voll: erst eine Ei-Attacke bei einer anderen Art entfernen");
    await h.click(add());
    expect(h.options()).toHaveLength(0);
    // Swapping and removing stay open.
    expect(tab().querySelector<HTMLButtonElement>('button[aria-label="Heuler entfernen"]')?.disabled).toBe(false);
    expect(tab().querySelector<HTMLButtonElement>('button[aria-label="Attacke in Zeile 2 wählen, jetzt Heuler"]')?.disabled).toBe(false);
  });

  it("add_is_locked_when_a_new_entry_would_need_two_words_and_one_is_left", async () => {
    await show({ moves: [], source: "overlay", budget: { used: 1909, total: 1910 }, locked: null });
    expect(add()?.getAttribute("aria-disabled")).toBe("true");
    expect(tab().textContent).toContain("Tabelle voll");
  });

  it("add_is_open_while_the_budget_has_room", async () => {
    await show({ ...FULL, budget: { used: 1909, total: 1910 } });
    expect(add()?.getAttribute("aria-disabled")).toBeNull();
    expect(tab().querySelector("[data-egg-budget]")?.textContent).toBe("3 Ei-Attacken · Tabelle 1 909 von 1 910 Einträgen");
  });

  it("add_is_locked_at_sixteen_moves", async () => {
    await show({ ...HGSS, moves: Array.from({ length: 16 }, () => 33) });
    expect(add()?.getAttribute("aria-disabled")).toBe("true");
    expect(tab().textContent).toContain("Höchstens 16 Ei-Attacken je Art");
  });

  it("a_refusal_of_the_backend_shows_as_an_error", async () => {
    await show(HGSS);
    vi.mocked(dataEdit).mockRejectedValue({ code: "error", message: "Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen", asset: null });
    await h.click(tab().querySelector<HTMLElement>('button[aria-label="Heuler entfernen"]'));
    await flush();
    expect(h.hand().querySelector("[role=alert]")?.textContent).toContain("Ei-Attacken-Tabelle voll: 1 910 von 1 910 Einträgen");
  });

  it("a_locked_table_shows_the_reason_without_fields", async () => {
    const reason = "Ei-Attacken: Overlay 5 trägt an 0x2923E keine Tabelle (Signatur fehlt)";
    await show({ moves: [], source: "overlay", budget: null, locked: reason });
    const notice = tab().querySelector("[role=status]");
    expect(notice?.textContent).toContain(reason);
    expect(notice?.textContent).not.toContain("read-only");
    expect(rows()).toHaveLength(0);
    expect(add()).toBeNull();
    expect(tab().querySelector("button")).toBeNull();
  });

  it("a_game_without_a_table_says_so", async () => {
    await show(null);
    expect(tab().textContent).toContain("Dieses Spiel hat keine Ei-Attacken-Tabelle, die das Studio kennt.");
    expect(rows()).toHaveLength(0);
    expect(add()).toBeNull();
  });

  it("marks_the_tab_unsaved_from_the_shared_table", async () => {
    // The table is asset 0 for every species: its unsaved changes dot the tab of any species.
    await show(HGSS, [{ kind: "EggMoves", index: 0 }]);
    const dotted = h.tabs().filter((t) => t.parentElement?.querySelector('[aria-label="ungespeichert"]')).map((t) => t.textContent?.replace(/\d+$/, ""));
    expect(dotted).toEqual(["Ei-Attacken"]);
  });
});
