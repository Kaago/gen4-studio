// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Table } from "./Table";
import type { TableProps, TableRow } from "./Table";
import { pressKey } from "./testUtils";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const columns: TableProps["columns"] = [
  { key: "level", label: "Level", width: 64, align: "end" },
  { key: "move", label: "Attacke" },
];

const rowsOf = (moves: [number, string][]): TableRow[] =>
  moves.map(([level, move], i) => ({
    key: `r${i}`,
    attrs: { "data-move": String(i) },
    cells: {
      level: <input aria-label={`Level von ${move}`} defaultValue={level} />,
      move: <button type="button">{move}</button>,
    },
  }));

const learnset = rowsOf([
  [1, "Tackle"],
  [5, "Ruckzuckhieb"],
  [9, "Flügelschlag"],
]);

describe("Table", () => {
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

  const grid = () => container.querySelector<HTMLElement>("[role=grid]")!;
  const bodyRows = () => [...container.querySelectorAll<HTMLElement>("[role=row][data-roving]")];
  const outside = (text: string) => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === text)!;
  /** The elements Tab stops at, in document order. */
  const tabOrder = () =>
    [...container.querySelectorAll<HTMLElement>("input, button, [tabindex]")].filter((el) => el.tabIndex >= 0 && !el.matches(":disabled"));
  /** Tab as a browser does it: the key goes to the focused element, and unless something used it, the focus moves along the Tab order. */
  const tab = (from: HTMLElement, backwards = false) => {
    if (pressKey(from, "Tab", { shiftKey: backwards }).defaultPrevented) return;
    const order = tabOrder();
    const next = order[order.indexOf(from) + (backwards ? -1 : 1)];
    act(() => next?.focus());
  };

  function Host(props: Partial<TableProps>) {
    const [selected, setSelected] = useState<string | undefined>(props.selected);
    return (
      <>
        <button type="button">Davor</button>
        <Table
          columns={columns}
          rows={learnset}
          ariaLabel="Lernset"
          selected={selected}
          onSelect={(key) => {
            setSelected(key);
            props.onSelect?.(key);
          }}
          {...(props.onDeleteRow ? { onDeleteRow: props.onDeleteRow } : {})}
          footer={props.footer}
        />
        <button type="button">Danach</button>
      </>
    );
  }

  it("is_a_named_grid_with_column_headers_rows_and_cells", () => {
    act(() => root.render(<Host footer="3 Attacken" />));
    expect(grid().getAttribute("aria-label")).toBe("Lernset");
    expect([...container.querySelectorAll("[role=columnheader]")].map((c) => c.textContent)).toEqual(["Level", "Attacke"]);
    expect(bodyRows()).toHaveLength(3);
    expect([...bodyRows()[2]!.querySelectorAll("[role=gridcell]")].map((c) => c.textContent)).toEqual(["", "Flügelschlag"]);
    expect(bodyRows()[1]!.getAttribute("data-move")).toBe("1");
    expect(container.textContent).toContain("3 Attacken");
  });

  it("has_one_tab_stop_on_the_rows_and_moves_it_with_the_arrow_keys", () => {
    const onSelect = vi.fn();
    act(() => root.render(<Host selected="r1" onSelect={onSelect} />));
    expect(bodyRows().map((r) => r.tabIndex)).toEqual([-1, 0, -1]);
    expect(bodyRows()[1]!.getAttribute("aria-selected")).toBe("true");
    act(() => bodyRows()[1]!.focus());
    pressKey(bodyRows()[1]!, "ArrowDown");
    expect(document.activeElement).toBe(bodyRows()[2]);
    expect(bodyRows().map((r) => r.tabIndex)).toEqual([-1, -1, 0]);
    // The selection follows the focus.
    expect(onSelect).toHaveBeenLastCalledWith("r2");
    expect(bodyRows()[2]!.getAttribute("aria-selected")).toBe("true");
  });

  it("a_click_on_a_row_selects_it", () => {
    const onSelect = vi.fn();
    act(() => root.render(<Host onSelect={onSelect} />));
    act(() => bodyRows()[0]!.click());
    expect(onSelect).toHaveBeenCalledWith("r0");
  });

  it("enter_goes_into_the_first_field_of_the_row_and_escape_back_to_the_row", () => {
    act(() => root.render(<Host selected="r1" />));
    const row = bodyRows()[1]!;
    act(() => row.focus());
    pressKey(row, "Enter");
    const field = row.querySelector("input")!;
    expect(document.activeElement).toBe(field);
    expect(field.getAttribute("aria-label")).toBe("Level von Ruckzuckhieb");
    pressKey(field, "Escape");
    expect(document.activeElement).toBe(row);
  });

  it("an_escape_the_field_used_itself_leaves_the_focus_in_the_field", () => {
    act(() => root.render(<Host selected="r1" />));
    const row = bodyRows()[1]!;
    const field = row.querySelector("input")!;
    act(() => field.focus());
    field.addEventListener("keydown", (e) => e.preventDefault());
    pressKey(field, "Escape");
    expect(document.activeElement).toBe(field);
  });

  it("delete_calls_on_delete_row_only_with_the_focus_on_the_row", () => {
    const onDeleteRow = vi.fn();
    act(() => root.render(<Host selected="r1" onDeleteRow={onDeleteRow} />));
    const row = bodyRows()[1]!;
    const field = row.querySelector("input")!;
    act(() => field.focus());
    pressKey(field, "Delete");
    expect(onDeleteRow).not.toHaveBeenCalled();
    act(() => row.focus());
    pressKey(row, "Delete");
    expect(onDeleteRow).toHaveBeenCalledExactlyOnceWith("r1");
  });

  it("tab_from_a_row_leaves_the_table_instead_of_walking_its_fields", () => {
    act(() => root.render(<Host selected="r1" />));
    const row = bodyRows()[1]!;
    // The rows are one stop: no field of any row is in the Tab order while the focus is outside the rows.
    expect(tabOrder()).toEqual([outside("Davor"), row, outside("Danach")]);
    act(() => row.focus());
    expect(tabOrder()).toEqual([outside("Davor"), row, outside("Danach")]);
    tab(row);
    expect(document.activeElement).toBe(outside("Danach"));
    act(() => row.focus());
    tab(row, true);
    expect(document.activeElement).toBe(outside("Davor"));
  });

  it("a_table_at_the_end_of_the_page_lets_tab_go_on", () => {
    act(() => root.render(<Table columns={columns} rows={learnset} ariaLabel="Lernset" selected="r0" />));
    const row = bodyRows()[0]!;
    act(() => row.focus());
    // Nothing after the table: the table does not take the key, the browser moves on (to its own controls).
    expect(pressKey(row, "Tab").defaultPrevented).toBe(false);
    expect(pressKey(row, "Tab", { shiftKey: true }).defaultPrevented).toBe(false);
  });

  it("after_enter_the_fields_of_that_row_only_join_the_tab_order_and_tab_cycles_through_them", () => {
    act(() => root.render(<Host selected="r1" />));
    const row = bodyRows()[1]!;
    act(() => row.focus());
    pressKey(row, "Enter");
    const [field, button] = [row.querySelector("input")!, row.querySelector("button")!];
    expect(document.activeElement).toBe(field);
    expect(tabOrder()).toEqual([outside("Davor"), row, field, button, outside("Danach")]);
    tab(field);
    expect(document.activeElement).toBe(button);
    tab(button);
    expect(document.activeElement).toBe(field);
    tab(field, true);
    expect(document.activeElement).toBe(button);
    pressKey(button, "Escape");
    expect(document.activeElement).toBe(row);
    expect(tabOrder()).toEqual([outside("Davor"), row, outside("Danach")]);
  });

  it("tabbing_in_from_outside_lands_on_the_selected_row_and_selects_nothing_else", () => {
    const onSelect = vi.fn();
    act(() => root.render(<Host selected="r1" onSelect={onSelect} />));
    const before = outside("Davor");
    act(() => before.focus());
    tab(before);
    expect(document.activeElement).toBe(bodyRows()[1]);
    expect(onSelect).not.toHaveBeenCalled();
    expect(bodyRows().map((r) => r.getAttribute("aria-selected"))).toEqual(["false", "true", "false"]);
    // And back in from behind.
    act(() => outside("Danach").focus());
    tab(outside("Danach"), true);
    expect(document.activeElement).toBe(bodyRows()[1]);
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("a_click_into_a_field_of_another_row_keeps_the_focus_there_and_makes_that_row_the_selected_stop", () => {
    const onSelect = vi.fn();
    act(() => root.render(<Host selected="r1" onSelect={onSelect} />));
    const row0 = bodyRows()[0]!;
    const field = row0.querySelector("input")!;
    act(() => {
      field.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
      field.focus();
    });
    expect(document.activeElement).toBe(field);
    expect(onSelect).toHaveBeenLastCalledWith("r0");
    expect(row0.getAttribute("aria-selected")).toBe("true");
    expect(tabOrder()).toEqual([outside("Davor"), row0, field, row0.querySelector("button"), outside("Danach")]);
  });

  it("a_field_that_sets_its_own_tab_index_later_stays_out_of_the_tab_order", () => {
    act(() => root.render(<Host selected="r1" />));
    const field = bodyRows()[0]!.querySelector("input")!;
    act(() => field.setAttribute("tabindex", "0"));
    // The observer runs after the mutation.
    return Promise.resolve().then(() => {
      expect(field.tabIndex).toBe(-1);
      act(() => field.focus());
      expect(field.getAttribute("tabindex")).toBe("0");
    });
  });

  it("backspace_on_the_row_deletes_it_too_but_not_from_a_field", () => {
    const onDeleteRow = vi.fn();
    act(() => root.render(<Host selected="r1" onDeleteRow={onDeleteRow} />));
    const row = bodyRows()[1]!;
    const field = row.querySelector("input")!;
    act(() => field.focus());
    pressKey(field, "Backspace");
    expect(onDeleteRow).not.toHaveBeenCalled();
    act(() => row.focus());
    pressKey(row, "Backspace");
    expect(onDeleteRow).toHaveBeenCalledExactlyOnceWith("r1");
  });

  it("a_disabled_row_is_marked_skipped_by_the_arrows_and_not_selected_or_deleted", () => {
    const onSelect = vi.fn();
    const onDeleteRow = vi.fn();
    const rows = learnset.map((r, i) => (i === 1 ? { ...r, disabled: true } : r));
    act(() =>
      root.render(<Table columns={columns} rows={rows} ariaLabel="Lernset" selected="r0" onSelect={onSelect} onDeleteRow={onDeleteRow} />),
    );
    expect(bodyRows()[1]!.getAttribute("aria-disabled")).toBe("true");
    act(() => bodyRows()[0]!.focus());
    pressKey(bodyRows()[0]!, "ArrowDown");
    expect(document.activeElement).toBe(bodyRows()[2]);
    act(() => bodyRows()[1]!.click());
    expect(onSelect).not.toHaveBeenCalledWith("r1");
    pressKey(bodyRows()[1]!, "Delete");
    expect(onDeleteRow).not.toHaveBeenCalled();
  });
});
