/**
 * Makes a dock's tab menu (dockview's `.dv-context-menu`, built for the mouse) work with the keyboard, as a menu does (ARIA menu pattern): it
 * takes the focus on its first item that is not locked; Arrow down and up walk all items (round), the locked ones too, so a screen reader
 * reads them and their reason; Home and End jump to the ends; Enter or Space runs the item, never a locked one (the menu then stays open).
 * Escape stays dockview's (a window listener closes the menu); `back` gives the focus back to where the menu was opened, also after an item ran.
 */
export function workMenuByKeyboard(menu: HTMLElement, back: () => void): void {
  const items = () => [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')];
  const locked = (item: HTMLElement | undefined) => item?.getAttribute("aria-disabled") === "true";
  for (const item of items()) item.tabIndex = -1;
  items().find((item) => !locked(item))?.focus();
  menu.addEventListener("keydown", (event) => {
    const list = items();
    const at = list.indexOf(document.activeElement as HTMLElement);
    const go = (index: number) => list[(index + list.length) % list.length]?.focus();
    switch (event.key) {
      case "ArrowDown":
        go(at + 1);
        break;
      case "ArrowUp":
        go(at < 0 ? -1 : at - 1);
        break;
      case "Home":
        go(0);
        break;
      case "End":
        go(-1);
        break;
      case "Enter":
      case " ":
        if (locked(list[at])) break;
        list[at]?.click();
        back();
        break;
      case "Escape":
        // dockview closes the menu on its own; the focus goes back first, so it does not fall to the page.
        back();
        return;
      default:
        return;
    }
    event.preventDefault();
    event.stopPropagation();
  });
}
