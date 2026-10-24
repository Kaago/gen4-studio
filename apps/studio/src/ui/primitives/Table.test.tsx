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
    cells: { level: <input aria-label={`Level von ${move}`} defaultValue={level} />, move },
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

  function Host(props: Partial<TableProps>) {
    const [selected, setSelected] = useState<string | undefined>(props.selected);
    return (
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
