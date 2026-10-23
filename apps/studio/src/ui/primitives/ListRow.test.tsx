// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ListRow } from "./ListRow";
import { TreeRow } from "./TreeRow";
import { pressKey } from "./testUtils";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ListRow and TreeRow", () => {
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

  it("list_row_is_an_option_whose_aria_selected_follows_selected", () => {
    act(() => root.render(<ListRow label="Route 201" selected={false} onSelect={() => {}} />));
    const row = () => container.querySelector("[role=option]")!;
    expect(row().getAttribute("aria-selected")).toBe("false");
    expect(row().textContent).toContain("Route 201");
    act(() => root.render(<ListRow label="Route 201" selected onSelect={() => {}} />));
    expect(row().getAttribute("aria-selected")).toBe("true");
  });

  it("list_row_shows_tech_and_trailing_content_and_passes_data_attributes", () => {
    act(() =>
      root.render(
        <ListRow
          label="Zweiblattdorf"
          tech="0412"
          icon="world"
          selected={false}
          onSelect={() => {}}
          trailing={<span>3 Verweise</span>}
          dataAttrs={{ "data-archive": 412 }}
        />,
      ),
    );
    const row = container.querySelector("[role=option]")!;
    expect(row.textContent).toContain("0412");
    expect(row.textContent).toContain("3 Verweise");
    expect(row.getAttribute("data-archive")).toBe("412");
  });

  it("list_row_selects_on_click_enter_and_space", () => {
    const onSelect = vi.fn();
    act(() => root.render(<ListRow label="Route 201" selected={false} onSelect={onSelect} />));
    const row = container.querySelector<HTMLElement>("[role=option]")!;
    act(() => row.click());
    pressKey(row, "Enter");
    pressKey(row, " ");
    pressKey(row, "a");
    expect(onSelect).toHaveBeenCalledTimes(3);
  });

  it("tree_row_is_a_treeitem_with_level_and_expanded_state", () => {
    act(() => root.render(<TreeRow label="Sinnoh" depth={2} expanded={false} onToggle={() => {}} selected={false} onSelect={() => {}} />));
    const row = () => container.querySelector("[role=treeitem]")!;
    expect(row().getAttribute("aria-level")).toBe("3");
    expect(row().getAttribute("aria-expanded")).toBe("false");
    act(() => root.render(<TreeRow label="Sinnoh" depth={2} expanded onToggle={() => {}} selected onSelect={() => {}} />));
    expect(row().getAttribute("aria-expanded")).toBe("true");
    expect(row().getAttribute("aria-selected")).toBe("true");
  });

  it("a_tree_row_without_expanded_is_a_leaf_without_aria_expanded", () => {
    act(() => root.render(<TreeRow label="Raum 3" depth={1} selected={false} onSelect={() => {}} />));
    expect(container.querySelector("[role=treeitem]")!.hasAttribute("aria-expanded")).toBe(false);
  });

  it("tree_row_toggles_with_arrow_keys_and_the_caret_without_selecting", () => {
    const onToggle = vi.fn();
    const onSelect = vi.fn();
    act(() => root.render(<TreeRow label="Sinnoh" depth={0} expanded={false} onToggle={onToggle} selected={false} onSelect={onSelect} />));
    const row = container.querySelector<HTMLElement>("[role=treeitem]")!;
    pressKey(row, "ArrowRight");
    expect(onToggle).toHaveBeenCalledTimes(1);
    act(() => root.render(<TreeRow label="Sinnoh" depth={0} expanded onToggle={onToggle} selected={false} onSelect={onSelect} />));
    pressKey(row, "ArrowRight");
    expect(onToggle).toHaveBeenCalledTimes(1);
    pressKey(row, "ArrowLeft");
    expect(onToggle).toHaveBeenCalledTimes(2);
    expect(onSelect).not.toHaveBeenCalled();
  });

  it("rows_carry_an_explicit_name_and_a_hover_hint_when_given", () => {
    act(() => root.render(<ListRow label="Zweiblattdorf" ariaLabel="Zweiblattdorf, 3 Header" title="Drei Header" selected={false} onSelect={() => {}} />));
    const row = container.querySelector("[role=option]")!;
    expect(row.getAttribute("aria-label")).toBe("Zweiblattdorf, 3 Header");
    expect(row.getAttribute("title")).toBe("Drei Header");
    act(() => root.render(<TreeRow label="Ort" ariaLabel="Ort, 3 Header" title="Hinweis" depth={0} selected={false} onSelect={() => {}} />));
    const item = container.querySelector("[role=treeitem]")!;
    expect(item.getAttribute("aria-label")).toBe("Ort, 3 Header");
    expect(item.getAttribute("title")).toBe("Hinweis");
  });

  it("a_disabled_row_is_aria_disabled_and_does_not_select", () => {
    const onSelect = vi.fn();
    act(() => root.render(<ListRow label="Gesperrt" selected={false} onSelect={onSelect} disabled />));
    const row = container.querySelector<HTMLElement>("[role=option]")!;
    expect(row.getAttribute("aria-disabled")).toBe("true");
    act(() => row.click());
    pressKey(row, "Enter");
    act(() => root.render(<TreeRow label="Gesperrt" depth={1} selected={false} onSelect={onSelect} disabled />));
    const item = container.querySelector<HTMLElement>("[role=treeitem]")!;
    expect(item.getAttribute("aria-disabled")).toBe("true");
    act(() => item.click());
    pressKey(item, " ");
    expect(onSelect).not.toHaveBeenCalled();
  });
});
