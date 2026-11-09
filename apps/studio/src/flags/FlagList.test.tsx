// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FlagEntryDto, FlagListDto } from "../api";
import { FLAGS, flagEntry, flagList } from "./flagTesting";
import { OFF_STATUS } from "../api.live";
import { liveValuesOf } from "../app/liveStore";
import type { LiveValues } from "../app/liveStore";
import { FlagList, ROW_LIMIT, liveOff } from "./FlagList";
import { outsideTheSave } from "./flagWords";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("FlagList", () => {
  let container: HTMLElement;
  let root: Root;
  const onSelect = vi.fn();
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    onSelect.mockReset();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const show = (list: FlagListDto = FLAGS, entries: readonly FlagEntryDto[] = list.entries, selected: number | null = null) =>
    act(async () => root.render(<FlagList list={list} entries={entries} selected={selected} onSelect={onSelect} />));
  const row = (n: number) => container.querySelector<HTMLElement>(`[data-flag-list] [data-flag-row="${n}"]`)!;
  /** The label and the line below it of a row, as `ListRow` names them. */
  const label = (r: HTMLElement) => document.getElementById(r.getAttribute("aria-labelledby") ?? "")!;
  const below = (r: HTMLElement) => document.getElementById(r.getAttribute("aria-describedby") ?? "")!;

  it("rows_show_own_name_pret_name_number_and_usage", async () => {
    await show(FLAGS, FLAGS.entries, 0x94);
    const list = container.querySelector("[data-flag-list]")!;
    expect(list.querySelector("[role=listbox]")).not.toBeNull();
    expect([...list.querySelectorAll("[data-flag-row]")].map((r) => r.getAttribute("data-flag-row"))).toEqual(["32", "148", "150", "371", "1360"]);
    // An own name in bold over the pret name in mono, with the number in hex and decimal.
    const own = row(148);
    expect(label(own).textContent).toBe("FLAG_HAT_MEDAILLE");
    expect(label(own).className).toMatch(/strong/);
    expect(below(own).textContent).toBe("FLAG_UNUSED_0x0094 · 0x0094 · 148");
    expect(below(own).className).toMatch(/techSubtitle/);
    expect(own.textContent).toContain("gesetzt 3 · geprüft 1");
    expect(own.getAttribute("aria-selected")).toBe("true");
    // A pret name alone: not bold, below it only the number; „unklar“ only when something is unclear.
    const pret = row(150);
    expect(label(pret).textContent).toBe("FLAG_UNUSED_0x0096");
    expect(label(pret).className).not.toMatch(/strong/);
    expect(below(pret).textContent).toBe("0x0096 · 150");
    expect(pret.textContent).toContain("gesetzt 0 · geprüft 0 · unklar 2");
    // Unnamed and free: kind and number, „unbenutzt“ and the plaque „frei“.
    const free = row(371);
    expect(label(free).textContent).toBe("Flag 0x0173");
    expect(free.textContent).toContain("unbenutzt");
    expect(free.querySelector("[data-flag-badge=free]")?.textContent).toBe("frei");
    // Reserved by a recipe: the plaque „reserviert“, the recipe as its title.
    await show(flagList("var", [flagEntry(0x406c, { name: "VAR_UNUSED_0x406C", reserved: "Vorlage „Leere Region“" })]));
    const reserved = row(0x406c).querySelector("[data-flag-badge=reserved]")!;
    expect(reserved.textContent).toBe("reserviert");
    expect(reserved.getAttribute("title")).toBe("Vorlage „Leere Region“");
    // A click selects the number.
    await show();
    await act(async () => row(0x173).click());
    expect(onSelect).toHaveBeenCalledWith(0x173);
  });

  /** The tooltip a hover over `el` opens (Base UI waits 400 ms). */
  const tipOf = async (el: HTMLElement) => {
    vi.useFakeTimers();
    try {
      await act(async () => {
        el.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
        el.dispatchEvent(new MouseEvent("mouseenter"));
        el.dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
        await vi.advanceTimersByTimeAsync(500);
      });
    } finally {
      vi.useRealTimers();
    }
    const tip = document.querySelector("[role=tooltip]")?.textContent;
    await act(async () => {
      el.dispatchEvent(new MouseEvent("mouseout", { bubbles: true }));
      el.dispatchEvent(new MouseEvent("mouseleave"));
    });
    return tip;
  };
  const cell = (n: number) => row(n).querySelector<HTMLElement>("[data-flag-live]")!;
  /** The live values of a snapshot whose flag bytes and variables (from 0x4000) are given. */
  const liveOf = (flags: number[], vars: number[] = []): LiveValues =>
    liveValuesOf({ ...OFF_STATUS, state: "live", snapshot: { revision: 3, flags, vars, taken_ms: 1 } })!;

  it("the_live_column_is_empty_with_the_off_tooltip_until_play_mode", async () => {
    await show();
    const head = container.querySelector("[data-flag-list] [data-flag-live-head]")!;
    expect(head.textContent).toBe("Live");
    expect(head.getAttribute("data-flag-live-head")).toBe("");
    const cells = [...container.querySelectorAll<HTMLElement>("[data-flag-live]")];
    expect(cells).toHaveLength(5);
    expect(cells.every((c) => c.textContent === "–" && c.tagName === "SPAN")).toBe(true);
    // Hovering a cell names how its value comes.
    expect(await tipOf(cells[0]!)).toBe("Kommt mit „Hier spielen“ in melonDS");
    expect(liveOff()).toBe("Kommt mit „Hier spielen“ in melonDS");
  });

  it("the_live_head_is_marked_on_in_play_mode", async () => {
    await act(async () => root.render(<FlagList list={FLAGS} entries={FLAGS.entries} selected={null} onSelect={onSelect} live={liveOf([])} onLiveToggle={vi.fn()} />));
    expect(container.querySelector("[data-flag-live-head]")?.getAttribute("data-flag-live-head")).toBe("on");
  });

  it("before_the_first_snapshot_the_head_is_on_and_the_cells_say_why_they_wait", async () => {
    await act(async () =>
      root.render(<FlagList list={FLAGS} entries={FLAGS.entries} selected={null} onSelect={onSelect} live={null} livePending="Wartet auf den Spielstand" onLiveToggle={vi.fn()} />),
    );
    expect(container.querySelector("[data-flag-live-head]")?.getAttribute("data-flag-live-head")).toBe("on");
    const cells = [...container.querySelectorAll<HTMLElement>("[data-flag-live]")];
    expect(cells.every((c) => c.textContent === "–" && c.tagName === "SPAN")).toBe(true);
    expect(await tipOf(cells[0]!)).toBe("Wartet auf den Spielstand");
  });

  it("shows_live_flag_values_as_toggles_and_var_values_as_numbers", async () => {
    const onLiveToggle = vi.fn();
    const flags = new Array<number>(200).fill(0);
    flags[0x94 >> 3] = 1 << (0x94 % 8);
    const list = flagList("flag", [...FLAGS.entries.slice(1, 4), flagEntry(0x9999, { in_save: false })]);
    await act(async () => root.render(<FlagList list={list} entries={list.entries} selected={null} onSelect={onSelect} live={liveOf(flags)} onLiveToggle={onLiveToggle} />));
    // A flag: „1“ or „0“ as a button; a click toggles it and does not choose the row.
    const set = cell(0x94);
    expect(set.tagName).toBe("BUTTON");
    expect(set.getAttribute("data-flag-live")).toBe("1");
    expect(set.textContent).toBe("1");
    expect(await tipOf(set)).toBe("Live: gesetzt · Klick löscht");
    const clear = cell(0x96);
    expect(clear.getAttribute("data-flag-live")).toBe("0");
    expect(clear.textContent).toBe("0");
    expect(await tipOf(clear)).toBe("Live: 0 · Klick setzt");
    await act(async () => set.click());
    expect(onLiveToggle).toHaveBeenCalledWith(0x94);
    expect(onSelect).not.toHaveBeenCalled();
    // Outside the save: „–“ with the reason, no button.
    const outside = cell(0x9999);
    expect(outside.tagName).toBe("SPAN");
    expect(outside.textContent).toBe("–");
    expect(await tipOf(outside)).toBe(outsideTheSave());
    // A variable: its value in decimal, changed in the detail.
    const vars = flagList("var", [flagEntry(0x4000), flagEntry(0x4001), flagEntry(0x8000, { in_save: false })]);
    await act(async () => root.render(<FlagList list={vars} entries={vars.entries} selected={null} onSelect={onSelect} live={liveOf([], [1234, 7])} onLiveToggle={onLiveToggle} />));
    expect(cell(0x4000).tagName).toBe("SPAN");
    expect(cell(0x4000).getAttribute("data-flag-live")).toBe("1234");
    expect(cell(0x4000).textContent).toBe("1234");
    expect(cell(0x4001).textContent).toBe("7");
    expect(await tipOf(cell(0x4000))).toBe("Live 1234 · ändern im Detail");
    expect(cell(0x8000).textContent).toBe("–");
    expect(await tipOf(cell(0x8000))).toBe(outsideTheSave());
  });

  it("map_local_rows_count_places", async () => {
    await show();
    const local = row(0x20);
    // A map-local number is counted by the places that use it, not by its uses; it is never „frei“.
    expect(local.textContent).toContain("an 54 Orten");
    expect(local.textContent).not.toContain("gesetzt");
    expect(local.querySelector("[data-flag-badge]")).toBeNull();
    await show(flagList("flag", [flagEntry(0x21, { area: "map_local", usage: { read: 1, write: 0, unknown: 0, sources: 1 }, places: 1 })]));
    expect(row(0x21).textContent).toContain("an 1 Ort");
    // The foot: how many of how many, and what scripts use without a number.
    await show(flagList("flag", FLAGS.entries, { counts: { script_local: 1234, dynamic: 17 } }), FLAGS.entries.slice(0, 2));
    expect(container.querySelector("[data-flag-list-foot]")?.textContent).toBe(
      "2 von 5 · Script-lokale Variablen 1.234 Vorkommen, Flags aus Variablen 17 (nicht im Index)",
    );
    // Nothing left after filter and search.
    await show(FLAGS, []);
    expect(container.querySelector("[data-flag-list]")?.textContent).toContain("Nichts passt zu Filter und Suche.");
  });

  it("a_capped_list_counts_the_rows_it_shows", async () => {
    const many = flagList(
      "flag",
      Array.from({ length: ROW_LIMIT + 20 }, (_, i) => flagEntry(i + 1)),
      { counts: { script_local: 0, dynamic: 3 } },
    );
    await show(many);
    const foot = () => container.querySelector("[data-flag-list-foot]")?.textContent;
    expect(container.querySelectorAll("[data-flag-row]")).toHaveLength(ROW_LIMIT);
    expect(foot()).toBe("500 von 520 gezeigt · Script-lokale Variablen 0 Vorkommen, Flags aus Variablen 3 (nicht im Index)");
    // The cap note stays.
    expect(container.textContent).toContain("Die ersten 500 Zeilen; Suche und Filter erreichen die übrigen.");
    // The chosen row past the cap shows as well, and counts.
    await show(many, many.entries, ROW_LIMIT + 10);
    expect(container.querySelectorAll("[data-flag-row]")).toHaveLength(ROW_LIMIT + 1);
    expect(foot()).toMatch(/^501 von 520 gezeigt · /);
    // After a filter: of how many it matched, and of the whole.
    await show(many, many.entries.slice(0, ROW_LIMIT + 5));
    expect(foot()).toMatch(/^500 von 505 gezeigt, 520 insgesamt · /);
    // Not capped: the plan's foot, unchanged.
    await show(many, many.entries.slice(0, 7));
    expect(foot()).toMatch(/^7 von 520 · /);
    expect(container.textContent).not.toContain("Die ersten 500 Zeilen");
  });
});
