// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { useEffect, useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CheckProblemDto, WarpDto } from "../api";
import { fakeChecks, finding } from "../checks/checksTesting";
import { Inspector } from "./Inspector";
import type { InspectorModel } from "./Inspector";

// The editors as stand-ins that count their mounts and keep a value in state: what a remount would lose (the loaded record, focus, a picker).
const editors = vi.hoisted(() => ({ mounts: 0 }));
vi.mock("./inspector/EventEditor", () => ({
  EventEditor: () => {
    const [kept] = useState(() => ++editors.mounts);
    useEffect(() => {}, []);
    return <div data-editor={kept}>Warp-Editor</div>;
  },
}));

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
    act(() => root.render(<Inspector model={model} headerEditor={undefined as never} matrixEditor={undefined as never} areaEditor={undefined as never} buildingEditor={undefined as never} eventEditor={undefined as never} depth="bauen" />));
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

describe("Inspector with findings of the checks", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    editors.mounts = 0;
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
  });
  const model: InspectorModel = { kind: "event", eventFile: 394, eventKind: "warp", index: 0 };
  const dest = finding("warp.dest-warp", "error", { key: "warp:0", args: ["0", "414", "99", "393", "2"], target: { kind: "event", eventFile: 394, eventKind: "warp", index: 0 } });
  const show = (findings: CheckProblemDto[]) =>
    act(() =>
      root.render(
        <Inspector
          model={model}
          headerEditor={undefined as never}
          matrixEditor={undefined as never}
          areaEditor={undefined as never}
          buildingEditor={undefined as never}
          eventEditor={undefined as never}
          depth="bauen"
          findings={findings}
          checks={fakeChecks()}
        />,
      ),
    );

  it("keeps_its_editor_when_the_records_findings_come_and_go", () => {
    show([]);
    const editor = container.querySelector("[data-editor]");
    expect(editor?.getAttribute("data-editor")).toBe("1");
    expect(container.textContent).not.toContain("Prüfungen");
    // A bad value makes a finding: the section comes, the editor stays the same element with the same state.
    show([dest]);
    expect(container.textContent).toContain("Prüfungen");
    expect(container.querySelector("[data-editor]")).toBe(editor);
    // The fix makes it go: still the same editor.
    show([]);
    expect(container.textContent).not.toContain("Prüfungen");
    expect(container.querySelector("[data-editor]")).toBe(editor);
    expect(editors.mounts).toBe(1);
  });
});
