// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { ProjectPreview } from "./ProjectPreview";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ProjectPreview", () => {
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

  it("shows the picture on the DS screen, decorative", () => {
    act(() => root.render(<ProjectPreview name="platin-hack" png="AAAA" />));
    const screen = container.querySelector('[aria-label="DS-Bildschirm, leer"]')!;
    const img = screen.querySelector("img")!;
    expect(img.getAttribute("alt")).toBe("");
    expect(img.getAttribute("src")).toBe("data:image/png;base64,AAAA");
    expect(img.getAttribute("width")).toBe("256");
    expect(img.getAttribute("height")).toBe("192");
    expect(container.textContent).toBe("");
  });

  it("shows the name on the empty screen without a picture, hidden from the accessibility tree", () => {
    act(() => root.render(<ProjectPreview name="platin-hack" png={null} />));
    const name = container.querySelector("[data-preview-name]")!;
    expect(name.textContent).toBe("platin-hack");
    expect(name.getAttribute("aria-hidden")).toBe("true");
    expect(container.querySelector("img")).toBeNull();
  });
});
