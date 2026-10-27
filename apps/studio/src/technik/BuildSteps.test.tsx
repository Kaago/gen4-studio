// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { BuildStep } from "../api";
import { visibleWords } from "../ui/visibleText";
import { BuildSteps } from "./BuildSteps";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const step = (id: string, over: Partial<BuildStep> = {}): BuildStep => ({ id, status: "skipped", ms: 0, done: 0, total: 0, cached: 0, reason: null, ...over });

/** HeartGold DE after a map change: the maps packed, everything else skipped or from the cache. */
const AFTER_A_MAP: BuildStep[] = [
  step("chatot", { ms: 4 }),
  step("pack:maps", { status: "incremental", ms: 21, done: 1, total: 540 }),
  step("pack:textArchives", { ms: 2 }),
  step("pack:pokeHeight", { ms: 1 }),
  step("watch:dynamicHeaders", { ms: 1 }),
  step("blz:arm9", { status: "incremental", ms: 3, total: 1, cached: 1 }),
  step("blz:overlays", { status: "incremental", ms: 12, total: 127, cached: 127 }),
  step("rom", { status: "full", ms: 50 }),
  step("crc", { status: "full", ms: 1 }),
];

describe("BuildSteps", () => {
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

  const rows = () => [...container.querySelectorAll<HTMLElement>("tr[data-step]")];
  const cells = (row: HTMLElement) => [...row.querySelectorAll("td")].map((td) => td.textContent);
  const fold = () => [...container.querySelectorAll<HTMLButtonElement>("button")].find((b) => b.textContent?.startsWith("Übersprungen"));

  it("shows_ran_steps_as_rows_and_folds_the_skipped", () => {
    act(() => root.render(<BuildSteps steps={AFTER_A_MAP} depth="bauen" />));
    const table = container.querySelector("table");
    expect([...(table?.querySelectorAll("th") ?? [])].map((th) => th.textContent)).toEqual(["Schritt", "Ergebnis", "Dauer"]);
    expect(rows().map((r) => [r.dataset.step, r.dataset.status])).toEqual([
      ["pack:maps", "incremental"],
      ["blz:arm9", "incremental"],
      ["blz:overlays", "incremental"],
      ["rom", "full"],
      ["crc", "full"],
    ]);
    expect(rows().map(cells)).toEqual([
      ["Karten packen", "1 Member eingesetzt", "0,02 s"],
      ["Spielcode (ARM9)", "1 aus dem Cache", "< 0,01 s"],
      ["Overlays", "127 aus dem Cache", "0,01 s"],
      ["ROM schreiben", "geschrieben", "0,05 s"],
      ["Prüfsummen", "aus der Basis übernommen", "< 0,01 s"],
    ]);
    // The skipped steps are folded under their count.
    expect(fold()?.textContent).toBe("Übersprungen (3)");
    expect(fold()?.getAttribute("aria-expanded")).toBe("false");
    act(() => fold()!.click());
    expect(rows().filter((r) => r.dataset.status === "skipped").map(cells)).toEqual([
      ["Texte", "übersprungen", "< 0,01 s"],
      ["Texte packen", "übersprungen", "< 0,01 s"],
      ["Spieldaten packen", "übersprungen", "< 0,01 s"],
    ]);
    // Bauen names no step by its identifier.
    expect(visibleWords(container)).not.toMatch(/pack:|blz:|pokeHeight|chatot/);
  });

  it("hides_watch_steps", () => {
    act(() => root.render(<BuildSteps steps={AFTER_A_MAP} depth="tuefteln" />));
    act(() => fold()!.click());
    expect(rows().map((r) => r.dataset.step)).not.toContain("watch:dynamicHeaders");
    expect(fold()?.textContent).toBe("Übersprungen (3)");
    // Tüfteln keeps the folder name and shows the step's identifier beside the words.
    expect(rows().find((r) => r.dataset.step === "pack:pokeHeight")?.querySelector("td")?.textContent).toBe("pokeHeight packenpack:pokeHeight");
    expect(container.textContent).not.toContain("dynamicHeaders");
  });

  it("without_skipped_steps_has_no_fold", () => {
    act(() => root.render(<BuildSteps steps={[step("rom", { status: "full", reason: "no-state", ms: 330 }), step("watch:x")]} depth="bauen" />));
    expect(rows().map(cells)).toEqual([["ROM schreiben", "geschrieben (erster Build, voll)", "0,33 s"]]);
    expect(fold()).toBeUndefined();
  });
});
