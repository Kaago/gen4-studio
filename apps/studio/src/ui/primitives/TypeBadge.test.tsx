// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { TypeBadge } from "./TypeBadge";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("TypeBadge", () => {
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

  const badge = () => container.querySelector<HTMLElement>("[data-type]")!;

  it("names_the_type_in_text_and_carries_its_number", () => {
    act(() => root.render(<TypeBadge type={10} />));
    expect(badge().textContent).toBe("Feuer");
    expect(badge().getAttribute("data-type")).toBe("10");
    expect(badge().hasAttribute("data-unknown")).toBe(false);
  });

  it("names_an_unknown_type_by_its_number", () => {
    act(() => root.render(<TypeBadge type={19} size="s" />));
    expect(badge().textContent).toBe("Typ 19");
    expect(badge().getAttribute("data-type")).toBe("19");
    expect(badge().hasAttribute("data-unknown")).toBe(true);
  });
});
