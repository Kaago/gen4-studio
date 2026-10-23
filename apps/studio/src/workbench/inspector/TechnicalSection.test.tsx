// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { InspectorModel } from "../Inspector";
import { TechnicalSection } from "./TechnicalSection";
import type { TechnicalSource } from "./technical";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const model: InspectorModel = { kind: "event", eventFile: 411, eventKind: "object", index: 3 };
const source: TechnicalSource = {
  kind: "event",
  eventKind: "object",
  event: { index: 3, x: 3, y: 2, z: 5, local_id: 1, graphics_id: 90, movement_type: 2, dir: 1, script: 2000 } as never,
  family: "Pt",
};

describe("TechnicalSection", () => {
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
  const head = () => container.querySelector<HTMLButtonElement>("button[aria-expanded]")!;

  it("is_closed_by_default_under_the_head_Technisch", () => {
    act(() => root.render(<TechnicalSection model={model} source={source} />));
    expect(head().textContent).toBe("Technisch");
    expect(head().getAttribute("aria-expanded")).toBe("false");
    expect(container.querySelector("table")).toBeNull();
  });

  it("stays_open_when_the_editor_is_replaced_by_another_one_and_back_as_a_hover_does", () => {
    act(() => root.render(<TechnicalSection model={model} source={source} />));
    act(() => head().click());
    // The hover model replaces the editor: its sections unmount.
    act(() => root.render(null));
    act(() => root.render(<TechnicalSection model={{ kind: "event", eventFile: 411, eventKind: "object", index: 4 }} source={source} />));
    expect(head().getAttribute("aria-expanded")).toBe("true");
    act(() => head().click());
    act(() => root.render(null));
    act(() => root.render(<TechnicalSection model={model} source={source} />));
    expect(head().getAttribute("aria-expanded")).toBe("false");
  });

  it("shows_the_asset_line_the_file_and_every_field_with_decimal_and_hex_once_opened", () => {
    act(() => root.render(<TechnicalSection model={model} source={source} />));
    act(() => head().click());
    expect(head().getAttribute("aria-expanded")).toBe("true");
    const text = container.textContent ?? "";
    expect(text).toContain("Eventdatei 0411, Index 3");
    expect(text).toContain("unpacked/eventFiles/0411");
    const script = [...container.querySelectorAll("tbody tr")].find((r) => r.querySelector("th")?.textContent === "script")!;
    expect([...script.querySelectorAll("td")].map((c) => c.textContent)).toEqual(["2000", "0x7D0"]);
    expect([...container.querySelectorAll("tbody th")].map((c) => c.textContent)).toContain("graphics_id");
  });

  it("shows_no_file_line_for_a_record_without_a_file", () => {
    const header: InspectorModel = { kind: "header", id: 1 };
    act(() => root.render(<TechnicalSection model={header} source={{ kind: "header", header: { Pt: { matrix_id: 1 } } as never }} />));
    act(() => head().click());
    expect(container.textContent).toContain("Header 0001, Eintrag der Header-Tabelle");
    expect(container.textContent).not.toContain("Datei");
  });
});
