// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { readCss, ruleBody } from "../cssSource";
import { BeforeAfter } from "./BeforeAfter";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("BeforeAfter", () => {
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
  const column = (name: string) => container.querySelector<HTMLElement>(`[role=group][aria-label="${name}"]`)!;
  const lines = (name: string) => [...column(name).querySelectorAll<HTMLElement>("[data-line]")].map((l) => [l.textContent, l.hasAttribute("data-changed")]);

  it("shows_both_sides_in_two_columns_and_marks_the_lines_that_changed", () => {
    act(() => root.render(<BeforeAfter label="Textarchiv 0433" before={"Hallo,\ndas ist eine viel zu lange Zeile\nTschüss"} after={"Hallo,\ndas ist eine viel zu\nlange Zeile\nTschüss"} />));
    expect(container.querySelector("[role=group][aria-label='Textarchiv 0433']")).not.toBeNull();
    expect(column("Vorher").textContent).toContain("Vorher");
    expect(lines("Vorher")).toEqual([
      ["Hallo,", false],
      ["das ist eine viel zu lange Zeile", true],
      ["Tschüss", false],
    ]);
    expect(lines("Nachher")).toEqual([
      ["Hallo,", false],
      ["das ist eine viel zu", true],
      ["lange Zeile", true],
      ["Tschüss", false],
    ]);
    // Never colour alone: what went is a deletion, what came an insertion, each with its sign.
    expect(column("Vorher").querySelectorAll("del")).toHaveLength(1);
    expect(column("Nachher").querySelectorAll("ins")).toHaveLength(2);
  });

  it("says_leer_for_an_empty_side", () => {
    act(() => root.render(<BeforeAfter before="" after={"script script_7 #7:\n\tEnd"} />));
    expect(column("Vorher").querySelectorAll("[data-line]")).toHaveLength(0);
    expect(column("Vorher").textContent).toContain("Leer");
    expect(lines("Nachher").every(([, changed]) => changed)).toBe(true);
    expect(container.querySelector("[role=group][aria-label='Vorher und Nachher']")).not.toBeNull();
  });

  it("highlights_changed_lines_in_the_selection_surface", () => {
    const css = readCss("./BeforeAfter.module.css", import.meta.url);
    expect(ruleBody(css, ".changed")).toMatch(/background:\s*var\(--g4-auswahl-flaeche\)/);
  });
});
