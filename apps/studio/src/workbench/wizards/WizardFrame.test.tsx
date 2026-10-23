// @vitest-environment jsdom
import { act, useState } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { pressKey, typeInto } from "../../ui/primitives/testUtils";
import { DraftInput } from "../DraftInput";
import { WizardFrame } from "./WizardFrame";
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
    const [x, setX] = useState("5");
    const steps = [
      {
        id: "a",
        title: "Quelle",
        canNext: true,
        content: (
          <div>
            <DraftInput label="Kachel x" type="number" value={x} onCommit={setX} />
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
    expect(dialog().hasAttribute("inert")).toBe(true);
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

  it("tells_the_status_line_which_step_shows", () => {
    show({ start: 1 });
    expect(wizardStatus.get()).toEqual({ step: 1, count: 3, title: "Anordnung", done: false });
    act(() => root.render(null));
    expect(wizardStatus.get()).toBeNull();
  });
});
