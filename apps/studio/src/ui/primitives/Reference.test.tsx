// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { Reference } from "./Reference";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("Reference", () => {
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

  const chip = () => container.querySelector<HTMLButtonElement>("button")!;
  const rightClick = () => act(() => void chip().dispatchEvent(new MouseEvent("contextmenu", { bubbles: true, cancelable: true, button: 2, clientX: 10, clientY: 10 })));
  const entries = () => [...document.querySelectorAll<HTMLElement>("[role=menuitem]")];

  it("shows_label_and_tech_and_opens_on_click", () => {
    const onOpen = vi.fn();
    act(() => root.render(<Reference area="pokemon" label="Staralili" tech="#396" onOpen={onOpen} />));
    expect(chip().textContent).toContain("Staralili");
    expect(chip().textContent).toContain("#396");
    act(() => chip().click());
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("passes_data_attributes_to_the_chip", () => {
    const onOpen = vi.fn();
    act(() => root.render(<Reference area="welt" label="Zweiblattdorf" tech="0012" onOpen={onOpen} dataAttrs={{ "data-header": 12 }} />));
    expect(chip().getAttribute("data-header")).toBe("12");
    act(() => container.querySelector<HTMLElement>('[data-header="12"]')!.click());
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("a_title_is_the_chips_tooltip", () => {
    act(() => root.render(<Reference area="geschichten" label="Heimkehr" onOpen={() => {}} />));
    expect(chip().hasAttribute("title")).toBe(false);
    act(() => root.render(<Reference area="geschichten" label="Heimkehr" title="STORY_HEIMKEHR" onOpen={() => {}} />));
    expect(chip().title).toBe("STORY_HEIMKEHR");
    expect(chip().textContent).toBe("Heimkehr");
  });

  it("a_disabled_reference_stays_visible_and_does_not_open", () => {
    const onOpen = vi.fn();
    act(() => root.render(<Reference area="welt" label="Scriptdatei 0040" onOpen={onOpen} disabled />));
    expect(chip().disabled).toBe(true);
    act(() => chip().click());
    expect(onOpen).not.toHaveBeenCalled();
  });

  it.each(["welt", "geschichten", "pokemon", "gegenstaende", "texte", "technik"] as const)("renders_the_%s_area_as_a_button", (area) => {
    act(() => root.render(<Reference area={area} label="Ziel" onOpen={() => {}} />));
    expect(chip().textContent).toContain("Ziel");
  });

  it("right_click_opens_a_used_in_menu_listing_where_it_is_used", () => {
    const first = vi.fn();
    act(() =>
      root.render(
        <Reference
          area="texte"
          label="Textarchiv 0412"
          onOpen={() => {}}
          usedIn={[
            { label: "Zweiblattdorf", onOpen: first },
            { label: "Route 201", onOpen: () => {} },
          ]}
        />,
      ),
    );
    expect(document.querySelector("[role=menu]")).toBeNull();
    rightClick();
    expect(document.querySelector("[role=menu]")).not.toBeNull();
    expect(document.body.textContent).toContain("Verwendet in");
    expect(entries().map((e) => e.textContent)).toEqual(["Zweiblattdorf", "Route 201"]);
    act(() => entries()[0]!.click());
    expect(first).toHaveBeenCalledTimes(1);
  });

  it("right_click_does_not_open_the_target", () => {
    const onOpen = vi.fn();
    act(() => root.render(<Reference area="texte" label="Textarchiv 0412" onOpen={onOpen} usedIn={[{ label: "Zweiblattdorf", onOpen: () => {} }]} />));
    rightClick();
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("an_unused_reference_offers_a_disabled_nowhere_entry", () => {
    act(() => root.render(<Reference area="gegenstaende" label="Trank" onOpen={() => {}} usedIn={[]} />));
    rightClick();
    const [only, ...rest] = entries();
    expect(rest).toHaveLength(0);
    expect(only?.textContent).toBe("Nirgends verwendet");
    expect(only?.getAttribute("aria-disabled") === "true" || only?.hasAttribute("data-disabled")).toBe(true);
  });

  it("without_usedIn_there_is_no_context_menu", () => {
    act(() => root.render(<Reference area="welt" label="Route 201" onOpen={() => {}} />));
    rightClick();
    expect(document.querySelector("[role=menu]")).toBeNull();
  });
});
