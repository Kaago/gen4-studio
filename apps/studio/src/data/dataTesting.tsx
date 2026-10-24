// Shared by the tests of the data areas: sample rows and names, the areas mounted with all their parts, and the ways a user clicks.
// Each test file mocks `../api` itself (`vi.mock` is hoisted per file).
import { act } from "react";
import type { ReactNode } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { vi } from "vitest";
import type { DataRowDto, ItemEntryDto, NameArchivesDto, NameTablesDto, ProjectInfo } from "../api";
import { createEditBus } from "../bus/editBus";
import { ShellProvider } from "../ui/frame/ShellContext";
import type { Crumb } from "../workbench/breadcrumb";
import { DataStore } from "./dataStore";
import type { DataAreaProps } from "./useDataArea";

export const projectPt: ProjectInfo = { root: "/pt", game_code: "CPUD", family: "Pt", language: "de", header_count: 3 };
export const projectDp: ProjectInfo = { root: "/dp", game_code: "ADAD", family: "Dp", language: "de", header_count: 3 };

const tech = (index: number) => String(index).padStart(4, "0");
export const species = (index: number, name: string, types: [number, number] = [0, 0]): DataRowDto => ({
  index,
  name,
  tech: tech(index),
  locked: null,
  extra: { types, number: index, stats: [50, 50, 50, 50, 50, 50] },
});
export const move = (index: number, name: string, type = 0): DataRowDto => ({ index, name, tech: tech(index), locked: null, extra: { type, category: 0, power: 40, accuracy: 100, pp: 35 } });
export const item = (index: number, name: string, pocket: number, icon = true): DataRowDto => ({
  index,
  name,
  tech: tech(index),
  locked: null,
  extra: { price: 100 * index, field_pocket: pocket, icon, data_index: index },
});

export const SPECIES = [species(1, "Bisasam", [12, 3]), species(396, "Staralili", [0, 2]), species(397, "Staravia", [0, 2]), species(520, "species 0520")];
export const MOVES = [move(17, "Flügelschlag", 2), move(33, "Tackle")];
export const ITEMS = [item(1, "Meisterball", 2), item(4, "Pokéball", 2), item(17, "Trank", 1)];
/** The text archives of the name tables of Platinum (the move and item descriptions are 646 and 391). */
export const PT_ARCHIVES: NameArchivesDto = { species: 412, moves: 647, items: 392, abilities: 610, types: 624, classes: 619, forms: 697 };
export const NAMES: NameTablesDto = { species: [], moves: [], items: [], abilities: [], types: [], classes: [], forms: [], archives: PT_ARCHIVES };
/** Item ids 1 and 4 share icon and palette; 17 has its own. */
export const ITEM_TABLE: ItemEntryDto[] = Array.from({ length: 18 }, (_, id) => ({ data: id, icon: id === 17 ? 30 : 5, palette: id === 17 ? 31 : 6, gba: 0 }));

export const flush = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));

/** Opens a Base UI menu the way a pointer does and returns its items (plain and radio). */
export function openMenu(trigger: HTMLElement): HTMLElement[] {
  act(() => {
    trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
    trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
    trigger.click();
  });
  return [...document.querySelectorAll<HTMLElement>("[role=menuitem], [role=menuitemradio]")];
}

/**
 * A data area with its parts in a fresh container, inside a shell. `tree` puts the area around its parts; `base` builds the props with a
 * fresh store and mock callbacks. The crumbs the area reported last are in `trail()`.
 */
export function dataHarness(tree: (props: DataAreaProps) => ReactNode) {
  localStorage.clear();
  const container = document.createElement("div");
  document.body.append(container);
  const root: Root = createRoot(container);
  const store = new DataStore();
  const trails: Crumb[][] = [];
  const bus = createEditBus();
  const calls = { goTo: vi.fn(), onTrail: vi.fn((crumbs: Crumb[]) => void trails.push(crumbs)), onEdited: vi.fn() };
  const base = (over: Partial<DataAreaProps> = {}): DataAreaProps => ({ project: projectPt, store, active: true, target: null, dirty: [], bus, headers: [], ...calls, ...over });
  const show = (props: DataAreaProps) => act(() => root.render(<ShellProvider initialArea="pokemon">{tree(props)}</ShellProvider>));
  const q = <E extends Element = HTMLElement>(selector: string) => container.querySelector<E>(selector);
  const buttons = () => [...container.querySelectorAll<HTMLButtonElement>("button")];
  const button = (text: string) => buttons().find((b) => (b.getAttribute("aria-label") ?? b.textContent ?? "").trim() === text);
  const trail = () => (trails[trails.length - 1] ?? []).map((c) => c.label);
  const lastTrail = () => trails[trails.length - 1] ?? [];
  const unmount = () => {
    act(() => root.unmount());
    container.remove();
  };
  return { container, store, bus, calls, base, show, q, button, buttons, trail, lastTrail, unmount };
}
