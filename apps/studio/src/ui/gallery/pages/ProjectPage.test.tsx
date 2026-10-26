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
    // Each of the four starts has the one main action.
    expect(primaries).toEqual(["Neues Spiel aus ROM…", "Neues Spiel aus ROM…", "Neues Spiel aus ROM…", "Neues Spiel aus ROM…"]);
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

  it("shows_the_project_card_in_each_state_with_pictures_it_draws_itself", async () => {
    // jsdom has no canvas: a stand-in that takes the drawing and answers a PNG per picture.
    let drawn = 0;
    const fills: string[] = [];
    const ctx = { fillRect: vi.fn(), globalAlpha: 1, set fillStyle(v: string) { fills.push(v); } };
    const getContext = vi.spyOn(HTMLCanvasElement.prototype, "getContext").mockImplementation((() => ctx) as never);
    const toDataURL = vi.spyOn(HTMLCanvasElement.prototype, "toDataURL").mockImplementation(() => `data:image/png;base64,UElD${drawn++}`);
    try {
      await act(async () => root.render(<ProjectPage />));
    } finally {
      getContext.mockRestore();
      toDataURL.mockRestore();
    }
    // Only scene colours, as CSS rgb().
    expect(fills.length).toBeGreaterThan(0);
    expect(fills.every((f) => /^rgb\(\d+ \d+ \d+\)$/.test(f))).toBe(true);
    const section = [...container.querySelectorAll("section")].find((s) => s.querySelector("h2")?.textContent === "Projektkarte")!;
    const card = (name: string) => section.querySelector(`[data-recent="/Projekte/${name}"]`)!.closest("li")!;
    const src = (name: string) => card(name).querySelector("img")?.getAttribute("src") ?? null;
    expect(src("platin-hack")).toBe("data:image/png;base64,UElD0");
    expect(card("platin-hack").textContent).toContain("Zuletzt");
    expect(src("johto-neu")).toBe("data:image/png;base64,UElD1");
    expect(src("erster-versuch")).toBeNull();
    expect(card("erster-versuch").querySelector("[data-preview-name]")?.textContent).toBe("erster-versuch");
    expect(card("alte-kopie").hasAttribute("data-missing")).toBe(true);
    expect(card("alte-kopie").textContent).toContain("nicht gefunden");
    expect(src("alte-kopie")).toBeNull();
  });
});
