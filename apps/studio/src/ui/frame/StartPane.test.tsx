// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { pressKey } from "../primitives/testUtils";
import { StartPane } from "./StartPane";
import type { StartPaneProps } from "./StartPane";
import type { RecentProject } from "./shellState";
import type { PreviewDto } from "../../api";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const HOUR = 3_600_000;
const project = (name: string, age: number, game = "Platin, DE"): RecentProject => ({ path: `/roms/${name}`, name, game, openedAt: Date.now() - age });
const preview = (source: "eigen" | "standard"): PreviewDto => ({ source, png: "iVBORw0KGgo=", standardMap: null });

describe("StartPane", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = { onOpen: vi.fn(), onOpenRecent: vi.fn(), onForget: vi.fn(), onChoosePreview: vi.fn(), onRemovePreview: vi.fn() };
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
    act(() => root.render(<StartPane recent={[]} missing={new Set()} previews={new Map()} {...handlers} {...props} />));
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
  /** Opens the "…" menu of a card and answers the labels of its entries. */
  const menuOf = (name: string) => {
    pressPointer(button(`Weitere Aktionen für ${name}`)!);
    return [...document.querySelectorAll<HTMLElement>("[role=menuitem]")];
  };
  const pictureOf = (path: string) => card(path)?.closest("li")?.querySelector("img") ?? null;

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
    expect(card("/roms/pt-a")?.closest("li")?.querySelector('[aria-label="DS-Bildschirm, leer"]')).not.toBeNull();
  });

  it("names_the_open_button_by_the_project_and_describes_it_by_game_and_time", () => {
    render({ recent: [project("pt-a", 2 * HOUR + 5_000)] });
    const open = card("/roms/pt-a")!;
    const textOf = (ids: string | null) => (ids ?? "").split(" ").map((id) => document.getElementById(id)?.textContent ?? "").join(" ");
    expect(textOf(open.getAttribute("aria-labelledby"))).toBe("pt-a");
    const description = textOf(open.getAttribute("aria-describedby"));
    expect(description).toContain("Platin, DE");
    expect(description).toContain("geöffnet vor 2 Stunden");
    expect(description).toContain("Zuletzt");
    // Only phrasing content inside the button: the DS screen (blocks) is beside it.
    expect(open.querySelector("div")).toBeNull();
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

  it("card shows the picture when a preview is there", () => {
    render({ recent: [project("pt-a", HOUR)], previews: new Map([["/roms/pt-a", preview("standard")]]) });
    const img = pictureOf("/roms/pt-a");
    expect(img).not.toBeNull();
    // Decorative: the open button stays named by the project.
    expect(img!.getAttribute("alt")).toBe("");
    expect(img!.getAttribute("src")).toMatch(/^data:image\/png;base64,/);
    expect(img!.getAttribute("src")).toBe("data:image/png;base64,iVBORw0KGgo=");
    // The picture lies on the DS screen, where the name was.
    expect(img!.closest('[aria-label="DS-Bildschirm, leer"]')).not.toBeNull();
    expect(card("/roms/pt-a")?.closest("li")?.querySelector("[data-preview-name]")).toBeNull();
    const textOf = (ids: string | null) => (ids ?? "").split(" ").map((id) => document.getElementById(id)?.textContent ?? "").join(" ");
    expect(textOf(card("/roms/pt-a")!.getAttribute("aria-labelledby"))).toBe("pt-a");
  });

  it("card shows the name without a preview", () => {
    render({ recent: [project("pt-a", HOUR), project("hg-b", 2 * HOUR)], previews: new Map([["/roms/hg-b", { source: null, png: null, standardMap: null }]]) });
    for (const path of ["/roms/pt-a", "/roms/hg-b"]) {
      const li = card(path)!.closest("li")!;
      expect(li.querySelector("img")).toBeNull();
      expect(li.querySelector("[data-preview-name]")?.textContent).toBe(path.slice("/roms/".length));
      expect(li.querySelector('[aria-label="DS-Bildschirm, leer"]')).not.toBeNull();
    }
  });

  it('missing card stays grey and has only "Aus der Liste nehmen"', () => {
    render({ recent: [project("gone", HOUR)], missing: new Set(["/roms/gone"]), previews: new Map([["/roms/gone", preview("eigen")]]) });
    const li = card("/roms/gone")!.closest("li")!;
    expect(li.hasAttribute("data-missing")).toBe(true);
    // A picture loaded before it went missing is not shown: the card is the grey placeholder.
    expect(li.querySelector("img")).toBeNull();
    expect(li.querySelector("[data-preview-name]")?.textContent).toBe("gone");
    expect(menuOf("gone").map((i) => i.textContent)).toEqual(["Aus der Liste nehmen"]);
    expect(document.querySelector("[role=separator]")).toBeNull();
  });

  it('menu offers "Vorschaubild wählen…", "Vorschaubild entfernen" only for eigen', () => {
    render({
      recent: [project("own", HOUR), project("std", 2 * HOUR), project("none", 3 * HOUR)],
      previews: new Map([
        ["/roms/own", preview("eigen")],
        ["/roms/std", preview("standard")],
      ]),
    });
    const own = menuOf("own");
    expect(own.map((i) => i.textContent)).toEqual(["Vorschaubild wählen…", "Vorschaubild entfernen", "Aus der Liste nehmen"]);
    // The separator stands between the picture's entries and leaving the list.
    const popup = own[0]!.parentElement!;
    expect([...popup.children].map((c) => c.getAttribute("role"))).toEqual(["menuitem", "menuitem", "separator", "menuitem"]);
    act(() => own[1]!.click());
    expect(handlers.onRemovePreview).toHaveBeenCalledWith("/roms/own");
    expect(menuOf("std").map((i) => i.textContent)).toEqual(["Vorschaubild wählen…", "Aus der Liste nehmen"]);
    const none = menuOf("none");
    expect(none.map((i) => i.textContent)).toEqual(["Vorschaubild wählen…", "Aus der Liste nehmen"]);
    act(() => none[0]!.click());
    expect(handlers.onChoosePreview).toHaveBeenCalledWith("/roms/none");
    expect(handlers.onOpenRecent).not.toHaveBeenCalled();
  });

  it('"Zuletzt" stays on the newest card', () => {
    render({
      recent: [project("older", 3 * HOUR), project("newest", 60_000)],
      previews: new Map([
        ["/roms/older", preview("eigen")],
        ["/roms/newest", preview("standard")],
      ]),
    });
    const marked = cards().filter((c) => c.textContent?.includes("Zuletzt"));
    expect(marked.map((c) => c.dataset.recent)).toEqual(["/roms/newest"]);
    expect(pictureOf("/roms/newest")).not.toBeNull();
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

  it("new_game_is_the_primary_action", () => {
    const onNewGame = vi.fn();
    render({ onNewGame });
    const actions = [...container.querySelectorAll<HTMLButtonElement>("[data-start-actions] button")];
    expect(actions.map((b) => b.textContent)).toEqual(["Neues Spiel aus ROM…", "Projekt öffnen⌘O"]);
    expect(actions.map((b) => b.dataset.kind)).toEqual(["primary", "secondary"]);
    // Only one primary button in the view.
    expect(container.querySelectorAll('button[data-kind="primary"]')).toHaveLength(1);
    expect(container.textContent).toContain("Du brauchst deine eigene ROM-Datei (.nds).");
    expect(container.textContent).toContain("Gen4 Studio liefert keine Spieldaten; du nutzt deine eigene ROM.");
    act(() => actions[0]!.click());
    expect(onNewGame).toHaveBeenCalledOnce();
    act(() => actions[1]!.click());
    expect(handlers.onOpen).toHaveBeenCalledOnce();
  });

  it("without_a_new_game_opening_a_project_stays_the_primary_action", () => {
    render();
    const actions = [...container.querySelectorAll<HTMLButtonElement>("[data-start-actions] button")];
    expect(actions.map((b) => b.dataset.kind)).toEqual(["primary"]);
    expect(container.textContent).not.toContain("Neues Spiel aus ROM");
    expect(container.textContent).not.toContain("eigene ROM");
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
