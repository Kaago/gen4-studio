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

  it("shows_rows_of_the_thread_overview_with_a_story_chip", () => {
    const block = container.querySelector<HTMLElement>('[data-gallery-state="threads"]')!;
    expect(block.querySelectorAll("[data-thread-row]")).toHaveLength(2);
    expect(block.querySelector("[data-thread-row='16448'] [data-thread-name]")?.textContent).toBe("VAR_RIVALIN");
    expect(block.querySelector("[data-thread-row='16448'] [data-thread-tech]")?.textContent).toBe("0x4040");
    expect(block.querySelector("[data-thread-row='16448'] [data-thread-story]")?.textContent).toContain("Rivalin");
    expect(block.textContent).toContain("3 Orte: Zweiblattdorf, Route 201, …");
    // A click on a row reads no graph (the gallery has no project to read it from): nothing hangs on „Graph wird gelesen …“.
    act(() => block.querySelector<HTMLElement>("[data-thread-row='16448']")!.click());
    expect(block.textContent).not.toContain("Graph wird gelesen");
    expect(block.querySelector("[data-thread-graph-box]")).toBeNull();
  });

  it("shows_story_rows_and_the_new_story_dialog", async () => {
    const block = container.querySelector<HTMLElement>('[data-gallery-state="stories"]')!;
    expect(block.querySelectorAll("[data-story-row]")).toHaveLength(2);
    expect(block.querySelector("[data-story-row=STORY_RIVALIN] [data-story-progress]")?.textContent).toBe("VAR_164480x4040");
    expect(block.querySelector("[data-story-row=STORY_0001] [role=status]")?.textContent).toContain("unbekanntes Feld");
    await act(async () => block.querySelector<HTMLElement>("[data-new-story]")!.click());
    const dialog = document.querySelector<HTMLElement>("[data-new-story-dialog]")!;
    expect(dialog.querySelector("h2")?.textContent).toBe("Neue Geschichte");
    expect([...dialog.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === "Geschichte anlegen")?.disabled).toBe(true);
  });
});
