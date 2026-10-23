// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it } from "vitest";
import type { WarpDto } from "../api";
import { Inspector } from "./Inspector";
import type { InspectorModel } from "./Inspector";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const warp: WarpDto = { index: 1, x: 8, z: 4, dest_header_id: 414, dest_warp_id: 0, height: 0 };

describe("Inspector read-only views", () => {
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
  // The hover views and the empty state read no context.
  const show = (model: InspectorModel) =>
    act(() => root.render(<Inspector model={model} headerEditor={undefined as never} matrixEditor={undefined as never} areaEditor={undefined as never} buildingEditor={undefined as never} eventEditor={undefined as never} />));
  const text = () => container.textContent ?? "";

  it("says_what_to_do_when_nothing_is_selected", () => {
    show({ kind: "none" });
    expect(text()).toContain("Nichts ausgewählt");
    expect(container.querySelector("h2")).toBeNull();
  });

  it("shows_the_hovered_event_with_the_view_only_badge_and_its_details", () => {
    show({ kind: "eventHover", eventFile: 5, eventKind: "warp", index: 1, data: warp, destName: "T01R0201" });
    expect(container.querySelector("h2")?.textContent).toBe("Warp 1 → Header 0414 (T01R0201)");
    expect(text()).toContain("Tür 2");
    expect(text()).toContain("Nur Ansicht");
    expect(text()).toContain("Position x 8, z 4");
    expect(container.querySelector("input, [role=combobox], [role=switch]")).toBeNull();
  });

  it("shows_the_hovered_tile_with_the_view_only_badge", () => {
    show({ kind: "tile", map: 187, x: 8, y: 4, absX: 40, absZ: 36, info: { kind: 0x5e, collision: 1, name: "WARP_STAIRS_EAST", blocked: false, class: null } as never });
    expect(container.querySelector("h2")?.textContent).toBe("Kachel x 8, y 4");
    expect(text()).toContain("Karte 0187");
    expect(text()).toContain("Nur Ansicht");
    expect(text()).toContain("Typ 0x5E WARP_STAIRS_EAST");
    expect(text()).toContain("Matrix-Position x 40, z 36 · Kollision 0x01");
  });
});
