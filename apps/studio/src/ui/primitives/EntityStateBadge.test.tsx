// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { EntityStateBadge } from "./EntityStateBadge";
import type { EntityState } from "./EntityStateBadge";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("EntityStateBadge", () => {
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

  it("says_geaendert_neu_and_geloescht_in_the_colour_of_each", () => {
    const cases: [EntityState, string, string][] = [
      ["changed", "geändert", "warning"],
      ["new", "neu", "info"],
      ["deleted", "gelöscht", "error"],
    ];
    for (const [state, word, kind] of cases) {
      act(() => root.render(<EntityStateBadge state={state} />));
      const badge = container.querySelector<HTMLElement>("[data-entity-state]");
      expect(badge?.getAttribute("data-entity-state")).toBe(state);
      expect(badge?.textContent).toBe(word);
      // The badge of the kind (its colour class and symbol).
      expect(badge?.querySelector(`[class*=${kind}]`)).not.toBeNull();
      expect(badge?.querySelector("svg")).not.toBeNull();
    }
  });

  it("compact_is_the_symbol_named_by_the_word", () => {
    act(() => root.render(<EntityStateBadge state="deleted" compact />));
    const badge = container.querySelector<HTMLElement>("[data-entity-state]");
    expect(badge?.getAttribute("data-entity-state")).toBe("deleted");
    expect(badge?.textContent).toBe("");
    expect(badge?.getAttribute("role")).toBe("img");
    expect(badge?.getAttribute("aria-label")).toBe("gelöscht");
    expect(badge?.getAttribute("title")).toBe("gelöscht");
    expect(badge?.querySelector("[class*=error] svg")).not.toBeNull();
  });
});
