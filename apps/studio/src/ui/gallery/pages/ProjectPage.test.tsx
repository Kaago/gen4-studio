// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ProjectPage } from "./ProjectPage";

vi.mock("@tauri-apps/api/webview", () => ({ getCurrentWebview: () => ({ onDragDropEvent: async () => () => {} }) }));

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("ProjectPage", () => {
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

  it("shows_the_main_action_the_drop_frame_and_the_wizard_with_example_data", async () => {
    await act(async () => root.render(<ProjectPage />));
    const primaries = [...container.querySelectorAll<HTMLButtonElement>('[data-start-actions] button[data-kind="primary"]')].map((b) => b.textContent);
    // Each of the three starts has the one main action.
    expect(primaries).toEqual(["Neues Spiel aus ROM…", "Neues Spiel aus ROM…", "Neues Spiel aus ROM…"]);
    expect(container.textContent).toContain("ROM hier ablegen");
    // The example ROM is read and released: the wizard moves on to the template, in the start's own deck.
    expect(container.querySelector("[data-wizard]")).toBeNull();
    const example = [...container.querySelectorAll("button")].find((b) => b.textContent === "Mit abgelegter Beispiel-ROM öffnen")!;
    await act(async () => example.click());
    await act(() => new Promise<void>((resolve) => setTimeout(resolve, 350)));
    const deck = container.querySelector("[data-start-deck]")!;
    expect(deck.querySelector('[aria-current="step"]')?.getAttribute("aria-label")).toBe("2. Vorlage");
    expect(deck.querySelector("[data-start-schau]")?.textContent).toContain("POKEMON PL");
  });
});
