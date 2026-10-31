// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { wizardStatus } from "../workbench/wizards/wizardStatus";
import { WizardHints } from "./WizardHints";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("WizardHints", () => {
  let container: HTMLElement;
  let root: Root;

  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    act(() => root.render(<WizardHints />));
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    act(() => wizardStatus.set(null));
  });

  it("a_closed_step_names_no_way_back_and_closes", () => {
    act(() => wizardStatus.set({ step: 1, count: 4, title: "Patch", done: false }));
    expect(container.textContent).toBe("↵ Weiter, ⌘[ Zurück, Esc Abbrechen");
    // „Prüfen“ after the release: the steps before are written.
    act(() => wizardStatus.set({ step: 2, count: 4, title: "Prüfen", done: false, closed: true }));
    expect(container.textContent).toBe("↵ Weiter, Esc Schließen");
    act(() => wizardStatus.set({ step: 2, count: 4, title: "Prüfen", done: true, closed: true }));
    expect(container.textContent).toBe("Esc Schließen");
  });
});
