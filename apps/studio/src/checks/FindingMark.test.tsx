// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { ProblemRow } from "../workbench/problems/problems";
import { finding } from "./checksTesting";
import { FindingMark, headerMarks, marksOf } from "./FindingMark";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("marksOf", () => {
  it("gives_each_subject_the_worst_of_its_errors_and_warnings_and_leaves_infos_out", () => {
    const rows: ProblemRow[] = [
      { severity: "error", file: "trainers/0003", line: 1, col: 1, message: "x", source: "trainer" },
      finding("trainer.no-moves", "warning", { subject: { kind: "trainer", index: 3 }, key: "slot:0:moves" }),
      finding("trainer.no-party", "error", { subject: { kind: "trainer", index: 4 }, key: "party" }),
      finding("trainer.no-moves", "warning", { subject: { kind: "trainer", index: 4 }, key: "slot:1:moves" }),
      finding("encounter.min-max", "info", { subject: { kind: "encounter", index: 5 }, key: "slot:surf:0" }),
      finding("encounter.level-zero", "warning", { subject: { kind: "encounter", index: 6 }, key: "slot:grass:0" }),
    ];
    expect([...marksOf(rows, "trainer")]).toEqual([
      [3, "warning"],
      [4, "error"],
    ]);
    expect([...marksOf(rows, "encounter")]).toEqual([[6, "warning"]]);
  });
});

describe("headerMarks", () => {
  it("marks_a_header_by_its_own_findings_and_those_of_its_event_file", () => {
    const h = (id: number, event_file_id: number) => ({ id, event_file_id }) as never;
    const rows: ProblemRow[] = [
      finding("warp.dest-warp", "error", { subject: { kind: "eventFile", index: 411 }, key: "warp:2" }),
      finding("header.level-target", "warning", { subject: { kind: "header", index: 9 }, key: "level:3" }),
      finding("warp.never-fires", "info", { subject: { kind: "eventFile", index: 500 }, key: "warp:0" }),
    ];
    expect([...headerMarks(rows, [h(7, 411), h(8, 411), h(9, 12), h(10, 500)])]).toEqual([
      [7, "error"],
      [8, "error"],
      [9, "warning"],
    ]);
  });
});

describe("FindingMark", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    container = document.createElement("div");
    root = createRoot(container);
  });
  afterEach(() => act(() => root.unmount()));

  it("is_a_symbol_with_a_name", () => {
    act(() => root.render(<FindingMark severity="error" />));
    const mark = container.querySelector<HTMLElement>("[role=img]")!;
    expect(mark.getAttribute("aria-label")).toBe("Fehler gefunden");
    expect(mark.dataset.findingMark).toBe("error");
    expect(mark.querySelector("svg")).not.toBeNull();
    act(() => root.render(<FindingMark severity="warning" />));
    expect(container.querySelector("[role=img]")?.getAttribute("aria-label")).toBe("Warnung gefunden");
  });
});
