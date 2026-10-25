// @vitest-environment jsdom
import { act } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { dataGet, dataList, getMonIcon, itemTable, nameTables, refs } from "../api";
import { readShellState, writeShellState } from "../ui/frame/shellState";
import { pressKey, typeInto } from "../ui/primitives/testUtils";
import { MOVES, NAMES, SPECIES, dataHarness, flush, move, openMenu, projectDp, species } from "./dataTesting";
import { resetIcons } from "./icons";
import { staraviaDoc } from "./pokedex/pokedexTesting";
import { PokemonArea, PokemonHand, PokemonList, PokemonStatus, PokemonView } from "./PokemonArea";
import { useDataArea } from "./useDataArea";

vi.mock("../api", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../api")>()),
  dataList: vi.fn(),
  nameTables: vi.fn(),
  itemTable: vi.fn(),
  getMonIcon: vi.fn(),
  getItemIcon: vi.fn(),
  dataGet: vi.fn(),
  refs: vi.fn(),
}));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("PokemonArea", () => {
  let h: ReturnType<typeof dataHarness>;

  beforeEach(() => {
    resetIcons();
    vi.mocked(dataList).mockReset().mockImplementation(async (kind) => (kind === "species" ? SPECIES : kind === "move" ? MOVES : []));
    vi.mocked(nameTables).mockReset().mockResolvedValue(NAMES);
    vi.mocked(itemTable).mockReset().mockResolvedValue(null);
    vi.mocked(getMonIcon).mockReset().mockResolvedValue(new Uint8Array([137, 80, 78, 71]).buffer);
    vi.mocked(dataGet).mockReset().mockImplementation(async () => staraviaDoc());
    vi.mocked(refs).mockReset().mockResolvedValue([]);
    URL.createObjectURL = vi.fn(() => "blob:icon");
    URL.revokeObjectURL = vi.fn();
    Element.prototype.scrollIntoView = () => {};
    h = dataHarness((props) => (
      <PokemonArea context={props}>
        <PokemonList open onToggle={() => {}} />
        <section data-view="">
          <PokemonView />
        </section>
        <section data-hand="">
          <PokemonHand />
        </section>
        <p data-status="">
          <PokemonStatus />
        </p>
      </PokemonArea>
    ));
  });
  afterEach(() => h.unmount());

  const switcher = () => h.q<HTMLElement>('section[aria-label="Pokémon"] header button[aria-haspopup="menu"]')!;
  /** Switches through the breadcrumb's second crumb, the same switcher as the list head's menu (that menu has a test of its own). */
  const switchTo = async (label: string) => {
    const entry = (h.lastTrail()[1]?.menu ?? []).find((m) => m.label === label)!;
    await act(async () => entry.onSelect());
    await flush();
  };
  const rowOf = (attr: string, index: number) => h.q(`[${attr}="${index}"]`);

  it("lists_the_species_with_name_four_digit_number_and_icon_and_reads_nothing_before_it_shows", async () => {
    h.show(h.base({ active: false }));
    await flush();
    expect(dataList).not.toHaveBeenCalled();
    h.show(h.base());
    await flush();
    expect(dataList).toHaveBeenCalledWith("species");
    expect(h.container.querySelectorAll("[data-species]")).toHaveLength(4);
    const staravia = rowOf("data-species", 397)!;
    expect(staravia.textContent).toContain("Staravia");
    expect(staravia.textContent).toContain("0397");
    expect(staravia.querySelector("[data-sprite=picture]")?.getAttribute("src")).toBe("blob:icon");
    expect(getMonIcon).toHaveBeenCalledWith(397);
    // A species without a name in the text archive is named in German.
    expect(rowOf("data-species", 520)?.textContent).toContain("Art 0520");
  });

  it("switches_the_list_and_the_breadcrumb_with_the_switcher_and_remembers_the_part", async () => {
    h.show(h.base());
    await flush();
    expect(h.trail()).toEqual(["Pokémon", "Pokédex"]);
    expect(switcher().textContent).toContain("Pokédex");
    const entries = openMenu(switcher());
    const starts = ["Pokédex", "Trainer", "Begegnungen", "Attacken"];
    expect(entries.map((e, i) => e.textContent?.startsWith(starts[i]!))).toEqual([true, true, true, true]);
    // The Pokédex counts its species; the moves are not read yet.
    expect(entries[0]?.textContent).toContain("4");
    expect(entries[1]?.getAttribute("aria-disabled")).toBeNull();
    // Every part is open (Begegnungen since 3b.3b).
    expect(entries.map((e) => e.getAttribute("aria-disabled"))).toEqual([null, null, null, null]);
    await act(async () => entries[3]!.click());
    await flush();
    expect(dataList).toHaveBeenCalledWith("move");
    expect(h.container.querySelectorAll("[data-species]")).toHaveLength(0);
    expect(rowOf("data-move", 17)?.textContent).toContain("Flügelschlag");
    expect(h.trail()).toEqual(["Pokémon", "Attacken"]);
    expect(readShellState().sub.pokemon).toBe("moves");
    // The breadcrumb's second crumb is the same switcher.
    const menu = h.lastTrail()[1]?.menu ?? [];
    expect(menu.map((m) => m.checked)).toEqual([false, false, false, true]);
    await act(async () => menu[0]!.onSelect());
    await flush();
    expect(rowOf("data-species", 397)).not.toBeNull();
  });

  it("keeps_the_selection_of_each_part", async () => {
    h.show(h.base());
    await flush();
    await act(async () => rowOf("data-species", 397)!.click());
    expect(h.trail()).toEqual(["Pokémon", "Pokédex", "Staravia"]);
    expect(h.q("[data-status]")?.textContent).toBe("Staravia, 0397");
    await switchTo("Attacken");
    expect(h.q("[data-status]")?.textContent).toBe("");
    await act(async () => rowOf("data-move", 17)!.click());
    expect(h.trail()).toEqual(["Pokémon", "Attacken", "Flügelschlag"]);
    await switchTo("Pokédex");
    expect(rowOf("data-species", 397)?.getAttribute("aria-selected")).toBe("true");
    expect(h.trail()).toEqual(["Pokémon", "Pokédex", "Staravia"]);
    expect(readShellState().selected).toEqual({ pokedex: 397, moves: 17 });
    // The last crumb leads to the selection.
    h.lastTrail()[2]?.onClick?.();
    expect(h.calls.goTo).toHaveBeenCalledWith({ kind: "species", index: 397 });
  });

  it("says_what_to_do_when_nothing_is_chosen_and_names_the_choice_in_view_and_hand", async () => {
    h.show(h.base());
    await flush();
    expect(h.q("[data-view]")?.textContent).toContain("Kein Pokémon gewählt.");
    expect(h.q("[data-view]")?.textContent).toContain("Wähle links eine Art; hier erscheinen Werte und Fundorte.");
    await act(async () => rowOf("data-species", 397)!.click());
    await flush();
    expect(h.q("[data-view]")?.textContent).toContain("Staravia");
    // The editor of the Pokédex: its tabs, named after the species.
    expect(h.q("[data-hand] [role=tablist]")?.getAttribute("aria-label")).toBe("Staravia bearbeiten");
  });

  it("has_no_head_title_that_repeats_the_tab", async () => {
    h.show(h.base());
    await flush();
    await act(async () => rowOf("data-species", 397)!.click());
    await flush();
    // The dock tabs say "Übersicht" and "Bearbeiten"; the heads in view and hand name the chosen species.
    const heads = [...h.container.querySelectorAll("[data-view] h2, [data-view] header, [data-hand] h2, [data-hand] header")].map((t) => t.textContent ?? "");
    expect(heads.some((t) => t.includes("Staravia"))).toBe(true);
    expect(heads.filter((t) => /^(Übersicht|Bearbeiten)$/.test(t.trim()))).toEqual([]);
  });

  it("offers_to_show_the_first_species_while_nothing_is_chosen", async () => {
    h.show(h.base());
    await flush();
    // The first row with a number above 0 (member 0 is the game's empty species "-----").
    await act(async () => h.button("Erste Art zeigen")!.click());
    await flush();
    expect(h.q("[data-view]")?.textContent).toContain("Bisasam");
    expect(h.button("Erste Art zeigen")).toBeUndefined();
  });

  it("renders_a_search_in_the_list_but_not_in_the_view_and_the_hand", async () => {
    // The view and the hand read the area's model; a search changes the list only, so they must not render again for it (Spec §9: a
    // list renders in under 50 ms, and the hand of a species alone took about 30 ms per keystroke).
    const renders = { model: 0 };
    function Probe() {
      useDataArea("pokemon");
      renders.model++;
      return null;
    }
    h.unmount();
    h = dataHarness((props) => (
      <PokemonArea context={props}>
        <PokemonList open onToggle={() => {}} />
        <Probe />
      </PokemonArea>
    ));
    h.show(h.base());
    await flush();
    const before = renders.model;
    const search = h.q<HTMLInputElement>('input[aria-label="Pokédex durchsuchen"]')!;
    typeInto(search, "stara");
    expect([...h.container.querySelectorAll("[data-species]")].map((r) => r.getAttribute("data-species"))).toEqual(["396", "397"]);
    typeInto(search, "");
    expect(h.container.querySelectorAll("[data-species]")).toHaveLength(4);
    expect(renders.model).toBe(before);
  });

  it("says_no_project_without_one", async () => {
    h.show(h.base({ project: null }));
    await flush();
    expect(h.container.textContent).toContain("Kein Projekt geöffnet.");
    expect(dataList).not.toHaveBeenCalled();
  });

  it("shows_the_lock_of_diamond_and_pearl_as_the_reason", async () => {
    vi.mocked(dataList).mockRejectedValue({ code: "read_only", message: "Diamant/Perl: ohne Testprojekt nicht freigegeben", asset: null });
    vi.mocked(nameTables).mockRejectedValue({ code: "read_only", message: "Diamant/Perl: ohne Testprojekt nicht freigegeben", asset: null });
    h.show(h.base({ project: projectDp }));
    await flush();
    expect(h.q("[data-view]")?.textContent).toContain("Diamant/Perl: ohne Testprojekt nicht freigegeben");
    expect(h.container.querySelectorAll("[data-species]")).toHaveLength(0);
  });

  it("searches_by_name_or_number_and_offers_to_clear_a_search_without_hits", async () => {
    h.show(h.base());
    await flush();
    const search = h.q<HTMLInputElement>('input[aria-label="Pokédex durchsuchen"]')!;
    typeInto(search, "stara");
    expect([...h.container.querySelectorAll("[data-species]")].map((r) => r.getAttribute("data-species"))).toEqual(["396", "397"]);
    typeInto(search, "397");
    expect([...h.container.querySelectorAll("[data-species]")].map((r) => r.getAttribute("data-species"))).toEqual(["397"]);
    typeInto(search, "xyz");
    expect(h.container.querySelectorAll("[data-species]")).toHaveLength(0);
    expect(h.container.textContent).toContain("Nichts gefunden für „xyz“");
    await act(async () => h.button("Filter lösen")!.click());
    expect(h.container.querySelectorAll("[data-species]")).toHaveLength(4);
  });

  it("opens_the_species_a_link_asks_for_in_the_pokedex", async () => {
    h.show(h.base());
    await flush();
    await switchTo("Attacken");
    h.show(h.base({ target: { sub: "pokedex", index: 396 } }));
    await flush();
    expect(readShellState().sub.pokemon).toBe("pokedex");
    expect(rowOf("data-species", 396)?.getAttribute("aria-selected")).toBe("true");
    expect(h.trail()).toEqual(["Pokémon", "Pokédex", "Staralili"]);
  });

  it("renders_a_window_of_a_long_list_and_reaches_all_of_it_with_the_keys", async () => {
    vi.mocked(dataList).mockImplementation(async (kind) => (kind === "species" ? Array.from({ length: 508 }, (_, i) => species(i, `Art Nummer ${i}`)) : []));
    h.show(h.base());
    await flush();
    const rendered = () => [...h.container.querySelectorAll("[data-species]")];
    expect(rendered().length).toBeGreaterThan(10);
    expect(rendered().length).toBeLessThan(100);
    // One tab stop in the list.
    expect(rendered().filter((r) => r.getAttribute("tabindex") === "0").map((r) => r.getAttribute("data-species"))).toEqual(["0"]);
    const focused = () => document.activeElement?.getAttribute("data-species");
    rowOf("data-species", 0)!.focus();
    pressKey(document.activeElement as HTMLElement, "End");
    expect(focused()).toBe("507");
    pressKey(document.activeElement as HTMLElement, "ArrowUp");
    expect(focused()).toBe("506");
    pressKey(document.activeElement as HTMLElement, "Home");
    expect(focused()).toBe("0");
    pressKey(document.activeElement as HTMLElement, "PageDown");
    expect(Number(focused())).toBeGreaterThan(10);
    pressKey(document.activeElement as HTMLElement, "Enter");
    expect(readShellState().selected.pokedex).toBe(Number(focused()));
    // A link to a member outside the window brings it in.
    h.show(h.base({ target: { sub: "pokedex", index: 400 } }));
    await flush();
    expect(rowOf("data-species", 400)?.getAttribute("aria-selected")).toBe("true");
  });

  describe("with a laid-out list", () => {
    // jsdom has no layout: the list is 240 px high, a row 24 px (the compact density), as a browser would measure them.
    const LIST = 240;
    const ROW = 24;
    const restore: (() => void)[] = [];
    const stub = (name: "clientHeight" | "offsetHeight", get: (el: HTMLElement) => number) => {
      // `clientHeight` is jsdom's on Element, `offsetHeight` on HTMLElement: put back what was there, or nothing.
      const before = Object.getOwnPropertyDescriptor(HTMLElement.prototype, name);
      Object.defineProperty(HTMLElement.prototype, name, { configurable: true, get(this: HTMLElement) { return get(this); } });
      restore.push(() => (before ? Object.defineProperty(HTMLElement.prototype, name, before) : delete (HTMLElement.prototype as unknown as Record<string, unknown>)[name]));
    };
    const listbox = () => h.q<HTMLElement>('[role=listbox][aria-label="Pokédex"]')!;
    const many = () => Array.from({ length: 508 }, (_, i) => species(i, `Art Nummer ${i}`));

    beforeEach(() => {
      vi.mocked(dataList).mockImplementation(async (kind) => (kind === "species" ? many() : kind === "move" ? MOVES : []));
      stub("clientHeight", (el) => (el.getAttribute("role") === "listbox" ? LIST : 0));
      stub("offsetHeight", (el) => (el.hasAttribute("data-pos") ? ROW : 0));
    });
    afterEach(() => {
      while (restore.length > 0) restore.pop()!();
    });

    it("scrolls_a_selection_it_mounts_with_into_view_with_the_measured_sizes", async () => {
      writeShellState({ ...readShellState(), selected: { pokedex: 400 } });
      h.show(h.base());
      await flush();
      // Row 400 ends at 401 × 24 px; the list shows 240 px of it.
      expect(listbox().scrollTop).toBe(401 * ROW - LIST);
      expect(rowOf("data-species", 400)).not.toBeNull();
      // Coming back to the Pokédex from another part mounts the list again: the selection is in view again.
      await switchTo("Attacken");

      await switchTo("Pokédex");
      expect(listbox().scrollTop).toBe(401 * ROW - LIST);
    });

    it("keeps_the_focused_row_when_the_list_scrolls_away_from_it", async () => {
      h.show(h.base());
      await flush();
      rowOf("data-species", 0)!.focus();
      const list = listbox();
      list.scrollTop = 300 * ROW;
      act(() => void list.dispatchEvent(new Event("scroll")));
      expect(rowOf("data-species", 300)).not.toBeNull();
      expect(document.activeElement?.getAttribute("data-species")).toBe("0");
      expect(rowOf("data-species", 0)?.getAttribute("tabindex")).toBe("0");
    });
  });

  it("tells_assistive_technology_where_a_rendered_row_stands_in_the_whole_list", async () => {
    h.show(h.base());
    await flush();
    const staravia = rowOf("data-species", 397)!;
    expect(staravia.getAttribute("aria-setsize")).toBe("4");
    expect(staravia.getAttribute("aria-posinset")).toBe("3");
  });

  it("reads_the_moves_again_when_one_changed", async () => {
    h.show(h.base());
    await flush();
    await switchTo("Attacken");
    vi.mocked(dataList).mockResolvedValue([move(17, "Flügelschlag neu", 2), MOVES[1]!]);
    act(() => h.store.onEvent({ topic: "dataChanged", kind: "move", index: 17 }));
    await flush();
    expect(rowOf("data-move", 17)?.textContent).toContain("Flügelschlag neu");
  });
});
