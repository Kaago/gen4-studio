// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { readCss, ruleBody } from "../cssSource";
import { Hinge } from "./Hinge";
import type { HingeProps } from "./Hinge";

const env = vi.hoisted(() => ({ tauri: false }));
vi.mock("../../platform/transport", () => ({ invoke: vi.fn(), inTauri: () => env.tauri }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const labels = (n: number) => Array.from({ length: n }, (_, i) => `Schritt ${i + 1}`);

describe("Hinge", () => {
  let container: HTMLElement;
  let root: Root;
  const handlers = () => ({ onDepth: vi.fn(), onProblems: vi.fn(), onUndoTo: vi.fn(), onRedo: vi.fn(), onPlay: vi.fn(), onDragHeight: vi.fn(), onToggleCollapse: vi.fn() });
  const base = (over: Partial<HingeProps> = {}): HingeProps => ({
    crumbs: [{ label: "Zweiblattdorf" }, { label: "Zweiblattdorf", tech: "0414" }],
    depth: "bauen",
    problems: { errors: 0, warnings: 0 },
    history: { undo: [], redo: [] },
    playing: false,
    canPlay: true,
    ...handlers(),
    ...over,
  });
  const show = (props: HingeProps) => act(() => root.render(<Hinge {...props} />));
  const bar = () => container.querySelector<HTMLElement>('[role="toolbar"]')!;
  const button = (name: string | RegExp) =>
    [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => {
      const text = b.getAttribute("aria-label") ?? b.textContent ?? "";
      return typeof name === "string" ? text === name || b.textContent === name : name.test(text);
    });
  /** Opens the menu the way a pointer does and returns its items. */
  const openMenu = (trigger: HTMLElement) => {
    act(() => {
      trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      trigger.click();
    });
    return [...document.querySelectorAll<HTMLElement>("[role=menuitem]")];
  };

  beforeEach(() => {
    env.tauri = false;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });

  it("is_a_toolbar_called_Scharnier_with_the_crumbs_the_last_one_marked", () => {
    show(base());
    expect(bar().getAttribute("aria-label")).toBe("Scharnier");
    const nav = container.querySelector("nav")!;
    expect(nav.textContent).toContain("Zweiblattdorf");
    expect(nav.textContent).toContain("0414");
    expect(nav.querySelectorAll('[aria-current="location"]')).toHaveLength(1);
    expect(nav.querySelector('[aria-current="location"]')?.textContent).toContain("0414");
  });

  it("gives_every_crumb_its_full_text_as_a_title_so_a_cut_off_crumb_stays_readable", () => {
    const onClick = vi.fn();
    show(base({ crumbs: [{ label: "Zweiblattdorf", onClick }, { label: "Außen 0412", tech: "0394" }, { label: "Warp 0 → Header 0412", tech: "Eventdatei 0394" }] }));
    const titles = [...container.querySelectorAll("nav li[title]")].map((li) => li.getAttribute("title"));
    expect(titles).toEqual(["Zweiblattdorf", "Außen 0412 0394", "Warp 0 → Header 0412 Eventdatei 0394"]);
    // A hover over the button inside a crumb shows the crumb's title.
    expect(button("Zweiblattdorf")!.closest("li")!.getAttribute("title")).toBe("Zweiblattdorf");
  });

  it("lets_the_earlier_crumbs_give_way_first_when_the_bar_is_narrow", () => {
    const css = readCss("./Hinge.module.css", import.meta.url);
    // In a narrow hinge the earlier crumbs shrink first (down to a few letters), so the last one, the selection, stays readable.
    const shrinkOf = (selector: string) => Number(/flex:\s*\S+\s+(\d+)/.exec(ruleBody(css, selector))?.[1] ?? NaN);
    expect(shrinkOf(".crumb")).toBeGreaterThan(shrinkOf(".last"));
    expect(ruleBody(css, ".crumb")).toMatch(/min-width:\s*calc\(var\(--g4-raum-/);
    expect(ruleBody(css, ".last")).toMatch(/min-width:\s*calc\(var\(--g4-raum-/);
    expect(ruleBody(css, ".last")).toMatch(/overflow:\s*hidden/);
    expect(ruleBody(css, ".last")).toMatch(/text-overflow:\s*ellipsis/);
    expect(ruleBody(css, ".crumbButton")).toMatch(/text-overflow:\s*ellipsis/);
  });

  it("makes_a_crumb_with_a_link_a_button_and_one_without_plain_text", () => {
    const onClick = vi.fn();
    show(base({ crumbs: [{ label: "Zweiblattdorf", onClick }, { label: "Kein Link" }] }));
    act(() => button("Zweiblattdorf")!.click());
    expect(onClick).toHaveBeenCalledOnce();
    expect(button("Kein Link")).toBeUndefined();
  });

  it("says_what_the_problem_light_counts_and_Alles_stimmig_at_zero", () => {
    show(base({ problems: { errors: 2, warnings: 1 } }));
    expect(button(/2 Fehler, 1 Warnung/)).toBeDefined();
    show(base({ problems: { errors: 0, warnings: 3 } }));
    expect(button(/3 Warnungen/)).toBeDefined();
    show(base());
    expect(button("Alles stimmig")).toBeDefined();
  });

  it("opens_the_problems_when_the_light_is_clicked", () => {
    const props = base({ problems: { errors: 1, warnings: 0 } });
    show(props);
    act(() => button(/1 Fehler/)!.click());
    expect(props.onProblems).toHaveBeenCalledOnce();
  });

  it("switches_the_depth_and_does_nothing_for_the_one_that_is_active", () => {
    const props = base();
    show(props);
    const pressed = () => [...container.querySelectorAll('[aria-pressed="true"]')].map((b) => b.textContent);
    expect(pressed()).toEqual(["Bauen"]);
    act(() => button("Bauen")!.click());
    expect(props.onDepth).not.toHaveBeenCalled();
    act(() => button("Tüfteln")!.click());
    expect(props.onDepth).toHaveBeenCalledExactlyOnceWith("tuefteln");
    show(base({ depth: "tuefteln" }));
    expect(pressed()).toEqual(["Tüfteln"]);
  });

  it("shows_the_number_of_undo_steps_and_lists_the_last_ten_labels_newest_first", () => {
    show(base({ history: { undo: labels(12), redo: [] } }));
    const trigger = button(/Verlauf/)!;
    expect(trigger.textContent).toContain("12");
    const items = openMenu(trigger);
    expect(items.map((i) => i.textContent)).toEqual(["Schritt 12", "Schritt 11", "Schritt 10", "Schritt 9", "Schritt 8", "Schritt 7", "Schritt 6", "Schritt 5", "Schritt 4", "Schritt 3"]);
  });

  it("goes_back_n_steps_when_the_nth_entry_is_clicked", () => {
    const props = base({ history: { undo: labels(5), redo: [] } });
    show(props);
    const items = openMenu(button(/Verlauf/)!);
    act(() => items[2]!.click());
    expect(props.onUndoTo).toHaveBeenCalledExactlyOnceWith(3);
  });

  it("calls_a_step_without_a_label_Aenderung", () => {
    show(base({ history: { undo: ["Tür verschoben", ""], redo: [] } }));
    expect(openMenu(button(/Verlauf/)!).map((i) => i.textContent)).toEqual(["Änderung", "Tür verschoben"]);
  });

  it("offers_nothing_to_go_back_to_while_the_history_is_empty", () => {
    const props = base();
    show(props);
    const items = openMenu(button(/Verlauf/)!);
    expect(items).toHaveLength(1);
    expect(items[0]!.hasAttribute("data-disabled")).toBe(true);
    act(() => items[0]!.click());
    expect(props.onUndoTo).not.toHaveBeenCalled();
  });

  it("disables_redo_while_the_redo_stack_is_empty_and_calls_it_otherwise", () => {
    const props = base();
    show(props);
    expect(button("Wiederholen")!.disabled).toBe(true);
    show({ ...props, history: { undo: ["a"], redo: ["b"] } });
    expect(button("Wiederholen")!.disabled).toBe(false);
    act(() => button("Wiederholen")!.click());
    expect(props.onRedo).toHaveBeenCalledOnce();
  });

  it("plays_with_the_hint_Cmd_R_inside_the_app_and_without_it_in_the_browser", () => {
    const props = base();
    env.tauri = true;
    show(props);
    expect(button(/Hier spielen/)!.textContent).toBe("Hier spielen⌘R");
    act(() => button(/Hier spielen/)!.click());
    expect(props.onPlay).toHaveBeenCalledOnce();
    env.tauri = false;
    show(props);
    expect(button(/Hier spielen/)!.textContent).toBe("Hier spielen");
  });

  it("disables_play_without_a_project_and_says_Baut_while_it_runs", () => {
    show(base({ canPlay: false }));
    expect(button(/Hier spielen/)!.disabled).toBe(true);
    show(base({ playing: true }));
    const playing = button(/Baut/)!;
    expect(playing.textContent).toContain("Baut …");
    expect(playing.disabled).toBe(true);
    expect(button(/Hier spielen/)).toBeUndefined();
  });

  it("has_a_hidden_slot_for_the_wizard_bar_only_where_wizards_show", () => {
    show(base());
    expect(container.querySelector("[data-wizard-bar]")).toBeNull();
    show(base({ wizard: false }));
    const slot = container.querySelector<HTMLElement>("[data-wizard-bar]")!;
    expect(slot.hidden).toBe(true);
    expect(container.querySelector("nav")).not.toBeNull();
  });

  it("shows_the_wizard_slot_instead_of_its_content_and_keeps_the_grip_without_collapsing", () => {
    const props = base({ wizard: true });
    show(props);
    const slot = container.querySelector<HTMLElement>("[data-wizard-bar]")!;
    expect(slot.hidden).toBe(false);
    expect(container.querySelector("nav")).toBeNull();
    expect(button(/Hier spielen/)).toBeUndefined();
    const grip = container.querySelector<HTMLElement>("[data-hinge-grip]")!;
    expect(grip).not.toBeNull();
    // The hand stays open while the wizard works in it.
    act(() => void grip.dispatchEvent(new MouseEvent("dblclick", { bubbles: true })));
    act(() => void grip.dispatchEvent(new MouseEvent("click", { bubbles: true, detail: 0 })));
    expect(props.onToggleCollapse).not.toHaveBeenCalled();
  });

  describe("grip", () => {
    const grip = () => container.querySelector<HTMLElement>("[data-hinge-grip]")!;
    /** A hinge whose bottom edge is at y 600 inside a deck that ends at y 886: the hand is 286 px high. */
    function geometry() {
      const rect = (bottom: number) => ({ bottom, top: bottom - 44, height: 44, left: 0, right: 100, width: 100, x: 0, y: bottom - 44, toJSON: () => ({}) }) as DOMRect;
      vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockImplementation(function (this: HTMLElement) {
        return this.getAttribute("role") === "toolbar" ? rect(600) : rect(886);
      });
    }
    const pointer = (el: Element, type: string, clientY: number) => {
      const event = new MouseEvent(type, { bubbles: true, cancelable: true, button: 0, clientY });
      Object.defineProperty(event, "pointerId", { value: 1 });
      act(() => void el.dispatchEvent(event));
    };

    beforeEach(() => {
      Element.prototype.setPointerCapture = vi.fn();
      Element.prototype.releasePointerCapture = vi.fn();
      Element.prototype.hasPointerCapture = vi.fn(() => true);
    });
    afterEach(() => vi.restoreAllMocks());

    it("sets_the_hand_height_while_dragging_with_the_pointer_captured_by_the_pressed_element", () => {
      const props = base();
      show(props);
      geometry();
      pointer(grip(), "pointerdown", 580);
      // The grip itself holds the pointer: a click goes to the capturing element, and the grip's double click must reach the grip.
      expect(vi.mocked(Element.prototype.setPointerCapture).mock.contexts).toEqual([grip()]);
      pointer(grip(), "pointermove", 480);
      // The pointer moved 100 px up: the hand grows from 286 to 386.
      expect(props.onDragHeight).toHaveBeenLastCalledWith(386);
      pointer(grip(), "pointermove", 600);
      expect(props.onDragHeight).toHaveBeenLastCalledWith(266);
      pointer(grip(), "pointerup", 600);
      vi.mocked(props.onDragHeight).mockClear();
      pointer(grip(), "pointermove", 100);
      expect(props.onDragHeight).not.toHaveBeenCalled();
    });

    it("keeps_the_height_between_200_and_560", () => {
      const props = base();
      show(props);
      geometry();
      pointer(grip(), "pointerdown", 580);
      pointer(grip(), "pointermove", -2000);
      expect(props.onDragHeight).toHaveBeenLastCalledWith(560);
      pointer(grip(), "pointermove", 5000);
      expect(props.onDragHeight).toHaveBeenLastCalledWith(200);
    });

    it("drags_from_an_empty_part_of_the_bar_but_not_from_a_button", () => {
      const props = base();
      show(props);
      geometry();
      pointer(bar(), "pointerdown", 580);
      pointer(bar(), "pointermove", 530);
      expect(props.onDragHeight).toHaveBeenLastCalledWith(336);
      pointer(bar(), "pointerup", 530);
      vi.mocked(props.onDragHeight).mockClear();
      pointer(button("Wiederholen")!, "pointerdown", 580);
      pointer(bar(), "pointermove", 500);
      expect(props.onDragHeight).not.toHaveBeenCalled();
    });

    it("does_not_start_a_drag_from_a_press_in_the_history_menu_which_React_delivers_through_the_portal", () => {
      const props = base({ history: { undo: ["a", "b"], redo: [] } });
      show(props);
      geometry();
      const [item] = openMenu(button(/Verlauf/)!);
      pointer(item!, "pointerdown", 580);
      pointer(bar(), "pointermove", 500);
      expect(props.onDragHeight).not.toHaveBeenCalled();
      expect(Element.prototype.setPointerCapture).not.toHaveBeenCalled();
    });

    it("collapses_and_expands_on_a_double_click_or_the_keyboard", () => {
      const props = base();
      show(props);
      act(() => void grip().dispatchEvent(new MouseEvent("dblclick", { bubbles: true, cancelable: true })));
      expect(props.onToggleCollapse).toHaveBeenCalledOnce();
      // A single mouse click only starts a drag; Enter and Space arrive as a click without a click count.
      act(() => void grip().dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, detail: 1 })));
      expect(props.onToggleCollapse).toHaveBeenCalledOnce();
      act(() => void grip().dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true, detail: 0 })));
      expect(props.onToggleCollapse).toHaveBeenCalledTimes(2);
    });
  });
});
