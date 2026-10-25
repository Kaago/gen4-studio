// @vitest-environment jsdom
import type { DockviewApi, IDockviewPanelProps } from "dockview-react";
import { act } from "react";
import type { FunctionComponent } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { DECK_PANELS, deckLayoutKey } from "../../workbench/layout";
import { scriptDirtyStore } from "../../workbench/script/scriptDirtyStore";
import type { PanelId } from "../../workbench/layout";
import { Deck, resetLayout } from "./Deck";
import { ShellProvider, useShell } from "./ShellContext";
import type { ShellApi } from "./ShellContext";
import { HAND_DEFAULT, readShellState, writeShellState } from "./shellState";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;
// jsdom has no ResizeObserver; dockview watches its container with one.
globalThis.ResizeObserver = class {
  observe() {}
  unobserve() {}
  disconnect() {}
} as unknown as typeof ResizeObserver;

interface Node {
  type: "leaf" | "branch";
  data: Node[] | { views: string[] };
}
/** The groups of a serialized grid in layout order, each as its list of panel ids. */
const groups = (node: Node): string[][] => (node.type === "leaf" ? [(node.data as { views: string[] }).views] : (node.data as Node[]).flatMap(groups));
const layoutOf = (api: DockviewApi) => groups(api.toJSON().grid.root as unknown as Node);
/** Lets dockview announce what it changed: its layout event comes a tick after the change. */
const settle = () => act(async () => void (await new Promise((resolve) => setTimeout(resolve, 0))));
/** Moves a panel into a group of its own, to the right of everything. */
const split = async (api: DockviewApi, panel: string) => {
  act(() => void api.getPanel(panel)!.api.moveTo({ group: api.addGroup({ direction: "right" }) }));
  await settle();
};

const leaf = (...views: string[]) => ({ type: "leaf", data: { views, activeView: views[0], id: views.join("+") }, size: 100 });
/** A serialized dockview layout with these groups side by side. */
const stored = (...grouped: string[][]) => ({
  grid: { root: { type: "branch", data: grouped.map((g) => leaf(...g)) }, width: 1000, height: 400, orientation: "HORIZONTAL" },
  panels: Object.fromEntries(grouped.flat().map((id) => [id, { id, contentComponent: id, title: id }])),
});

const Empty: FunctionComponent<IDockviewPanelProps> = () => null;
const components = (ids: PanelId[]) => Object.fromEntries(ids.map((id) => [id, Empty]));
const SCHAU = components(["world", "matrix", "worldmap"]);
const HAND = components(["inspector", "script", "problems"]);
const TEXT_SCHAU = components(["textView"]);
const TEXT_HAND = components(["textEdit", "problems"]);

describe("Deck", () => {
  let container: HTMLElement;
  let root: Root;
  let shell: ShellApi;
  const schauRef = { current: null as DockviewApi | null };
  const handRef = { current: null as DockviewApi | null };

  function Probe() {
    shell = useShell();
    return null;
  }
  /** Renders the deck (again: the docks stay); `wizard` as App gives it to the world's deck. */
  const render = async (area: "welt" | "texte", wizard?: boolean) => {
    const [schauComponents, handComponents] = area === "welt" ? [SCHAU, HAND] : [TEXT_SCHAU, TEXT_HAND];
    await act(async () =>
      root.render(
        <ShellProvider initialArea={area}>
          <Probe />
          <Deck area={area} schauComponents={schauComponents} handComponents={handComponents} schauRef={schauRef} handRef={handRef} hinge={<div role="toolbar" aria-label="Scharnier" />} wizard={wizard} />
        </ShellProvider>,
      ),
    );
  };
  const mount = async (area: "welt" | "texte" = "welt", wizard?: boolean) => {
    schauRef.current = null;
    handRef.current = null;
    await render(area, wizard);
  };
  const schau = () => schauRef.current!;
  const hand = () => handRef.current!;
  const dock = (which: "schau" | "hand") => container.querySelector<HTMLElement>(`[data-dock="${which}"]`)!;
  const headerHidden = (api: DockviewApi, panel: string) => api.getPanel(panel)!.group.header.hidden;

  beforeEach(() => {
    localStorage.clear();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    scriptDirtyStore.set(0);
  });

  it("renders_two_docks_each_in_its_own_dockview", async () => {
    await mount();
    expect(dock("schau")).not.toBeNull();
    expect(dock("hand")).not.toBeNull();
    // Each dock is its own dockview in its own container; the panels of one are not in the other.
    expect(dock("schau").contains(dock("hand"))).toBe(false);
    expect(schau()).not.toBe(hand());
    // Dockview numbers its instances; two in one document must not share an id (drag and drop tells them apart by it).
    expect(schau().id).not.toBe(hand().id);
  });

  it("puts_the_hinge_above_the_view_and_the_splitter_between_view_and_hand", async () => {
    await mount();
    const hinge = container.querySelector('[role="toolbar"]')!;
    const splitter = container.querySelector('[data-splitter="hand"]')!;
    const follows = (a: Element, b: Element) => Boolean(a.compareDocumentPosition(b) & Node.DOCUMENT_POSITION_FOLLOWING);
    expect(follows(hinge, dock("schau"))).toBe(true);
    expect(follows(dock("schau"), splitter)).toBe(true);
    expect(follows(splitter, dock("hand"))).toBe(true);
    expect(splitter.getAttribute("aria-controls")).toBe("dock-welt-hand");
  });

  it("hides_the_splitter_behind_an_open_wizard", async () => {
    await mount("welt", false);
    expect(container.querySelector('[data-splitter="hand"]')).not.toBeNull();
    await render("welt", true);
    expect(container.querySelector('[data-splitter="hand"]')).toBeNull();
    await render("welt", false);
    expect(container.querySelector('[data-splitter="hand"]')).not.toBeNull();
  });

  it("builds_the_default_layouts_of_both_docks", async () => {
    await mount();
    expect(layoutOf(schau())).toEqual([["world", "matrix", "worldmap"]]);
    expect(layoutOf(hand())).toEqual([["inspector", "script", "problems"]]);
    expect(schau().panels.map((p) => p.id).sort()).toEqual(DECK_PANELS.welt.schau.map((p) => p.id).sort());
    expect(hand().panels.map((p) => p.id).sort()).toEqual(DECK_PANELS.welt.hand.map((p) => p.id).sort());
    // The scene is what the view opens with, the inspector what the hand opens with.
    expect(schau().activePanel?.id).toBe("world");
    expect(hand().activePanel?.id).toBe("inspector");
  });

  it("restores_a_stored_layout_of_each_dock", async () => {
    localStorage.setItem(deckLayoutKey("welt", "schau"), JSON.stringify(stored(["world", "matrix"], ["worldmap"])));
    localStorage.setItem(deckLayoutKey("welt", "hand"), JSON.stringify(stored(["inspector"], ["script", "problems"])));
    await mount();
    expect(layoutOf(schau())).toEqual([["world", "matrix"], ["worldmap"]]);
    expect(layoutOf(hand())).toEqual([["inspector"], ["script", "problems"]]);
  });

  it("resets_only_the_view_when_its_stored_layout_is_unusable", async () => {
    // The view's layout lacks a panel (a layout of another panel set); the hand's was arranged by the user.
    localStorage.setItem(deckLayoutKey("welt", "schau"), JSON.stringify(stored(["world", "matrix"])));
    localStorage.setItem(deckLayoutKey("welt", "hand"), JSON.stringify(stored(["inspector"], ["script", "problems"])));
    await mount();
    expect(layoutOf(schau())).toEqual([["world", "matrix", "worldmap"]]);
    expect(layoutOf(hand())).toEqual([["inspector"], ["script", "problems"]]);
  });

  it("resets_only_the_hand_when_its_stored_json_is_broken_and_keeps_the_view", async () => {
    localStorage.setItem(deckLayoutKey("welt", "schau"), JSON.stringify(stored(["world"], ["matrix", "worldmap"])));
    localStorage.setItem(deckLayoutKey("welt", "hand"), "{not json");
    await mount();
    expect(layoutOf(schau())).toEqual([["world"], ["matrix", "worldmap"]]);
    expect(layoutOf(hand())).toEqual([["inspector", "script", "problems"]]);
  });

  it("falls_back_to_the_default_of_a_dock_whose_stored_grid_cannot_be_restored", async () => {
    // Right panels, but a grid dockview refuses.
    const broken = { ...stored(["world", "matrix", "worldmap"]), grid: { root: { type: "nonsense" } } };
    localStorage.setItem(deckLayoutKey("welt", "schau"), JSON.stringify(broken));
    localStorage.setItem(deckLayoutKey("welt", "hand"), JSON.stringify(stored(["inspector"], ["script", "problems"])));
    await mount();
    expect(layoutOf(schau())).toEqual([["world", "matrix", "worldmap"]]);
    expect(layoutOf(hand())).toEqual([["inspector"], ["script", "problems"]]);
  });

  it("falls_back_to_the_default_of_a_world_hand_that_still_has_the_text_panel", async () => {
    // What v7 stored: the text panel was a tab of the world's hand. Under the old key and under the new one it is not restored.
    const v7 = JSON.stringify(stored(["tools"], ["inspector", "text"], ["script", "problems"]));
    localStorage.setItem("g4.layout.v7.welt.hand", v7);
    localStorage.setItem(deckLayoutKey("welt", "hand"), v7);
    await mount();
    expect(layoutOf(hand())).toEqual([["inspector", "script", "problems"]]);
    expect(hand().getPanel("text")).toBeUndefined();
  });

  it("falls_back_to_the_default_of_a_world_hand_that_still_has_the_tools", async () => {
    // What v8 stored: the tools were a group left of the inspector. Under the new key they are not restored.
    localStorage.setItem(deckLayoutKey("welt", "hand"), JSON.stringify(stored(["tools"], ["inspector"], ["script", "problems"])));
    await mount();
    expect(layoutOf(hand())).toEqual([["inspector", "script", "problems"]]);
    expect(hand().getPanel("tools")).toBeUndefined();
  });

  it("falls_back_to_the_default_of_a_world_hand_that_still_has_the_context", async () => {
    // What v9 stored: „An diesem Ort“ was a group right of the inspector. Under the new key it is not restored.
    localStorage.setItem(deckLayoutKey("welt", "hand"), JSON.stringify(stored(["inspector", "script", "problems"], ["context"])));
    await mount();
    expect(layoutOf(hand())).toEqual([["inspector", "script", "problems"]]);
    expect(hand().getPanel("context")).toBeUndefined();
  });

  it("ignores_the_layout_stored_by_the_single_dock_version", async () => {
    localStorage.setItem("g4.layout.v5", JSON.stringify(stored(["world", "matrix", "worldmap"])));
    await mount();
    expect(layoutOf(schau())).toEqual([["world", "matrix", "worldmap"]]);
  });

  it("remembers_each_dock_under_its_own_key", async () => {
    await mount();
    await split(schau(), "worldmap");
    const savedSchau = JSON.parse(localStorage.getItem(deckLayoutKey("welt", "schau"))!);
    expect(Object.keys(savedSchau.panels).sort()).toEqual(["matrix", "world", "worldmap"]);
    expect(groups(savedSchau.grid.root)).toEqual([["world", "matrix"], ["worldmap"]]);
    // The hand's key still holds the hand's panels and nothing of the view.
    const savedHand = JSON.parse(localStorage.getItem(deckLayoutKey("welt", "hand"))!);
    expect(Object.keys(savedHand.panels).sort()).toEqual(["inspector", "problems", "script"]);
  });

  it("hides_the_tab_bar_of_a_group_with_one_panel_and_shows_it_from_two_panels", async () => {
    await mount();
    // Three panels in one group: tab bar.
    expect(headerHidden(schau(), "world")).toBe(false);
    // Moving one out leaves a group of one (no tab bar) and a group of two (tab bar).
    await split(schau(), "worldmap");
    expect(headerHidden(schau(), "worldmap")).toBe(true);
    expect(headerHidden(schau(), "world")).toBe(false);
    expect(headerHidden(schau(), "matrix")).toBe(false);
    // The hand is not touched by the view's change.
    expect(headerHidden(hand(), "inspector")).toBe(false);
    // Another one out: two groups of one, one of ... world alone, matrix alone, worldmap alone.
    await split(schau(), "matrix");
    expect(["world", "matrix", "worldmap"].map((p) => headerHidden(schau(), p))).toEqual([true, true, true]);
    // Back into one group: the tab bar returns.
    act(() => void schau().getPanel("matrix")!.api.moveTo({ group: schau().getPanel("world")!.group }));
    await settle();
    expect(headerHidden(schau(), "world")).toBe(false);
    expect(headerHidden(schau(), "matrix")).toBe(false);
    expect(headerHidden(schau(), "worldmap")).toBe(true);
  });

  it("hides_the_tab_bar_of_a_restored_group_with_one_panel", async () => {
    localStorage.setItem(deckLayoutKey("welt", "hand"), JSON.stringify(stored(["inspector", "script"], ["problems"])));
    await mount();
    expect(headerHidden(hand(), "problems")).toBe(true);
    expect(headerHidden(hand(), "inspector")).toBe(false);
  });

  it("gives_the_hand_the_remembered_height_and_the_view_the_rest", async () => {
    await mount();
    expect(dock("hand").style.height).toBe(`${HAND_DEFAULT}px`);
    expect(dock("hand").hidden).toBe(false);
    act(() => shell.setHandHeight("welt", 400));
    expect(dock("hand").style.height).toBe("400px");
    expect(dock("schau").style.height).toBe("");
  });

  it("hides_a_collapsed_hand_but_keeps_its_dock_mounted_and_remembers_the_height", async () => {
    writeShellState({ ...readShellState(), handHeight: { ...readShellState().handHeight, welt: 333 } });
    await mount();
    const before = hand();
    act(() => shell.toggleHandCollapsed("welt"));
    expect(dock("hand").hidden).toBe(true);
    expect(hand()).toBe(before);
    expect(hand().panels).toHaveLength(DECK_PANELS.welt.hand.length);
    act(() => shell.toggleHandCollapsed("welt"));
    expect(dock("hand").hidden).toBe(false);
    expect(dock("hand").style.height).toBe("333px");
  });

  it("does_not_rebuild_a_dock_when_the_frame_state_changes", async () => {
    await mount();
    const [a, b] = [schau(), hand()];
    act(() => shell.setTool("paint"));
    act(() => shell.setHandHeight("welt", 300));
    expect(schauRef.current).toBe(a);
    expect(handRef.current).toBe(b);
  });

  it("builds_the_default_layouts_of_the_text_area_under_its_own_keys", async () => {
    await mount("texte");
    expect(layoutOf(schau())).toEqual([["textView"]]);
    expect(layoutOf(hand())).toEqual([["textEdit", "problems"]]);
    expect(hand().activePanel?.id).toBe("textEdit");
    expect(Object.keys(JSON.parse(localStorage.getItem(deckLayoutKey("texte", "hand"))!).panels).sort()).toEqual(["problems", "textEdit"]);
    expect(localStorage.getItem(deckLayoutKey("welt", "hand"))).toBeNull();
    // The docks of an area carry its name, so two decks in one document do not share an element id.
    expect(dock("schau").id).toBe("dock-texte-schau");
    expect(dock("hand").id).toBe("dock-texte-hand");
  });

  it("takes_the_height_and_the_collapse_of_the_hand_from_its_own_area", async () => {
    const before = readShellState();
    writeShellState({ ...before, handHeight: { ...before.handHeight, welt: 500, texte: 250 }, handCollapsed: { ...before.handCollapsed, welt: true } });
    await mount("texte");
    expect(dock("hand").style.height).toBe("250px");
    expect(dock("hand").hidden).toBe(false);
    act(() => shell.toggleHandCollapsed("texte"));
    expect(dock("hand").hidden).toBe(true);
  });

  it("resets_both_docks_of_an_area_to_their_defaults_and_remembers_them", async () => {
    await mount();
    await split(schau(), "worldmap");
    await split(hand(), "problems");
    act(() => resetLayout("welt", { schau: schauRef, hand: handRef }));
    await settle();
    expect(layoutOf(schau())).toEqual([["world", "matrix", "worldmap"]]);
    expect(layoutOf(hand())).toEqual([["inspector", "script", "problems"]]);
    expect(headerHidden(schau(), "world")).toBe(false);
    expect(hand().activePanel?.id).toBe("inspector");
    expect(groups(JSON.parse(localStorage.getItem(deckLayoutKey("welt", "schau"))!).grid.root)).toEqual([["world", "matrix", "worldmap"]]);
    expect(groups(JSON.parse(localStorage.getItem(deckLayoutKey("welt", "hand"))!).grid.root)).toEqual([["inspector", "script", "problems"]]);
  });

  it("marks_the_script_tab_with_a_dot_while_a_script_is_unsaved_and_keeps_its_title", async () => {
    await mount();
    const tabOf = (title: string) => [...dock("hand").querySelectorAll<HTMLElement>('[role="tab"]')].find((t) => t.textContent === title);
    const dotIn = (tab: HTMLElement | undefined) => tab?.querySelector('[aria-label="ungespeichert"]') ?? null;
    expect(tabOf("Script")).toBeDefined();
    expect(dotIn(tabOf("Script"))).toBeNull();
    act(() => scriptDirtyStore.set(2));
    expect(dotIn(tabOf("Script"))?.getAttribute("role")).toBe("img");
    // Only the script tab carries it; the title is not touched.
    expect(dotIn(tabOf("Inspector"))).toBeNull();
    expect(dotIn(tabOf("Probleme"))).toBeNull();
    expect(hand().getPanel("script")!.title).toBe("Script");
    act(() => scriptDirtyStore.set(0));
    expect(dotIn(tabOf("Script"))).toBeNull();
  });

  it("has_a_slot_for_the_wizard_step_only_where_wizards_show", async () => {
    await mount("texte");
    expect(container.querySelector("[data-wizard-hand]")).toBeNull();
    await mount("welt", false);
    expect(container.querySelector<HTMLElement>("[data-wizard-hand]")!.hidden).toBe(true);
    expect(dock("hand").hidden).toBe(false);
  });

  it("hides_the_hand_dock_behind_an_open_wizard_without_rebuilding_it", async () => {
    await mount("welt", false);
    const before = hand();
    await render("welt", true);
    const slot = container.querySelector<HTMLElement>("[data-wizard-hand]")!;
    expect(slot.hidden).toBe(false);
    expect(dock("hand").hidden).toBe(true);
    expect(slot.style.height).toBe(`${HAND_DEFAULT}px`);
    await render("welt", false);
    expect(slot.hidden).toBe(true);
    expect(dock("hand").hidden).toBe(false);
    expect(hand()).toBe(before);
  });

  it("lifts_a_collapsed_hand_for_the_wizard_without_touching_the_remembered_height", async () => {
    const state = readShellState();
    writeShellState({ ...state, handHeight: { ...state.handHeight, welt: 220 }, handCollapsed: { ...state.handCollapsed, welt: true } });
    await mount("welt", true);
    const slot = container.querySelector<HTMLElement>("[data-wizard-hand]")!;
    expect(slot.hidden).toBe(false);
    expect(slot.style.height).toBe(`${HAND_DEFAULT}px`);
    expect(shell.state.handHeight.welt).toBe(220);
    expect(shell.state.handCollapsed.welt).toBe(true);
    // An open hand keeps its own height for the wizard.
    act(() => shell.toggleHandCollapsed("welt"));
    act(() => shell.setHandHeight("welt", 420));
    expect(slot.style.height).toBe("420px");
  });

  it("drops_the_stored_layouts_of_a_deck_that_is_not_built_so_it_starts_from_its_default", () => {
    // "Layout zurücksetzen" on Start, Technik or Einstellungen works on the world, which may never have been built in this session.
    localStorage.setItem(deckLayoutKey("texte", "schau"), JSON.stringify({ panels: { textView: {} } }));
    localStorage.setItem(deckLayoutKey("texte", "hand"), JSON.stringify({ panels: { textEdit: {} } }));
    localStorage.setItem(deckLayoutKey("welt", "hand"), JSON.stringify({ panels: { tools: {} } }));
    expect(() => resetLayout("texte", { schau: { current: null }, hand: { current: null } })).not.toThrow();
    expect(localStorage.getItem(deckLayoutKey("texte", "schau"))).toBeNull();
    expect(localStorage.getItem(deckLayoutKey("texte", "hand"))).toBeNull();
    // Another area's layout stays.
    expect(localStorage.getItem(deckLayoutKey("welt", "hand"))).not.toBeNull();
  });
});
