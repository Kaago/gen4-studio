// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataList, machineTable } from "../../api";
import type { DataRowDto } from "../../api";
import { typeInto } from "../../ui/primitives/testUtils";
import { flush, species } from "../dataTesting";
import { resetIcons } from "../icons";
import { pokedexHarness, wireApi } from "./pokedexTesting";
import { chooseContext, readyWith, contextTexts } from "../../original/resetTesting";

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
  machineTable: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("PokedexList", () => {
  let h: ReturnType<typeof pokedexHarness>;

  beforeEach(() => {
    resetIcons();
    wireApi();
    Element.prototype.scrollIntoView = () => {};
    h = pokedexHarness();
  });
  afterEach(() => h.unmount());

  const shown = () => [...h.container.querySelectorAll("[data-species]")].map((r) => Number(r.getAttribute("data-species")));
  const row = (index: number) => h.q<HTMLElement>(`[data-species="${index}"]`);

  it("counts_species_and_forms_in_the_head", async () => {
    vi.mocked(dataList).mockImplementation(async (kind) => (kind === "species" ? Array.from({ length: 508 }, (_, i) => species(i, `Art Nummer ${i}`)) : []));
    await h.open(null);
    expect(h.container.querySelector('section[aria-label="Pokémon"] header')?.textContent).toContain("493 Arten, 12 Formen");
  });

  it("finds_a_species_by_its_number", async () => {
    await h.open(null);
    typeInto(h.q<HTMLInputElement>('input[aria-label="Pokédex durchsuchen"]')!, "397");
    expect(shown()).toEqual([397]);
    expect(h.q<HTMLInputElement>('input[aria-label="Pokédex durchsuchen"]')?.placeholder).toBe("Name oder Nummer");
  });

  it("filters_by_type_with_chips", async () => {
    await h.open(null);
    const chips = h.q<HTMLElement>('[role=group][aria-label="Typ"]')!;
    const flying = [...chips.querySelectorAll<HTMLButtonElement>("button[aria-pressed]")].find((b) => b.textContent?.startsWith("Flug"))!;
    await act(async () => flying.click());
    await flush();
    expect(shown()).toEqual([396, 397, 398]);
    expect(h.container.textContent).toContain("Typ Flug: 3");
  });

  it("names_forms_and_species_without_a_name", async () => {
    await h.open(null);
    expect(row(503)?.textContent).toContain("Hitze-Rotom");
    expect(row(520)?.textContent).toContain("Art 0520");
  });

  it("marks_a_species_with_unsaved_parts", async () => {
    await h.open(null, "bauen", { dirty: [{ kind: "Evolution", index: 397 }, { kind: "Move", index: 396 }] });
    expect(row(397)?.querySelector('[aria-label="ungespeichert"]')).not.toBeNull();
    expect(row(396)?.querySelector('[aria-label="ungespeichert"]')).toBeNull();
  });

  it("lets_the_machine_grid_show_bare_numbers_without_the_machine_table", async () => {
    vi.mocked(machineTable).mockResolvedValue(null);
    await h.open();
    await h.showTab("TM/VM");
    const grid = h.hand().querySelector<HTMLElement>("[role=group][aria-label='TM und VM']")!;
    const tm40 = [...grid.querySelectorAll<HTMLElement>("[role=checkbox]")][39]!;
    expect(tm40.getAttribute("aria-label") ?? tm40.closest("[data-cell]")?.textContent).toBe("TM40");
  });

  it("keeps_rows_whose_member_is_locked", async () => {
    const locked: DataRowDto = { index: 398, name: "Staraptor", tech: "0398", locked: "personal 0398: 36 bytes, expected 44", extra: null };
    vi.mocked(dataList).mockImplementation(async (kind) => (kind === "species" ? [species(1, "Bisasam"), locked] : []));
    await h.open(null);
    expect(row(398)?.textContent).toContain("Staraptor");
  });

  it("rows_offer_reset_and_lock_a_new_species", async () => {
    // The base has 504 species (0 to 503): 520 is new.
    h.reset.status = readyWith({ species: 504 });
    await h.open();
    chooseContext(row(503)!, "Auf Original zurücksetzen…");
    expect(h.reset.open).toHaveBeenCalledWith({ kind: "species", index: 503 });
    expect(contextTexts(row(520)!)).toEqual(["Auf Original zurücksetzen…Kein Original"]);
    expect(row(520)?.getAttribute("aria-selected")).toBe("false");
  });
});
