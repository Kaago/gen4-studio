// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { pages } from "../../workbench/text/textModel";
import { DSScreen } from "./DSScreen";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("DSScreen", () => {
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

  const three = pages("Hallo!\\nDies ist Zeile zwei.\\rNoch eine Seite.\\fUnd die dritte.");
  const group = () => container.querySelector<HTMLElement>("[role=group]");

  it("names_the_page_and_shows_the_prompt_arrow_of_a_page_that_waits", () => {
    expect(three).toHaveLength(3);
    act(() => root.render(<DSScreen variant="textbox" pages={three} page={1} scale={1} />));
    expect(group()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 2 von 3");
    expect(group()?.textContent).toContain("Noch eine Seite.");
    const arrow = [...container.querySelectorAll("[aria-hidden=true]")].find((e) => e.textContent === "▼");
    expect(arrow).toBeDefined();
  });

  it("shows_no_prompt_arrow_on_the_last_page", () => {
    act(() => root.render(<DSScreen variant="textbox" pages={three} page={2} scale={1} />));
    expect(group()?.getAttribute("aria-label")).toBe("DS-Bildschirm, Seite 3 von 3");
    expect(container.textContent).not.toContain("▼");
  });

  it("shows_a_control_code_as_a_code_badge_with_its_text", () => {
    act(() => root.render(<DSScreen variant="textbox" pages={pages("Hallo {COLOR, 0, 5}Welt")} page={0} scale={1} />));
    const badge = [...container.querySelectorAll("span")].find((e) => e.textContent === "{COLOR, 0, 5}");
    expect(badge).toBeDefined();
    expect(container.textContent).toContain("Hallo ");
    expect(container.textContent).toContain("Welt");
  });

  it("shows_an_unknown_code_as_a_question_mark_titled_with_the_code", () => {
    act(() => root.render(<DSScreen variant="textbox" pages={pages("A\\x0123B")} page={0} scale={1} />));
    const badge = container.querySelector("[title]");
    expect(badge?.textContent).toBe("?");
    expect(badge?.getAttribute("title")).toBe("\\x0123");
  });

  it("renders_the_same_text_as_a_sign", () => {
    act(() => root.render(<DSScreen variant="sign" pages={pages("Route 201\\nZweiblattdorf")} page={0} scale={2} />));
    expect(group()?.textContent).toContain("Route 201");
    expect(group()?.textContent).toContain("Zweiblattdorf");
  });

  it("warns_when_a_line_runs_out_of_the_box", () => {
    const over = pages("a\\nb\\nc");
    expect(over[1].overflow).toBe(true);
    act(() => root.render(<DSScreen variant="textbox" pages={over} page={1} scale={1} />));
    expect(container.querySelector("[aria-label='Zeile läuft aus der Box']")).not.toBeNull();
    act(() => root.render(<DSScreen variant="textbox" pages={over} page={0} scale={1} />));
    expect(container.querySelector("[aria-label='Zeile läuft aus der Box']")).toBeNull();
  });

  it("shows_only_the_ground_and_the_background_when_empty", () => {
    act(() => root.render(<DSScreen variant="empty" pages={three} page={0} scale={1} background={<i data-testid="karte" />} />));
    expect(container.querySelector("[data-testid=karte]")).not.toBeNull();
    expect(group()?.textContent).toBe("");
  });

  it("sizes_itself_by_the_integer_scale_through_a_custom_property", () => {
    act(() => root.render(<DSScreen variant="empty" pages={[]} page={0} scale={3} />));
    expect(group()?.style.getPropertyValue("--ds-scale")).toBe("3");
  });

  it("falls_back_to_scale_one_when_none_is_given_and_ResizeObserver_is_missing", () => {
    act(() => root.render(<DSScreen variant="empty" pages={[]} page={0} />));
    expect(group()?.style.getPropertyValue("--ds-scale")).toBe("1");
  });
});
