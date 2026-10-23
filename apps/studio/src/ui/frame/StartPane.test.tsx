// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { pressKey } from "../primitives/testUtils";
import { StartPane } from "./StartPane";
import type { StartPaneProps } from "./StartPane";
import type { RecentProject } from "./shellState";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const HOUR = 3_600_000;
const project = (name: string, age: number, game = "Platin, DE"): RecentProject => ({ path: `/roms/${name}`, name, game, openedAt: Date.now() - age });

describe("StartPane", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = { onOpen: vi.fn(), onOpenRecent: vi.fn(), onForget: vi.fn() };
  beforeEach(() => {
    for (const fn of Object.values(handlers)) fn.mockClear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const render = (props: Partial<StartPaneProps> = {}) =>
    act(() => root.render(<StartPane recent={[]} missing={new Set()} {...handlers} {...props} />));
  const card = (path: string) => container.querySelector<HTMLElement>(`[data-recent="${path}"]`);
  const cards = () => [...container.querySelectorAll<HTMLElement>("[data-recent]")];
  const button = (label: string) =>
    [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === label || b.getAttribute("aria-label") === label);
  const pressPointer = (el: HTMLElement) =>
    act(() => {
      el.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      el.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      el.click();
    });

  it("shows_each_recent_project_as_a_card_with_an_empty_ds_screen_its_game_and_when_it_was_opened", () => {
    render({ recent: [project("pt-a", 2 * HOUR + 5_000), project("hg-b", 5 * 60_000 + 5_000, "HeartGold/SoulSilver, EN")] });
    expect(container.querySelector("[role=list]")?.getAttribute("aria-label")).toBe("Zuletzt geöffnete Projekte");
    expect(cards()).toHaveLength(2);
    expect(card("/roms/pt-a")?.textContent).toContain("pt-a");
    expect(card("/roms/pt-a")?.textContent).toContain("Platin, DE");
    expect(card("/roms/pt-a")?.textContent).toContain("geöffnet vor 2 Stunden");
    expect(card("/roms/hg-b")?.textContent).toContain("geöffnet vor 5 Minuten");
    // The path is the card's tooltip; the preview is an empty DS screen.
    expect(card("/roms/pt-a")?.getAttribute("title")).toBe("/roms/pt-a");
    expect(card("/roms/pt-a")?.querySelector('[aria-label="DS-Bildschirm, leer"]')).not.toBeNull();
  });

  it("marks_only_the_newest_card_as_zuletzt", () => {
    render({ recent: [project("older", 3 * HOUR), project("newest", 60_000), project("oldest", 30 * HOUR)] });
    const marked = cards().filter((c) => c.textContent?.includes("Zuletzt"));
    expect(marked.map((c) => c.dataset.recent)).toEqual(["/roms/newest"]);
  });

  it("a_click_on_a_card_opens_its_project", () => {
    render({ recent: [project("pt-a", HOUR)] });
    act(() => card("/roms/pt-a")!.click());
    expect(handlers.onOpenRecent).toHaveBeenCalledWith("/roms/pt-a");
  });

  it("the_more_menu_of_a_card_takes_it_off_the_list", () => {
    render({ recent: [project("pt-a", HOUR), project("hg-b", 2 * HOUR)] });
    pressPointer(button("Weitere Aktionen für hg-b")!);
    const item = [...document.querySelectorAll<HTMLElement>("[role=menuitem]")].find((i) => i.textContent === "Aus der Liste nehmen");
    expect(item).toBeDefined();
    act(() => item!.click());
    expect(handlers.onForget).toHaveBeenCalledWith("/roms/hg-b");
    expect(handlers.onOpenRecent).not.toHaveBeenCalled();
  });

  it("a_card_says_nicht_gefunden_only_after_it_failed_to_open_and_offers_to_leave_the_list", () => {
    render({ recent: [project("gone", HOUR), project("here", 2 * HOUR)] });
    expect(container.textContent).not.toContain("nicht gefunden");
    render({ recent: [project("gone", HOUR), project("here", 2 * HOUR)], missing: new Set(["/roms/gone"]) });
    expect(card("/roms/gone")?.textContent).toContain("nicht gefunden");
    expect(card("/roms/gone")?.closest("[data-missing]")).not.toBeNull();
    expect(card("/roms/here")?.textContent).not.toContain("nicht gefunden");
    act(() => button("Aus der Liste nehmen")!.click());
    expect(handlers.onForget).toHaveBeenCalledWith("/roms/gone");
  });

  it("the_arrow_keys_move_over_the_cards_with_one_tab_stop", () => {
    render({ recent: [project("a", HOUR), project("b", 2 * HOUR), project("c", 3 * HOUR)] });
    const tabStops = () => cards().filter((c) => c.tabIndex === 0).map((c) => c.dataset.recent);
    expect(tabStops()).toEqual(["/roms/a"]);
    card("/roms/a")!.focus();
    pressKey(card("/roms/a")!, "ArrowRight");
    expect(document.activeElement).toBe(card("/roms/b"));
    expect(tabStops()).toEqual(["/roms/b"]);
    pressKey(card("/roms/b")!, "End");
    expect(document.activeElement).toBe(card("/roms/c"));
  });

  it("opens_another_project_with_the_primary_button_that_shows_cmd_o", () => {
    render();
    expect(container.textContent).toContain("Noch kein Projekt geöffnet.");
    expect(container.textContent).toContain("Projekte sind entpackte DSPRE-Ordner.");
    const open = button("Projekt öffnen⌘O");
    expect(open).toBeDefined();
    act(() => open!.click());
    expect(handlers.onOpen).toHaveBeenCalledOnce();
  });
});
