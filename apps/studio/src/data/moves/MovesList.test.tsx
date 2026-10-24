// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataGet } from "../../api";
import { openSelect, typeInto } from "../../ui/primitives/testUtils";
import { flush } from "../dataTesting";
import { resetIcons } from "../icons";
import { movesHarness, wingAttackDoc, wireApi } from "./movesTesting";

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

describe("MovesList", () => {
  let h: ReturnType<typeof movesHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = movesHarness();
  });
  afterEach(() => h.unmount());

  const pane = () => h.q<HTMLElement>('section[aria-label="Pokémon"]')!;
  const shown = () => [...pane().querySelectorAll("[data-move]")].map((r) => Number(r.getAttribute("data-move")));
  const row = (index: number) => pane().querySelector<HTMLElement>(`[data-move="${index}"]`);
  const search = () => pane().querySelector<HTMLInputElement>('input[aria-label="Attacken durchsuchen"]')!;

  it("lists_the_moves_with_their_count_and_names_one_without_a_name_by_its_number", async () => {
    await h.open(null);
    expect(pane().querySelector("header")?.textContent).toContain("7 Attacken");
    expect(shown()).toEqual([16, 17, 33, 45, 297, 469, 470]);
    expect(row(17)?.getAttribute("aria-label")).toBe("Flügelschlag, 0017");
    expect(row(17)?.querySelector("[data-type]")?.textContent).toBe("Flug");
    expect(row(469)?.getAttribute("aria-label")).toBe("Attacke 0469, 0469");
  });

  it("finds_a_move_by_name_or_number", async () => {
    await h.open(null);
    typeInto(search(), "flügel");
    expect(shown()).toEqual([17]);
    typeInto(search(), "297");
    expect(shown()).toEqual([297]);
  });

  it("filters_by_type_with_chips", async () => {
    await h.open(null);
    const chips = pane().querySelector<HTMLElement>('[role=group][aria-label="Typ"]')!;
    const flying = [...chips.querySelectorAll<HTMLButtonElement>("button[aria-pressed]")].find((b) => b.textContent?.startsWith("Flug"))!;
    expect(flying.textContent).toContain("3");
    await act(async () => flying.click());
    await flush();
    expect(shown()).toEqual([16, 17, 297]);
  });

  it("filters_by_category", async () => {
    await h.open(null);
    const category = pane().querySelector<HTMLElement>('[aria-label="Kategorie"]')!;
    expect(category.textContent).toContain("Alle Kategorien");
    openSelect(category);
    await flush();
    expect(h.options().map((o) => o.textContent)).toEqual(["Alle Kategorien", "Physisch", "Speziell", "Status"]);
    await h.click(h.options().find((o) => o.textContent === "Status"));
    expect(shown()).toEqual([45, 297]);
    expect(pane().textContent).toContain("Kategorie Status: 2");
    await h.click(h.button("Filter lösen"));
    expect(shown()).toHaveLength(7);
  });

  it("marks_an_unsaved_move", async () => {
    await h.open(null, "bauen", { dirty: [{ kind: "Move", index: 17 }] });
    expect(row(17)?.querySelector('[aria-label="ungespeichert"]')).not.toBeNull();
    expect(row(16)?.querySelector('[aria-label="ungespeichert"]')).toBeNull();
  });

  it("lists_a_locked_move_and_opens_it_with_its_reason", async () => {
    vi.mocked(dataGet).mockResolvedValue(wingAttackDoc({}, { fields: [], locked: "move 0470: 18 bytes, expected 16", raw_locked: Array.from({ length: 18 }, () => 0) }));
    await h.open(470);
    expect(row(470)).not.toBeNull();
    expect(h.view().textContent).toContain("Attacke 0470: 18 Bytes statt 16");
  });
});
