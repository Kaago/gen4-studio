// Helpers for the tests of the inspector editors: the actions of the inspector head live in its menu „Aktionen“.
import { act } from "react";
import { openSelect } from "../../ui/primitives/testUtils";

/** The menu button „Aktionen“ of the inspector head under `root`; null when the head has no actions. */
export function actionsButton(root: ParentNode): HTMLButtonElement | null {
  return root.querySelector<HTMLButtonElement>("header[data-inspector-kind] button[aria-label='Aktionen']");
}

/** The open menu's items (Base UI renders the popup into the document body). */
function menuItems(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>("[role=menu] [role=menuitem]")];
}

/** Opens the head's menu and returns the text of its items (label and detail), then closes it again. Empty without a menu. */
export function headActions(root: ParentNode): string[] {
  const button = actionsButton(root);
  if (button === null) return [];
  openSelect(button);
  const texts = menuItems().map((item) => item.textContent ?? "");
  act(() => {
    document.querySelector<HTMLElement>("[role=menu]")?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
  });
  return texts;
}

/** The item `label` of the head's menu, the menu opened (undefined when there is no such item or no menu). */
export function headAction(root: ParentNode, label: string): HTMLElement | undefined {
  const button = actionsButton(root);
  if (button === null) return undefined;
  if (button.getAttribute("aria-expanded") !== "true") openSelect(button);
  return menuItems().find((item) => item.querySelector("span")?.textContent === label);
}

/** Opens the head's menu and picks the item `label`; fails when there is none. */
export function chooseAction(root: ParentNode, label: string): void {
  const item = headAction(root, label);
  if (item === undefined) throw new Error(`no action „${label}“ in the inspector head`);
  act(() => item.click());
}
