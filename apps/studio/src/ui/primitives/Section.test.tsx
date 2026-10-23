// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { EmptyState } from "./EmptyState";
import { InspectorRow } from "./InspectorRow";
import { PanelHead } from "./PanelHead";
import { Section } from "./Section";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Section, InspectorRow, PanelHead and EmptyState", () => {
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

  it("section_head_is_a_button_whose_aria_expanded_follows_open", () => {
    act(() =>
      root.render(
        <Section title="Ereignisse" count={4} open onToggle={() => {}}>
          <p>Inhalt</p>
        </Section>,
      ),
    );
    const head = () => container.querySelector<HTMLElement>("button[aria-expanded]")!;
    expect(head().getAttribute("aria-expanded")).toBe("true");
    expect(head().textContent).toContain("Ereignisse");
    expect(head().textContent).toContain("4");
    expect(container.textContent).toContain("Inhalt");
    act(() =>
      root.render(
        <Section title="Ereignisse" open={false} onToggle={() => {}}>
          <p>Inhalt</p>
        </Section>,
      ),
    );
    expect(head().getAttribute("aria-expanded")).toBe("false");
    expect(container.textContent).not.toContain("Inhalt");
  });

  it("section_reports_a_toggle_click_and_leaves_the_state_to_its_owner", () => {
    const onToggle = vi.fn();
    act(() =>
      root.render(
        <Section title="Ereignisse" open onToggle={onToggle}>
          <p>Inhalt</p>
        </Section>,
      ),
    );
    act(() => container.querySelector<HTMLElement>("button[aria-expanded]")!.click());
    expect(onToggle).toHaveBeenCalledTimes(1);
    expect(container.querySelector("button[aria-expanded]")!.getAttribute("aria-expanded")).toBe("true");
  });

  it("section_content_is_a_region_named_by_its_head", () => {
    act(() =>
      root.render(
        <Section title="Ereignisse" open onToggle={() => {}}>
          <p>Inhalt</p>
        </Section>,
      ),
    );
    const region = container.querySelector("[role=region]")!;
    expect(region.getAttribute("aria-labelledby")).toBe(container.querySelector("button[aria-expanded]")!.id);
    expect(container.querySelector("button[aria-expanded]")!.getAttribute("aria-controls")).toBe(region.id);
  });

  it("inspector_row_shows_label_children_and_hint", () => {
    act(() =>
      root.render(
        <InspectorRow label="Modell" hint="16 Bit">
          <input aria-label="Modell" />
        </InspectorRow>,
      ),
    );
    expect(container.textContent).toContain("Modell");
    expect(container.textContent).toContain("16 Bit");
    expect(container.querySelector("input")).not.toBeNull();
  });

  it("inspector_row_names_a_deviation_in_text_not_only_by_color", () => {
    act(() => root.render(<InspectorRow label="Modell">x</InspectorRow>));
    expect(container.textContent).not.toContain("weicht ab");
    act(() => root.render(<InspectorRow label="Modell" deviates>x</InspectorRow>));
    expect(container.textContent).toContain("weicht ab");
  });

  it("panel_head_has_title_subtitle_and_actions_in_a_header", () => {
    act(() => root.render(<PanelHead title="Orte" subtitle="Platin" actions={<button type="button">Neu</button>} />));
    expect(container.querySelector("header")).not.toBeNull();
    expect(container.querySelector("h2")?.textContent).toBe("Orte");
    expect(container.textContent).toContain("Platin");
    expect(container.querySelector("header button")?.textContent).toBe("Neu");
  });

  it("empty_state_says_what_and_why_and_offers_an_action", () => {
    const onClick = vi.fn();
    act(() => root.render(<EmptyState what="Noch kein Projekt" why="Öffne einen entpackten DSPRE-Ordner." action={{ label: "Projekt öffnen", onClick }} />));
    expect(container.textContent).toContain("Noch kein Projekt");
    expect(container.textContent).toContain("Öffne einen entpackten DSPRE-Ordner.");
    act(() => container.querySelector<HTMLElement>("button")!.click());
    expect(onClick).toHaveBeenCalledTimes(1);
  });

  it("empty_state_without_action_has_no_button", () => {
    act(() => root.render(<EmptyState what="Nichts da" why="Wähle links einen Ort." />));
    expect(container.querySelector("button")).toBeNull();
  });
});
