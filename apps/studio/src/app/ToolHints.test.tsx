// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ToolHints } from "./ToolHints";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ToolHints", () => {
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

  it("names_the_active_tool_and_pairs_each_key_with_what_it_does", () => {
    act(() => root.render(<ToolHints tool="select" />));
    expect(container.textContent).toBe("Auswählen·Klick wählt·Ziehen verschiebt");
    expect([...container.querySelectorAll("kbd")].map((k) => k.textContent)).toEqual(["Klick", "Ziehen"]);
  });

  it("follows_the_tool", () => {
    act(() => root.render(<ToolHints tool="paint" />));
    expect(container.textContent).toContain("Gelände malen");
    expect(container.textContent).toContain("Klick setzt Kollision");
    expect(container.textContent).toContain("⇧Klick löscht");
    act(() => root.render(<ToolHints tool="door" />));
    expect(container.textContent).toContain("Klick legt eine Tür an");
    expect(container.textContent).toContain("Esc bricht ab");
  });
});
