// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { HeaderSummary } from "../../api";
import { HeaderSearch } from "./HeaderSearch";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const header = (id: number): HeaderSummary => ({ id, internal_name: `H${id}`, location_name: "", matrix_id: 0, area_data_id: 0, event_file_id: 0, text_archive_id: 0, script_file_id: 0, level_script_id: 0 });

describe("HeaderSearch", () => {
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
  const rows = () => [...container.querySelectorAll("li")].map((li) => li.textContent);

  it("lists_the_headers_with_nothing_extra_by_default", () => {
    act(() => root.render(<HeaderSearch headers={[header(1), header(2)]} current={1} onPick={() => {}} />));
    expect(rows()).toEqual(["0001 H1", "0002 H2"]);
  });

  it("adds_what_renderExtra_returns_to_each_row", () => {
    act(() => root.render(<HeaderSearch headers={[header(1), header(2)]} current={1} onPick={() => {}} renderExtra={(h) => `n=${h.id * 10}`} />));
    expect(rows()).toEqual(["0001 H1n=10", "0002 H2n=20"]);
  });
});
