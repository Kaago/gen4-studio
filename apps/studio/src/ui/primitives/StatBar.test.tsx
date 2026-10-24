// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { StatBar } from "./StatBar";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("StatBar", () => {
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

  const meter = () => container.querySelector<HTMLElement>("[role=meter]")!;
  const nameOf = (el: HTMLElement) => document.getElementById(el.getAttribute("aria-labelledby") ?? "")?.textContent;

  it("is_a_meter_named_by_its_label_with_value_and_range", () => {
    act(() => root.render(<StatBar label="Angriff" value={85} average={63.4} />));
    expect(nameOf(meter())).toBe("Angriff");
    expect(meter().getAttribute("aria-valuenow")).toBe("85");
    expect(meter().getAttribute("aria-valuemin")).toBe("0");
    expect(meter().getAttribute("aria-valuemax")).toBe("255");
    expect(container.textContent).toContain("85");
  });

  it("says_in_text_where_the_value_stands_against_the_average", () => {
    act(() => root.render(<StatBar label="Angriff" value={85} average={63.4} />));
    expect(container.textContent).toContain("über Schnitt");
    expect(meter().getAttribute("aria-valuetext")).toBe("85, über Schnitt");
    act(() => root.render(<StatBar label="Spezial-Angriff" value={40} average={63.4} />));
    expect(container.textContent).toContain("unter Schnitt");
    act(() => root.render(<StatBar label="Initiative" value={63} average={63.4} />));
    expect(container.textContent).toContain("im Schnitt");
  });

  it("takes_another_maximum", () => {
    act(() => root.render(<StatBar label="Anteil" value={20} max={100} average={10} />));
    expect(meter().getAttribute("aria-valuemax")).toBe("100");
  });

  it("names_a_deviation_from_the_original_in_text", () => {
    act(() => root.render(<StatBar label="Angriff" value={85} average={63} deviates />));
    expect(container.querySelector("[data-deviates]")).not.toBeNull();
    expect(container.textContent).toContain("weicht ab");
    act(() => root.render(<StatBar label="Angriff" value={85} average={63} />));
    expect(container.querySelector("[data-deviates]")).toBeNull();
    expect(container.textContent).not.toContain("weicht ab");
  });
});
