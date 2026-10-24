// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { fakeChecks, finding } from "../../checks/checksTesting";
import type { ChecksApi } from "../../checks/useChecks";
import type { ProblemRow } from "../problems/problems";
import { ChecksSection, findingsFor } from "./ChecksSection";
import { resetSectionFolds } from "./foldState";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const event = (eventKind: "warp" | "object" | "coord" | "bg", index: number) => ({ kind: "event", eventFile: 411, eventKind, index }) as const;
const warp2 = finding("warp.dest-warp", "error", { args: ["2", "414", "5", "411", "3"], target: event("warp", 2) });
const warp2Never = finding("warp.never-fires", "info", { args: ["2", "8", "4", "0x00", "0x00"], target: event("warp", 2), fingerprint: "00000000000000c3" });
const warp3 = finding("warp.dest-warp", "error", { key: "warp:3", target: event("warp", 3) });
const object0 = finding("event.trainer-blind", "warning", { key: "object:0", args: ["0", "3001", "1"], target: event("object", 0) });
const header = finding("header.wild-dangling", "error", { subject: { kind: "header", index: 7 }, key: "wild_pokemon", args: ["7", "140", "120"], target: { kind: "header", id: 7 } });
const cell = finding("matrix.map-range", "error", { subject: { kind: "matrix", index: 0 }, key: "cell:3,4", args: ["0", "3", "4", "900", "600"], target: { kind: "matrixCell", matrix: 0, x: 3, y: 4 } });
const area = finding("area.tileset-range", "error", { subject: { kind: "areaData", index: 9 }, key: "map_tileset", args: ["9", "map_tileset", "80", "70"], target: { kind: "area", id: 9 } });
const building = finding("map.building-range", "error", { subject: { kind: "map", index: 12 }, key: "building:1", args: ["12", "1", "700", "600"], target: { kind: "matrixCell", matrix: 0, x: 1, y: 1 } });
const compile: ProblemRow = { severity: "error", file: "expanded/scripts/0001.rotom", line: 1, col: 1, message: "x", source: "compile" };
const rows: ProblemRow[] = [compile, warp2, warp2Never, warp3, object0, header, cell, area, building];

describe("findingsFor", () => {
  it("picks_the_findings_of_the_chosen_record", () => {
    expect(findingsFor(rows, { kind: "event", eventFile: 411, eventKind: "warp", index: 2 })).toEqual([warp2, warp2Never]);
    expect(findingsFor(rows, { kind: "event", eventFile: 411, eventKind: "object", index: 0 })).toEqual([object0]);
    expect(findingsFor(rows, { kind: "event", eventFile: 412, eventKind: "warp", index: 2 })).toEqual([]);
    expect(findingsFor(rows, { kind: "header", id: 7 })).toEqual([header]);
    expect(findingsFor(rows, { kind: "matrixCell", matrix: 0, x: 3, y: 4 })).toEqual([cell]);
    expect(findingsFor(rows, { kind: "area", id: 9 })).toEqual([area]);
    expect(findingsFor(rows, { kind: "building", map: 12, index: 1 })).toEqual([building]);
    expect(findingsFor(rows, { kind: "none" })).toEqual([]);
  });
});

describe("ChecksSection", () => {
  let container: HTMLElement;
  let root: Root;
  let checks: ChecksApi;
  beforeEach(() => {
    resetSectionFolds();
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
    checks = fakeChecks();
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    document.body.innerHTML = "";
  });
  const region = () => container.querySelector<HTMLElement>("[role=region]");
  const head = () => container.querySelector<HTMLButtonElement>("h3 button");
  const buttons = (scope: Element) => [...scope.querySelectorAll<HTMLButtonElement>("button")].map((b) => b.textContent);

  it("lists_the_findings_of_the_chosen_event_with_their_sentences", () => {
    act(() => root.render(<ChecksSection findings={findingsFor(rows, { kind: "event", eventFile: 411, eventKind: "warp", index: 2 })} checks={checks} />));
    expect(head()?.textContent).toBe("Prüfungen2");
    const items = [...region()!.querySelectorAll<HTMLElement>("[data-finding]")];
    expect(items.map((i) => [i.dataset.check, i.dataset.state])).toEqual([
      ["warp.dest-warp", "error"],
      ["warp.never-fires", "info"],
    ]);
    expect(items[0]!.textContent).toContain("Die Tür führt zu Warp 5 in Header 0414, aber dessen Eventdatei 0411 hat nur 3 Warps.");
    // The record is open already: no „Öffnen“; an error is never hidden.
    expect(buttons(items[0]!)).toEqual([]);
    expect(buttons(items[1]!)).toEqual(["Ausblenden"]);
    act(() => items[1]!.querySelector("button")!.click());
    expect(checks.ignore).toHaveBeenCalledWith(warp2Never);
  });

  it("offers_beheben_for_a_finding_with_a_fix", async () => {
    const wide = finding("text.too-wide", "warning", { args: ["433", "1", "1", "240", "216"], fixes: [{ id: "wrap", label: "Umbrechen", irreversible: false }] });
    act(() => root.render(<ChecksSection findings={[wide]} checks={checks} />));
    await act(async () => [...region()!.querySelectorAll("button")].find((b) => b.textContent === "Beheben…")!.click());
    expect(document.querySelector("[role=dialog]")).not.toBeNull();
    expect(checks.planFix).toHaveBeenCalledWith(wide, "wrap");
  });

  it("shows_nothing_without_findings", () => {
    act(() => root.render(<ChecksSection findings={[]} checks={checks} />));
    expect(container.innerHTML).toBe("");
  });
});
