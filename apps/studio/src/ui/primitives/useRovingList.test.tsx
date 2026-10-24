// @vitest-environment jsdom
import { act, useRef } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ListRow } from "./ListRow";
import { pressKey } from "./testUtils";
import { useRovingList } from "./useRovingList";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

function List({ labels, selected, orientation }: { labels: string[]; selected?: string; orientation?: "vertical" | "horizontal" | "grid" }) {
  const ref = useRef<HTMLDivElement>(null);
  useRovingList(ref, { orientation });
  return (
    <div ref={ref} role="listbox" aria-label="Orte">
      {labels.map((l) => (
        <ListRow key={l} label={l} selected={l === selected} onSelect={() => {}} />
      ))}
    </div>
  );
}

/** The host mounts only once `open` is true (the way a panel swaps an empty state for its list), so it is absent on the first render. */
function LateList({ open, labels, keyed }: { open: boolean; labels: string[]; keyed?: string }) {
  const ref = useRef<HTMLDivElement>(null);
  useRovingList(ref);
  if (!open) return <p>Noch nichts</p>;
  return (
    <div key={keyed} ref={ref} role="listbox" aria-label="Orte">
      {labels.map((l) => (
        <ListRow key={l} label={l} selected={false} onSelect={() => {}} />
      ))}
    </div>
  );
}

describe("useRovingList", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  const rows = () => [...container.querySelectorAll<HTMLElement>("[role=option]")];
  const tabStops = () => rows().filter((r) => r.tabIndex === 0).map((r) => r.textContent);

  it("exactly_one_row_is_a_tab_stop_the_first_one_by_default", () => {
    act(() => root.render(<List labels={["A", "B", "C"]} />));
    expect(tabStops()).toEqual(["A"]);
  });

  it("the_selected_row_is_the_initial_tab_stop", () => {
    act(() => root.render(<List labels={["A", "B", "C"]} selected="B" />));
    expect(tabStops()).toEqual(["B"]);
  });

  it("arrow_down_moves_the_tab_stop_and_focus_to_the_next_row", () => {
    act(() => root.render(<List labels={["A", "B", "C"]} />));
    pressKey(rows()[0]!, "ArrowDown");
    expect(tabStops()).toEqual(["B"]);
    expect(document.activeElement).toBe(rows()[1]);
    pressKey(rows()[1]!, "ArrowUp");
    expect(tabStops()).toEqual(["A"]);
  });

  it("home_and_end_jump_to_the_first_and_last_row", () => {
    act(() => root.render(<List labels={["A", "B", "C"]} />));
    pressKey(rows()[0]!, "End");
    expect(tabStops()).toEqual(["C"]);
    expect(document.activeElement).toBe(rows()[2]);
    pressKey(rows()[2]!, "Home");
    expect(tabStops()).toEqual(["A"]);
  });

  it("does_not_wrap_around_at_the_ends", () => {
    act(() => root.render(<List labels={["A", "B"]} />));
    pressKey(rows()[0]!, "ArrowUp");
    expect(tabStops()).toEqual(["A"]);
    pressKey(rows()[0]!, "End");
    pressKey(rows()[1]!, "ArrowDown");
    expect(tabStops()).toEqual(["B"]);
  });

  it("horizontal_lists_use_left_and_right", () => {
    act(() => root.render(<List labels={["A", "B"]} orientation="horizontal" />));
    pressKey(rows()[0]!, "ArrowDown");
    expect(tabStops()).toEqual(["A"]);
    pressKey(rows()[0]!, "ArrowRight");
    expect(tabStops()).toEqual(["B"]);
  });

  it("a_grid_moves_by_one_with_left_and_right_and_by_a_row_with_up_and_down", () => {
    act(() => root.render(<List labels={["A", "B", "C", "D", "E"]} orientation="grid" />));
    // Three columns: A B C on the first row, D E on the second (jsdom has no layout, so the rows are given).
    // The page position counts, not offsetTop (which a positioned cell around each item would make 0 for all).
    rows().forEach((r, i) => {
      Object.defineProperty(r, "offsetTop", { configurable: true, value: 0 });
      r.getBoundingClientRect = () => ({ top: i < 3 ? 10 : 110 }) as DOMRect;
    });
    pressKey(rows()[0]!, "ArrowRight");
    expect(tabStops()).toEqual(["B"]);
    pressKey(rows()[1]!, "ArrowDown");
    expect(tabStops()).toEqual(["E"]);
    expect(document.activeElement).toBe(rows()[4]);
    pressKey(rows()[4]!, "ArrowLeft");
    expect(tabStops()).toEqual(["D"]);
    pressKey(rows()[3]!, "ArrowUp");
    expect(tabStops()).toEqual(["A"]);
    // Below C there is no card: Down stays.
    pressKey(rows()[0]!, "End");
    pressKey(rows()[4]!, "ArrowUp");
    expect(tabStops()).toEqual(["B"]);
    pressKey(rows()[1]!, "ArrowRight");
    pressKey(rows()[2]!, "ArrowDown");
    expect(tabStops()).toEqual(["C"]);
  });

  it("focusing_a_row_makes_it_the_tab_stop", () => {
    act(() => root.render(<List labels={["A", "B", "C"]} />));
    act(() => rows()[2]!.focus());
    expect(tabStops()).toEqual(["C"]);
  });

  it("rows_added_later_join_without_creating_a_second_tab_stop", async () => {
    act(() => root.render(<List labels={["A", "B"]} />));
    await act(async () => root.render(<List labels={["A", "B", "C", "D"]} />));
    expect(tabStops()).toEqual(["A"]);
  });

  it("a_removed_tab_stop_hands_over_to_the_selected_or_first_row", async () => {
    act(() => root.render(<List labels={["A", "B", "C"]} />));
    pressKey(rows()[0]!, "ArrowDown");
    expect(tabStops()).toEqual(["B"]);
    await act(async () => root.render(<List labels={["A", "C"]} selected="C" />));
    expect(tabStops()).toEqual(["C"]);
  });

  it("wires_up_a_host_that_mounts_after_the_first_render", async () => {
    await act(async () => root.render(<LateList open={false} labels={["A", "B", "C"]} />));
    expect(rows()).toHaveLength(0);
    await act(async () => root.render(<LateList open labels={["A", "B", "C"]} />));
    expect(tabStops()).toEqual(["A"]);
    pressKey(rows()[0]!, "ArrowDown");
    expect(tabStops()).toEqual(["B"]);
    expect(document.activeElement).toBe(rows()[1]);
  });

  it("wires_up_a_host_that_is_replaced_by_a_new_element", async () => {
    await act(async () => root.render(<LateList open labels={["A", "B"]} keyed="one" />));
    await act(async () => root.render(<LateList open labels={["A", "B"]} keyed="two" />));
    expect(tabStops()).toEqual(["A"]);
    pressKey(rows()[0]!, "ArrowDown");
    expect(tabStops()).toEqual(["B"]);
  });
});
