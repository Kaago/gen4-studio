// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ChecksFeed } from "../checks/useChecks";
import type { Depth } from "../ui/frame/shellState";
import { visibleWords } from "../ui/visibleText";
import { ProblemsStore } from "../workbench/problems/problems";
import { HingeHost } from "./HingeHost";
import { HistoryFeed } from "./useHistory";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("HingeHost", () => {
  let container: HTMLElement;
  let root: Root;
  let checks: ChecksFeed;
  let history: HistoryFeed;
  const show = (depth: Depth) => {
    const noop = vi.fn();
    act(() =>
      root.render(
        <HingeHost
          problems={new ProblemsStore()}
          history={history}
          checks={checks}
          crumbs={[]}
          depth={depth}
          onDepth={noop}
          onProblems={noop}
          onUndoTo={noop}
          onRedo={noop}
          onPlay={noop}
          playing={false}
          canPlay
        />,
      ),
    );
  };
  beforeEach(() => {
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    checks = new ChecksFeed();
    history = new HistoryFeed();
    show("bauen");
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    document.body.innerHTML = "";
  });
  /** Opens the history menu the way a pointer does and returns the words of its items. */
  const historyMenu = () => {
    const trigger = [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => /Verlauf/.test(b.getAttribute("aria-label") ?? ""))!;
    act(() => {
      trigger.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, cancelable: true, button: 0 }));
      trigger.dispatchEvent(new MouseEvent("mouseup", { bubbles: true, cancelable: true, button: 0 }));
      trigger.click();
    });
    return [...document.querySelectorAll<HTMLElement>("[role=menuitem]")].map((item) => visibleWords(item));
  };
  const light = () => container.querySelector<HTMLButtonElement>("[data-light]")!;

  it("says_Nicht_geprueft_until_a_report_arrived_and_Alles_stimmig_only_after_one", () => {
    expect(light().getAttribute("aria-label")).toBe("Nicht geprüft");
    expect(light().getAttribute("data-light")).toBe("unchecked");
    act(() => checks.update({ running: true }));
    expect(light().getAttribute("aria-label")).toBe("Prüft …");
    // The first run ended without a report (it failed, or answered with none).
    act(() => checks.update({ running: false }));
    expect(light().getAttribute("aria-label")).toBe("Nicht geprüft");
    act(() => checks.update({ reported: true }));
    expect(light().getAttribute("aria-label")).toBe("Alles stimmig");
    expect(light().getAttribute("data-light")).toBe("sound");
  });

  it("says_no_raster_matrix_or_header_in_building", () => {
    act(() => history.set({ undo: ["edit header", "edit matrix cell", "resize matrix", "assign header 0414 to matrix 0003"], redo: [] }));
    const items = historyMenu();
    expect(items).toEqual(["Ort 0414 Weltkarte 0003 zuweisen", "Größe der Weltkarte geändert", "Zelle geändert", "Ort geändert"]);
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
  });

  it("passes_play_mode_through_to_the_hinge", () => {
    const onStop = vi.fn();
    const noop = vi.fn();
    act(() =>
      root.render(
        <HingeHost
          problems={new ProblemsStore()}
          history={history}
          checks={checks}
          crumbs={[]}
          depth="bauen"
          onDepth={noop}
          onProblems={noop}
          onUndoTo={noop}
          onRedo={noop}
          onPlay={noop}
          playing={false}
          canPlay
          live={{ state: "waiting_for_save", text: "Wartet auf den Spielstand", breakpoints: 0, onResume: noop, onReload: noop, onStop }}
        />,
      ),
    );
    expect(container.querySelector("[data-hinge-mode]")?.getAttribute("data-hinge-mode")).toBe("play");
    expect(container.querySelector("[data-live-state]")?.textContent).toBe("Wartet auf den Spielstand");
    act(() => container.querySelector<HTMLButtonElement>("button[data-live-stop]")!.click());
    expect(onStop).toHaveBeenCalledOnce();
  });

  it("keeps_the_technical_words_in_tinkering", () => {
    show("tuefteln");
    act(() => history.set({ undo: ["edit header", "edit matrix cell"], redo: [] }));
    expect(historyMenu()).toEqual(["Rasterfeld geändert", "Header geändert"]);
  });
});
