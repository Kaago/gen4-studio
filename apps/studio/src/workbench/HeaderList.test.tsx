// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { HeaderSummary } from "../api";
import { HeaderList } from "./HeaderList";

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

  it("names a row by its location name, with the id and the internal name as its grey part, and marks it for the shown matrix", () => {
    const { headers } = countedHeaders(2);
    act(() => root.render(<HeaderList {...props(headers, { selected: 1, shownMatrix: 1 })} />));
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
});
