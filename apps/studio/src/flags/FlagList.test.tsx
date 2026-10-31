// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { FlagEntryDto, FlagListDto } from "../api";
import { FLAGS, flagEntry, flagList } from "./flagTesting";
import { FlagList } from "./FlagList";

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

  it("the_live_column_is_empty_with_its_tooltip", async () => {
    await show();
    const head = container.querySelector("[data-flag-list] [data-flag-live-head]")!;
    expect(head.textContent).toBe("Live");
    const cells = [...container.querySelectorAll<HTMLElement>("[data-flag-live]")];
    expect(cells).toHaveLength(5);
    expect(cells.every((c) => c.textContent === "–")).toBe(true);
    // Hovering a cell names why it is empty.
    vi.useFakeTimers();
    try {
      await act(async () => {
        const cell = cells[0]!;
        cell.dispatchEvent(new MouseEvent("mouseover", { bubbles: true }));
        cell.dispatchEvent(new MouseEvent("mouseenter"));
        cell.dispatchEvent(new MouseEvent("mousemove", { bubbles: true }));
        await vi.advanceTimersByTimeAsync(500);
      });
    } finally {
      vi.useRealTimers();
    }
    expect(document.querySelector("[role=tooltip]")?.textContent).toBe("Kommt mit dem Spielmodus (3d.9)");
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
});
