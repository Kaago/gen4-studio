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
});
