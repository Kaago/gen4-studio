// Helpers for the tests of the places offering „Auf Original zurücksetzen…“: a reset api with a spy, the status of a found and of a
// missing base ROM, and the ways a user opens a menu „Aktionen“ and a row's context menu.
import { act } from "react";
import type { ReactNode } from "react";
import { vi } from "vitest";
import type { Mock } from "vitest";
import type { OriginalStatus } from "../api";
import { openSelect } from "../ui/primitives/testUtils";
import { ResetProvider } from "./OriginalContext";
import type { ResetApi } from "./OriginalContext";

/** A found base ROM; each kind has the entries a test needs below its count („Kein Original“ from the count on). */
export const READY: Extract<OriginalStatus, { state: "ready" }> = {
  state: "ready",
  label: "Platin DE (Testprojekt)",
  sha1: "d40f19ac24d67c721a4ab25e1c013154443d75b4",
  counts: {
    map: 600,
    event_file: 600,
    header: 500,
    text_archive: 600,
    trainer: 900,
    species: 600,
    encounter: 180,
    script: 1200,
    map_texture: 80,
    building_texture: 80,
    building_model: 600,
    interior_building_model: 300,
  },
};

/** A found base ROM with only these counts (a kind left out counts 0). */
export const readyWith = (counts: Record<string, number>): OriginalStatus => ({ ...READY, counts });

/** No base ROM: every entry is locked with „Basis-ROM fehlt“. */
export const MISSING: OriginalStatus = { state: "unavailable", reason: "Diesem Projekt ist keine Basis-ROM zugeordnet." };

/** A reset api whose `open` is a spy; `status` can be changed before the next render. */
export function resetSpy(status: OriginalStatus | null = READY): ResetApi & { open: Mock<ResetApi["open"]> } {
  return { status, open: vi.fn<ResetApi["open"]>() };
}

/** `node` below `api` (a fresh object each time, so a changed status reaches every `useReset`). */
export function withReset(api: ResetApi, node: ReactNode): ReactNode {
  return <ResetProvider value={{ status: api.status, open: api.open }}>{node}</ResetProvider>;
}

/** The items of the open menu (Base UI renders the popup into the document body). */
function menuItems(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>("[role=menu] [role=menuitem]")];
}

/** Closes the open menu (Escape). */
export function closeMenu() {
  act(() => {
    document.querySelector<HTMLElement>("[role=menu]")?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
  });
}

/** The menu button „Aktionen“ under `root`; null when there is none. */
export function actionsMenu(root: ParentNode): HTMLButtonElement | null {
  return root.querySelector<HTMLButtonElement>("button[aria-label='Aktionen']");
}

/** Opens the menu „Aktionen“ under `root` and returns the text of its items (label and detail), then closes it. Empty without a menu. */
export function actionTexts(root: ParentNode): string[] {
  const button = actionsMenu(root);
  if (button === null) return [];
  openSelect(button);
  const texts = menuItems().map((item) => item.textContent ?? "");
  closeMenu();
  return texts;
}

/** The item `label` of the menu „Aktionen“ under `root`, the menu opened (undefined when there is no such item or no menu). */
export function actionItem(root: ParentNode, label: string): HTMLElement | undefined {
  const button = actionsMenu(root);
  if (button === null) return undefined;
  if (button.getAttribute("aria-expanded") !== "true") openSelect(button);
  return menuItems().find((item) => item.querySelector("span")?.textContent === label);
}

/** Opens the menu „Aktionen“ under `root` and picks the item `label`; fails when there is none. */
export function chooseItem(root: ParentNode, label: string): void {
  const item = actionItem(root, label);
  if (item === undefined) throw new Error(`no item „${label}“ in the menu „Aktionen“`);
  act(() => item.click());
}

/** Opens the context menu of `row` with a right click and returns the text of its items, then closes it. */
export function contextTexts(row: HTMLElement): string[] {
  rightClick(row);
  const texts = menuItems().map((item) => item.textContent ?? "");
  closeMenu();
  return texts;
}

/** Opens the context menu of `row` and picks the item `label`; fails when there is none. */
export function chooseContext(row: HTMLElement, label: string): void {
  rightClick(row);
  const item = menuItems().find((i) => i.querySelector("span")?.textContent === label);
  if (item === undefined) throw new Error(`no context menu item „${label}“`);
  act(() => item.click());
}

/** Whether a menu item is disabled (Base UI marks it both ways). */
export const isDisabled = (item: HTMLElement | undefined) => item?.getAttribute("aria-disabled") === "true" || (item?.hasAttribute("data-disabled") ?? false);

function rightClick(row: HTMLElement) {
  act(() => void row.dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, button: 2, clientX: 10, clientY: 10 })));
}
