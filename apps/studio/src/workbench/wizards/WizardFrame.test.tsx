// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { NumberField } from "../../ui/primitives";
import { pressKey, typeInto } from "../../ui/primitives/testUtils";
import { WizardFrame, WizardSlotScope } from "./WizardFrame";
import type { WizardFrameProps } from "./WizardFrame";
import { wizardStatus } from "./wizardStatus";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("WizardFrame", () => {
  let container: HTMLElement;
  let root: Root;
  const onStep = vi.fn();
  const onSubmit = vi.fn();
  const onCancel = vi.fn();
  const onShow = vi.fn();
  const onPlay = vi.fn();
  beforeEach(() => {
    for (const fn of [onStep, onSubmit, onCancel, onShow, onPlay]) fn.mockReset();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    document.querySelectorAll("[data-slot-test]").forEach((e) => e.remove());
  });

  /** A wizard of three steps with its step as state: the first step has a number field (with a draft) and a text area. */
  function Harness(props: Partial<WizardFrameProps> & { start?: number }) {
    const { start = 0, ...rest } = props;
    const [step, setStep] = useState(start);
    const [x, setX] = useState(5);
    const steps = [
      {
        id: "a",
        title: "Quelle",
        canNext: true,
        content: (
          <div>
            <NumberField label="Kachel x" value={x} onCommit={setX} />
            <textarea aria-label="Notiz" />
            <span data-x>{x}</span>
          </div>
        ),
      },
      { id: "b", title: "Anordnung", canNext: true, content: <input aria-label="Name" /> },
      { id: "c", title: "Zusammenfassung", canNext: true, content: <p>Alles bereit.</p> },
    ];
    return (
      <WizardFrame
        title="Raum anlegen"
        icon="room"
        steps={steps}
        step={step}
        onStep={(i) => {
          onStep(i);
          setStep(i);
        }}
        busy={false}
        submitLabel="Anlegen"
        onSubmit={onSubmit}
        onCancel={onCancel}
        {...rest}
      />
    );
  }
  const show = (props: Partial<WizardFrameProps> & { start?: number } = {}) => act(() => root.render(<Harness {...props} />));
  const dialog = () => container.querySelector<HTMLElement>('[role="dialog"]')!;
  const button = (name: string) => [...document.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent === name);
  const field = (name: string) => container.querySelector<HTMLInputElement>(`[aria-label="${name}"]`)!;
  const current = () => container.querySelector('[aria-current="step"]')?.getAttribute("aria-label");

  it("is_a_dialog_named_by_the_title_in_its_bar_and_not_modal", () => {
    show();
    const titleId = dialog().getAttribute("aria-labelledby")!;
    expect(document.getElementById(titleId)?.textContent).toBe("Raum anlegen");
    expect(dialog().hasAttribute("aria-modal")).toBe(false);
    expect(dialog().hasAttribute("data-wizard")).toBe(true);
    expect(current()).toBe("1. Quelle");
  });

  it("puts_the_focus_on_the_first_control_and_gives_it_back_to_the_trigger_when_it_closes", () => {
    const trigger = document.createElement("button");
    document.body.append(trigger);
    trigger.focus();
    show();
    expect(document.activeElement).toBe(field("Kachel x"));
    act(() => root.render(null));
    expect(document.activeElement).toBe(trigger);
    trigger.remove();
  });

  it("lets_enter_on_a_button_do_only_what_the_button_does", () => {
    show({ start: 1 });
    act(() => button("Zurück")!.focus());
    pressKey(button("Zurück")!, "Enter");
    pressKey(button("Abbrechen")!, "Enter");
    expect(onStep).not.toHaveBeenCalled();
    expect(onCancel).not.toHaveBeenCalled();
    expect(current()).toBe("2. Anordnung");
  });

  it("commits_a_typed_draft_on_enter_and_moves_on_with_the_second_enter", () => {
    show();
    act(() => field("Kachel x").focus());
    typeInto(field("Kachel x"), "7");
    pressKey(field("Kachel x"), "Enter");
    expect(container.querySelector("[data-x]")?.textContent).toBe("7");
    expect(onStep).not.toHaveBeenCalled();
    // The field keeps the focus, so the next Enter comes from it.
    expect(document.activeElement).toBe(field("Kachel x"));
    pressKey(field("Kachel x"), "Enter");
    expect(onStep).toHaveBeenCalledExactlyOnceWith(1);
    expect(current()).toBe("2. Anordnung");
  });

  it("takes_a_draft_back_on_escape_and_cancels_on_the_second_escape", () => {
    show();
    act(() => field("Kachel x").focus());
    typeInto(field("Kachel x"), "9");
    pressKey(field("Kachel x"), "Escape");
    expect(field("Kachel x").value).toBe("5");
    expect(onCancel).not.toHaveBeenCalled();
    pressKey(field("Kachel x"), "Escape");
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("never_moves_on_with_enter_in_a_text_area_and_not_with_a_key_from_outside", () => {
    show();
    act(() => field("Notiz").focus());
    pressKey(field("Notiz"), "Enter");
    pressKey(document.body, "Enter");
    pressKey(document.body, "Escape");
    expect(onStep).not.toHaveBeenCalled();
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("submits_with_enter_on_the_last_step_and_goes_back_with_cmd_bracket", () => {
    show({ start: 2 });
    pressKey(dialog(), "Enter");
    expect(onSubmit).toHaveBeenCalledOnce();
    pressKey(dialog(), "[", { metaKey: true });
    expect(onStep).toHaveBeenCalledExactlyOnceWith(1);
  });

  it("blocks_everything_while_busy", () => {
    show({ start: 1, busy: true });
    // The step's content and foot are inert; the step itself is not, so it can hold the focus.
    expect(dialog().hasAttribute("inert")).toBe(false);
    expect(field("Name").closest("[inert]")).not.toBeNull();
    expect(dialog().getAttribute("aria-busy")).toBe("true");
    expect(button("Weiter")?.disabled).toBe(true);
    expect(button("Zurück")?.disabled).toBe(true);
    expect(button("Abbrechen")?.disabled).toBe(true);
    pressKey(dialog(), "Enter");
    pressKey(dialog(), "Escape");
    pressKey(dialog(), "[", { metaKey: true });
    expect(onStep).not.toHaveBeenCalled();
    expect(onCancel).not.toHaveBeenCalled();
  });

  it("takes_the_keyboard_back_when_the_work_ends_without_a_step_change", () => {
    // Applying the patch, a refused submit, a failed "Zeigen": the browser drops the focus of an inert frame and a disabled button.
    show({ start: 1 });
    act(() => field("Name").focus());
    show({ start: 1, busy: true });
    act(() => (document.activeElement as HTMLElement | null)?.blur());
    expect(document.activeElement).toBe(document.body);
    show({ start: 1, busy: false, error: "the room's header cannot also be the header outside" });
    expect(dialog().contains(document.activeElement)).toBe(true);
    pressKey(document.activeElement as HTMLElement, "Escape");
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("the_step_holds_the_focus_while_the_work_runs_and_hands_it_to_a_marked_control_after", () => {
    // The work starts from „Weiter“ in the bar (its button is disabled then, which drops the focus in a browser).
    show({ start: 1 });
    act(() => button("Weiter")!.focus());
    show({ start: 1, busy: true });
    expect(document.activeElement).toBe(dialog());
    // Also when the browser dropped it to the body first.
    act(() => dialog().blur());
    show({ start: 1, busy: false });
    show({ start: 1, busy: true });
    expect(document.activeElement).toBe(dialog());
    // The work ends on the same step with a way on of its own: the control marked `data-wizard-focus`, not the first one.
    function Refused() {
      const steps = [
        { id: "a", title: "Quelle", canNext: true, content: <p>a</p> },
        {
          id: "b",
          title: "Patch",
          canNext: true,
          content: (
            <div>
              <input type="checkbox" aria-label="README" />
              <button type="button" data-wizard-focus="">
                Ersetzen
              </button>
            </div>
          ),
        },
      ];
      return <WizardFrame title="Veröffentlichen" icon="publish" steps={steps} step={1} onStep={onStep} busy={false} submitLabel="Fertig" onSubmit={onSubmit} onCancel={onCancel} />;
    }
    act(() => root.render(<Refused />));
    expect(document.activeElement?.textContent).toBe("Ersetzen");
  });

  it("ignores_a_held_enter", () => {
    // A held key must not run through the steps into the submit that cannot be undone.
    show({ start: 2 });
    pressKey(dialog(), "Enter", { repeat: true });
    expect(onSubmit).not.toHaveBeenCalled();
    act(() => root.render(null));
    show();
    act(() => field("Kachel x").focus());
    pressKey(field("Kachel x"), "Enter", { repeat: true });
    expect(onStep).not.toHaveBeenCalled();
  });

  it("shows_the_summary_with_the_irreversible_badge_on_every_step_and_an_error_instead", () => {
    show({ summary: "Legt Raster „Höhle“ mit einer Kopie von Karte 0003 an.", irreversible: true });
    expect(dialog().textContent).toContain("Legt Raster „Höhle“ mit einer Kopie von Karte 0003 an.");
    expect(dialog().textContent).toContain("Nicht rückgängig");
    show({ summary: "Legt Raster „Höhle“ an.", irreversible: true, error: "map 9999 does not exist" });
    expect(dialog().querySelector('[role="alert"]')?.textContent).toBe("map 9999 does not exist");
    expect(dialog().textContent).not.toContain("Legt Raster");
  });

  it("replaces_the_step_by_the_result_with_show_play_and_close", () => {
    show({ result: <p>Raum angelegt.</p>, onShow, onPlay });
    expect(dialog().textContent).toContain("Raum angelegt.");
    expect(button("Weiter")).toBeUndefined();
    expect(button("Abbrechen")).toBeUndefined();
    act(() => button("Zeigen")!.click());
    act(() => button("Hier spielen")!.click());
    act(() => button("Schließen")!.click());
    expect(onShow).toHaveBeenCalledOnce();
    expect(onPlay).toHaveBeenCalledOnce();
    expect(onCancel).toHaveBeenCalledOnce();
    // A result without anything to show (it stopped half-way) only closes.
    show({ result: <p>Anlegen unterbrochen</p> });
    expect(button("Zeigen")).toBeUndefined();
    expect(button("Hier spielen")).toBeUndefined();
    expect(button("Schließen")).toBeDefined();
  });

  it("result_uses_the_show_label", () => {
    show({ result: <p>Projekt angelegt</p>, onShow, showLabel: "Öffnen" });
    expect(button("Zeigen")).toBeUndefined();
    act(() => button("Öffnen")!.click());
    expect(onShow).toHaveBeenCalledOnce();
    // Without `onPlay` there is no "Hier spielen" beside it.
    expect(button("Hier spielen")).toBeUndefined();
  });

  it("renders_the_bar_and_the_step_into_the_slots_of_the_hinge_and_the_hand", () => {
    const bar = document.createElement("div");
    bar.setAttribute("data-wizard-bar", "");
    bar.setAttribute("data-slot-test", "");
    const hand = document.createElement("div");
    hand.setAttribute("data-wizard-hand", "");
    hand.setAttribute("data-slot-test", "");
    document.body.append(bar, hand);
    show();
    expect(bar.querySelector('ol[aria-label="Schritte"]')).not.toBeNull();
    expect(hand.querySelector('[role="dialog"]')).not.toBeNull();
    expect(container.querySelector('[role="dialog"]')).toBeNull();
    // Enter in the step still moves on, and Escape on a bar button cancels.
    const name = hand.querySelector<HTMLInputElement>('[aria-label="Kachel x"]')!;
    act(() => name.focus());
    pressKey(name, "Enter");
    expect(onStep).toHaveBeenCalledExactlyOnceWith(1);
    pressKey(bar.querySelector("button")!, "Escape");
    expect(onCancel).toHaveBeenCalledOnce();
  });

  it("renders_into_the_slots_of_its_scope_and_not_into_the_first_ones_of_the_document", () => {
    const slots = (where: HTMLElement) => {
      const bar = document.createElement("div");
      bar.setAttribute("data-wizard-bar", "");
      const hand = document.createElement("div");
      hand.setAttribute("data-wizard-hand", "");
      where.append(bar, hand);
      return { bar, hand };
    };
    // The world's deck (built earlier, hidden) comes first in the document; the start's own deck after it.
    const world = document.createElement("div");
    world.setAttribute("data-slot-test", "");
    const start = document.createElement("div");
    start.setAttribute("data-slot-test", "");
    document.body.append(world, start);
    const outer = slots(world);
    const own = slots(start);
    act(() =>
      root.render(
        <WizardSlotScope.Provider value={start}>
          <Harness />
        </WizardSlotScope.Provider>,
      ),
    );
    expect(own.bar.querySelector('ol[aria-label="Schritte"]')).not.toBeNull();
    expect(own.hand.querySelector('[role="dialog"]')).not.toBeNull();
    expect(outer.bar.childElementCount).toBe(0);
    expect(outer.hand.childElementCount).toBe(0);
  });

  it("tells_the_status_line_which_step_shows", () => {
    show({ start: 1 });
    expect(wizardStatus.get()).toEqual({ step: 1, count: 3, title: "Anordnung", done: false });
    act(() => root.render(null));
    expect(wizardStatus.get()).toBeNull();
  });

  it("a_result_title_is_a_last_pill_after_the_steps_and_the_last_step_still_submits", () => {
    const pills = () => [...container.querySelectorAll('[aria-label="Schritte"] li')].map((li) => [li.getAttribute("aria-label"), li.getAttribute("data-state")]);
    show({ start: 2, resultTitle: "Fertig" });
    expect(pills()).toEqual([
      ["1. Quelle", "done"],
      ["2. Anordnung", "done"],
      ["3. Zusammenfassung", "current"],
      ["4. Fertig", "open"],
    ]);
    // The last real step carries the submit button and Enter submits; the status line counts the result too.
    expect(button("Anlegen")).toBeDefined();
    expect(button("Weiter")).toBeUndefined();
    act(() => button("Anlegen")!.click());
    expect(onSubmit).toHaveBeenCalledOnce();
    expect(wizardStatus.get()).toEqual({ step: 2, count: 4, title: "Zusammenfassung", done: false });
    show({ start: 2, resultTitle: "Fertig", result: <p>Veröffentlicht.</p> });
    expect(pills().every(([, state]) => state === "done")).toBe(true);
  });

  it("a_closed_step_has_no_way_back_and_closes_instead_of_cancelling", () => {
    // The steps of the harness, the last one closed (what the step before did is written).
    function Closed() {
      const steps = [
        { id: "a", title: "Quelle", canNext: true, content: <p>a</p> },
        { id: "b", title: "Prüfen", canNext: true, closed: true, content: <p>b</p> },
      ];
      return <WizardFrame title="Veröffentlichen" icon="publish" steps={steps} step={1} onStep={onStep} busy={false} submitLabel="Fertig" onSubmit={onSubmit} onCancel={onCancel} />;
    }
    act(() => root.render(<Closed />));
    // The status line learns that the step is closed (its hints name no way back).
    expect(wizardStatus.get()).toEqual({ step: 1, count: 2, title: "Prüfen", done: false, closed: true });
    expect(button("Zurück")).toBeUndefined();
    expect(button("Abbrechen")).toBeUndefined();
    act(() => button("Schließen")!.click());
    expect(onCancel).toHaveBeenCalledOnce();
    act(() => void dialog().dispatchEvent(new KeyboardEvent("keydown", { key: "[", metaKey: true, bubbles: true, cancelable: true })));
    expect(onStep).not.toHaveBeenCalled();
    // Without it: „Zurück“ and „Abbrechen“ as before.
    show({ start: 1 });
    expect(button("Zurück")).toBeDefined();
    expect(button("Abbrechen")).toBeDefined();
  });
});
