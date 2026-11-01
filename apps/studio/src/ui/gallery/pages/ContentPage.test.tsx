// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { readCss, ruleBody } from "../../cssSource";
import { ContentPage } from "./ContentPage";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ContentPage", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<ContentPage />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("shows_inspector_rows_in_an_inspector_as_narrow_as_the_werkbanks", () => {
    const narrow = container.querySelector<HTMLElement>('[data-gallery-state="inspector-schmal"]')!;
    expect(narrow.textContent).toContain("Zweiblattdorf");
    expect([...narrow.querySelectorAll("button")].map((b) => b.textContent)).toContain("Ändern…");
    // The box is the container the rows ask, 280 px wide.
    const css = readCss("../Gallery.module.css", import.meta.url);
    expect(ruleBody(css, ".inspectorNarrow")).toMatch(/container:\s*inspector\s*\/\s*inline-size/);
    expect(ruleBody(css, ".inspectorNarrow")).toMatch(/width:\s*280px/);
  });

  it("shows_a_change_table_with_word_spans", () => {
    const block = container.querySelector<HTMLElement>('[data-gallery-state="change-table"]')!;
    expect(block.querySelector("table caption")?.textContent).toBe("Trainer 0001 gegen den letzten Commit");
    expect(block.querySelector("del")?.textContent).toBe("Zweiblattdorf");
    expect(block.querySelector("ins")?.textContent).toBe("Sandgemme");
    expect(block.textContent).toContain("und 3 weitere Felder");
  });

  it("shows_a_flag_row_and_its_detail_with_two_ways", () => {
    const block = container.querySelector<HTMLElement>('[data-gallery-state="flags"]')!;
    expect(block.querySelectorAll("[data-flag-row]")).toHaveLength(1);
    expect(block.querySelector("[data-flag-detail] h2")?.textContent).toBe("FLAG_HAT_MEDAILLE");
    expect(block.querySelectorAll("[data-flag-use]")).toHaveLength(2);
  });

  it("shows_the_seven_story_kinds_at_rest_and_selected_and_a_noted_only_node", () => {
    const block = container.querySelector<HTMLElement>('[data-gallery-state="story-nodes"]')!;
    const kinds = ["start", "dialog", "condition", "battle", "reward", "progress", "appearance"];
    for (const kind of kinds) {
      expect(block.querySelectorAll(`[data-story-node][data-kind="${kind}"]:not([data-selected])`).length).toBeGreaterThanOrEqual(1);
      expect(block.querySelectorAll(`[data-story-node][data-kind="${kind}"][data-selected]`)).toHaveLength(1);
    }
    expect(block.querySelectorAll("[data-story-node][data-noted-only]")).toHaveLength(1);
    expect(block.querySelectorAll('[data-story-node][data-kind="value"]')).toHaveLength(1);
    expect(block.querySelectorAll('[data-story-node][data-kind="place"][data-loose]')).toHaveLength(1);
    expect(block.querySelector('[data-story-node][role="button"]')?.getAttribute("aria-label")).toBe("Ort: Zweiblattdorf, Scriptdatei 0101");
  });
});
