// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { InspectorHead } from "./InspectorHead";
import type { InspectorHeadProps } from "./InspectorHead";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("InspectorHead", () => {
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
  const show = (props: InspectorHeadProps) => act(() => root.render(<InspectorHead {...props} />));
  const text = () => container.textContent ?? "";

  it("shows_the_title_as_a_heading_and_the_subtitle", () => {
    show({ title: "Objekt 2 · ID 1", subtitle: "Figur 3 von 5 in Zweiblattdorf" });
    expect(container.querySelector("h2")?.textContent).toBe("Objekt 2 · ID 1");
    expect(text()).toContain("Figur 3 von 5 in Zweiblattdorf");
  });

  it("shows_no_badge_for_a_plain_record", () => {
    show({ title: "Zweiblattdorf" });
    expect(text()).not.toContain("Ungespeichert");
    expect(text()).not.toContain("Gesperrt");
    expect(text()).not.toContain("Nur Ansicht");
  });

  it("shows_the_badges_the_props_ask_for", () => {
    show({ title: "Zweiblattdorf", unsaved: true });
    expect(text()).toContain("Ungespeichert");
    expect(text()).not.toContain("Gesperrt");
    show({ title: "Zweiblattdorf", locked: true });
    expect(text()).toContain("Gesperrt");
    expect(text()).not.toContain("Ungespeichert");
  });

  it("marks_a_hover_view_as_view_only", () => {
    show({ title: "Warp 0", viewOnly: true });
    expect(text()).toContain("Nur Ansicht");
  });
});
