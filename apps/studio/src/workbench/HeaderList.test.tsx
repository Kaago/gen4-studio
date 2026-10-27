// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary } from "../api";
import type { Depth, ListSort } from "../ui/frame/shellState";
import { HeaderList } from "./HeaderList";
import { chooseContext, contextTexts, resetSpy, withReset } from "../original/resetTesting";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

/** Headers whose `location_name` counts its reads: a row reads it a fixed number of times per render, so reads count row renders. */
function countedHeaders(n: number): { headers: HeaderSummary[]; reads: () => number } {
  let reads = 0;
  const headers = Array.from({ length: n }, (_, id) => {
    const h = { id, internal_name: `H${id}`, matrix_id: id % 7, area_data_id: 0, event_file_id: id, text_archive_id: 0, script_file_id: 0, level_script_id: 0 } as HeaderSummary;
    Object.defineProperty(h, "location_name", {
      enumerable: true,
      get() {
        reads++;
        return `Ort ${id}`;
      },
    });
    return h;
  });
  return { headers, reads: () => reads };
}

describe("HeaderList", () => {
  let container: HTMLElement;
  let root: Root;
  const scrolled: string[] = [];
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    scrolled.length = 0;
    // jsdom has no layout, hence no scrollIntoView.
    Element.prototype.scrollIntoView = function (this: Element) {
      scrolled.push(this.getAttribute("data-header-id") ?? "?");
    };
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const props = (headers: HeaderSummary[], over: Partial<Parameters<typeof HeaderList>[0]> = {}) => ({
    info: null,
    headers,
    selected: null,
    shownMatrix: null,
    revealToken: 0,
    depth: "bauen" as Depth,
    sort: "id" as ListSort,
    onSelect: () => {},
    ...over,
  });

  it("does not render its rows again for the same props, and only the changed rows for a new selection", () => {
    const { headers, reads } = countedHeaders(600);
    const base = props(headers);
    act(() => root.render(<HeaderList {...base} />));
    expect(container.querySelectorAll("[role=option]")).toHaveLength(600);
    // Reads of one row render (all rows render alike on the first pass).
    const perRow = reads() / 600;
    expect(perRow).toBeGreaterThan(0);

    const afterFirst = reads();
    act(() => root.render(<HeaderList {...base} />));
    expect(reads()).toBe(afterFirst);

    act(() => root.render(<HeaderList {...base} selected={3} />));
    // Only the newly selected row renders, not 600.
    expect(reads() - afterFirst).toBe(perRow);
    act(() => root.render(<HeaderList {...base} selected={5} />));
    // The previously and the newly selected row.
    expect(reads() - afterFirst).toBe(3 * perRow);

    // Marking the headers of another matrix renders exactly the rows whose mark changed.
    const before = reads();
    act(() => root.render(<HeaderList {...base} selected={5} shownMatrix={2} />));
    expect(reads() - before).toBe(headers.filter((h) => h.matrix_id === 2).length * perRow);
  });

  it("shows_the_number_and_the_code_as_the_grey_part_in_bauen_so_rows_of_one_place_differ", () => {
    // Eight rooms of one place all read "Zweiblattdorf": the number and the code tell them apart (Bauen speaks of a number, not a header).
    const { headers } = countedHeaders(2);
    act(() => root.render(<HeaderList {...props(headers)} />));
    const row = container.querySelector<HTMLElement>('[data-header-id="1"]')!;
    expect(row.textContent).toBe("Ort 10001 · H1");
    expect(row.getAttribute("title")).toBe("Ort 1 · Nummer 0001 · H1");
  });

  it("names_its_search_and_list_without_header_in_bauen", () => {
    const { headers } = countedHeaders(2);
    act(() => root.render(<HeaderList {...props(headers)} />));
    expect(container.querySelector('input[aria-label="Orte suchen"]')).not.toBeNull();
    expect(container.querySelector('[role="listbox"]')?.getAttribute("aria-label")).toBe("Orte");
    act(() => root.render(<HeaderList {...props(headers, { depth: "tuefteln" })} />));
    expect(container.querySelector('input[aria-label="Header suchen"]')).not.toBeNull();
    expect(container.querySelector('[role="listbox"]')?.getAttribute("aria-label")).toBe("Header");
  });

  it("shows_no_matrix_tag_in_building", () => {
    const { headers } = countedHeaders(3);
    act(() => root.render(<HeaderList {...props(headers, { info: { root: "/p", game_code: "CPUE", family: "Pt", language: "de", header_count: 3 }, shownMatrix: 1 })} />));
    expect(container.textContent).not.toMatch(/Header|Matrix|Raster|M1\b/);
    expect([...container.querySelectorAll("[title]")].map((e) => e.getAttribute("title")).join(" ")).not.toMatch(/Header|Matrix|Raster/);
    expect(container.textContent).toContain("3 Orte");
    act(() => root.render(<HeaderList {...props(headers, { depth: "tuefteln" })} />));
    expect(container.querySelector('[data-header-id="1"]')?.textContent).toBe("Ort 10001 · H1M1");
  });

  it("sorts_by_number_name_and_code", () => {
    const listed = [
      { id: 0, internal_name: "T10", location_name: "Route 10", matrix_id: 0 },
      { id: 1, internal_name: "R02", location_name: "Äther", matrix_id: 0 },
      { id: 2, internal_name: "A01", location_name: "Route 2", matrix_id: 0 },
      { id: 3, internal_name: "B01", location_name: "", matrix_id: 0 },
    ] as HeaderSummary[];
    const order = () => [...container.querySelectorAll("[role=option]")].map((o) => o.getAttribute("data-header-id"));
    act(() => root.render(<HeaderList {...props(listed)} />));
    expect(order()).toEqual(["0", "1", "2", "3"]);
    // By the name the row shows: a header without a place by its code.
    act(() => root.render(<HeaderList {...props(listed, { sort: "name" })} />));
    expect(order()).toEqual(["1", "3", "2", "0"]);
    act(() => root.render(<HeaderList {...props(listed, { sort: "code" })} />));
    expect(order()).toEqual(["2", "3", "1", "0"]);
  });

  it("names a row by its location name, with the id and the internal name as its grey part, and marks it for the shown matrix", () => {
    const { headers } = countedHeaders(2);
    act(() => root.render(<HeaderList {...props(headers, { selected: 1, shownMatrix: 1, depth: "tuefteln" })} />));
    const row = container.querySelector<HTMLElement>('[data-header-id="1"]')!;
    expect(row.getAttribute("role")).toBe("option");
    expect(row.getAttribute("aria-selected")).toBe("true");
    expect(row.textContent).toBe("Ort 10001 · H1M1");
    expect(row.querySelector('[aria-label="Raster wird angezeigt"]')).not.toBeNull();
    expect(container.querySelector('[data-header-id="0"]')?.querySelector('[aria-label="Raster wird angezeigt"]')).toBeNull();
  });

  it("scrolls the selected row into view by its id, again for a new reveal token, and calls onSelect with the header", () => {
    const { headers } = countedHeaders(50);
    const onSelect = vi.fn();
    const base = props(headers, { onSelect });
    act(() => root.render(<HeaderList {...base} selected={12} />));
    expect(scrolled).toEqual(["12"]);
    act(() => root.render(<HeaderList {...base} selected={12} />));
    expect(scrolled).toEqual(["12"]);
    act(() => root.render(<HeaderList {...base} selected={12} revealToken={1} />));
    expect(scrolled).toEqual(["12", "12"]);

    const button = container.querySelector<HTMLElement>('[data-header-id="30"]');
    act(() => button?.click());
    expect(onSelect).toHaveBeenCalledWith(headers[30]);
  });

  it("clears a search that hides the selected header before scrolling to it", () => {
    const { headers } = countedHeaders(20);
    const base = props(headers);
    act(() => root.render(<HeaderList {...base} />));
    const input = container.querySelector<HTMLInputElement>("input")!;
    const setValue = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
    act(() => {
      setValue.call(input, "Ort 1");
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
    expect(container.querySelectorAll("[role=option]").length).toBeLessThan(20);
    act(() => root.render(<HeaderList {...base} selected={4} />));
    expect(container.querySelectorAll("[role=option]")).toHaveLength(20);
    expect(scrolled).toEqual(["4"]);
  });

  it("header_rows_have_reset", () => {
    const reset = resetSpy();
    const { headers } = countedHeaders(3);
    act(() => root.render(withReset(reset, <HeaderList {...props(headers)} />)));
    const row = container.querySelector<HTMLElement>('[data-header-id="2"]')!;
    expect(contextTexts(row)).toEqual(["Header auf Original zurücksetzen…"]);
    chooseContext(row, "Header auf Original zurücksetzen…");
    expect(reset.open).toHaveBeenCalledWith({ kind: "header", id: 2 });
  });
});
