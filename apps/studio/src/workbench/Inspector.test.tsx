// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { useEffect, useState } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { CheckProblemDto, WarpDto } from "../api";
import { fakeChecks, finding } from "../checks/checksTesting";
import type { Depth } from "../ui/frame/shellState";
import { visibleWords } from "../ui/visibleText";
import { useInspectorState } from "../app/useInspectorState";
import type { InspectorState } from "../app/useInspectorState";
import { Inspector } from "./Inspector";
import type { InspectorModel, SceneHover } from "./Inspector";

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

describe("Inspector and the scene's hover", () => {
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
  const tile: SceneHover = { kind: "tile", map: 187, x: 8, y: 4, absX: 40, absZ: 36, info: { kind: 0x5e, collision: 1, name: "WARP_STAIRS_EAST", blocked: false, class: null } as never };
  const marker: SceneHover = { kind: "eventHover", eventFile: 5, eventKind: "warp", index: 1, data: warp, destName: "T01R0201" };
  const text = () => container.textContent ?? "";
  let pins: InspectorState | null = null;
  /** The inspector as App wires it: the state of selection and hover, the inspector given what the state says it shows. */
  function Wired({ depth }: { depth: Depth }) {
    pins = useInspectorState();
    return (
      <Inspector model={pins.pinned} headerEditor={undefined as never} matrixEditor={undefined as never} areaEditor={undefined as never} buildingEditor={undefined as never} eventEditor={undefined as never} depth={depth} />
    );
  }
  const show = (depth: Depth = "bauen") => act(() => root.render(<Wired depth={depth} />));

  it("says_what_to_do_when_nothing_is_selected", () => {
    show();
    expect(text()).toContain("Nichts ausgewählt");
    expect(text()).not.toContain("Kachel");
    expect(container.querySelector("h2")).toBeNull();
  });

  it("shows_the_selection_while_the_pointer_hovers_a_tile_or_marker", () => {
    show();
    act(() => pins!.setPinned({ kind: "event", eventFile: 394, eventKind: "object", index: 0 }));
    const editor = container.querySelector("[data-editor]");
    expect(editor).not.toBeNull();
    for (const hover of [tile, marker, null]) {
      act(() => pins!.setHover(hover));
      // The editor of the selection stays, the same element: no hover view takes its place.
      expect(container.querySelector("[data-editor]")).toBe(editor);
      expect(text()).not.toMatch(/Kachel|Nur Ansicht|Warp 1 →/);
    }
    expect(editors.mounts).toBe(1);
    // Nothing selected: the pointer over a tile or a marker does not fill the inspector either.
    act(() => pins!.setPinned({ kind: "none" }));
    for (const hover of [tile, marker]) {
      act(() => pins!.setHover(hover));
      expect(text()).toContain("Nichts ausgewählt");
      expect(container.querySelector("h2")).toBeNull();
      expect(pins!.hover).toBe(hover);
    }
    // The inspector has no view of a hover at all: handed one, it says nothing is selected (the status line names the hover).
    for (const hover of [tile, marker]) {
      act(() =>
        root.render(<Inspector model={hover} headerEditor={undefined as never} matrixEditor={undefined as never} areaEditor={undefined as never} buildingEditor={undefined as never} eventEditor={undefined as never} depth="bauen" />),
      );
      expect(text()).toContain("Nichts ausgewählt");
      expect(text()).not.toMatch(/Kachel|Nur Ansicht|Warp 1 →/);
    }
  });

  it("says_no_raster_matrix_or_header_in_building", () => {
    show();
    act(() => pins!.setHover(tile));
    expect(visibleWords(container)).not.toMatch(/Raster|Matrix|Header/);
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
