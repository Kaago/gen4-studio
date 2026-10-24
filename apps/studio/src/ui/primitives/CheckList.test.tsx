// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { CheckList } from "./CheckList";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("CheckList", () => {
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

  it("is_a_list_whose_rows_say_their_state_with_a_symbol_and_words", () => {
    act(() =>
      root.render(
        <CheckList
          ariaLabel="Teile"
          rows={[
            { id: "a", state: "ok", text: "Modell" },
            { id: "b", state: "error", text: "Kollision", trailing: <button type="button">x</button> },
          ]}
        />,
      ),
    );
    const list = container.querySelector("[role=list][aria-label='Teile']")!;
    const rows = [...list.querySelectorAll("li")];
    expect(rows.map((r) => r.dataset.state)).toEqual(["ok", "error"]);
    expect(rows[0]!.querySelector("[role=img]")?.getAttribute("aria-label")).toBe("In Ordnung");
    expect(rows[1]!.querySelector("[role=img]")?.getAttribute("aria-label")).toBe("Fehler");
    expect(rows[1]!.querySelector("svg")).not.toBeNull();
    expect(rows[1]!.querySelector("button")?.textContent).toBe("x");
  });
});
