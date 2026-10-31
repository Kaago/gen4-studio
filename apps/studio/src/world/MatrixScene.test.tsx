// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import type { Root } from "react-dom/client";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

// The scene in jsdom: no WebGL, so the renderer draws nothing; everything else (scene, camera, loaders) is the real one.
vi.mock("three", async (importOriginal) => {
  const three = await importOriginal<typeof import("three")>();
  class FakeRenderer {
    domElement = document.createElement("canvas");
    setPixelRatio() {}
    setClearColor() {}
    setSize() {}
    render() {}
    dispose() {}
  }
  return { ...three, WebGLRenderer: FakeRenderer };
});
vi.mock("../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../api")>()), tileBehaviors: vi.fn(async () => []) }));

import type { DraftDto, MatrixDto } from "../api";
import { draftCount } from "../api";
import { createEditBus } from "../bus/editBus";
import MatrixScene from "./MatrixScene";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

class FakeObserver {
  observe() {}
  disconnect() {}
}

/** Matrix 4 without maps: the load builds the scene without fetching one. */
const matrix: MatrixDto = { index: 4, name: "m", width: 2, height: 2, maps: [null, null, null, null], headers: null, altitudes: null };

const draft: DraftDto = {
  cells: [
    { matrix: 4, x: 1, y: 0, source_map: 9 },
    { matrix: null, x: 0, y: 0, source_map: 9 },
  ],
  tiles: [{ matrix: 4, x: 40, z: 5 }],
  events: [
    { kind: "warp", matrix: 4, x: 40, z: 5, look: null, target: { header: 3, warp: 0 } },
    { kind: "warp", matrix: null, x: 3, z: 6, look: null, target: { header: 1, warp: 2 } },
  ],
  messages: [],
  notes: [],
  irreversible: [],
};

const noop = () => {};

describe("MatrixScene", () => {
  let container: HTMLElement;
  let root: Root;
  beforeEach(() => {
    vi.stubGlobal("ResizeObserver", FakeObserver);
    container = document.createElement("div");
    document.body.append(container);
    root = createRoot(container);
  });
  afterEach(() => {
    act(() => root.unmount());
    container.remove();
    vi.unstubAllGlobals();
  });

  const bus = createEditBus();
  const show = (d: DraftDto | null) =>
    act(() =>
      root.render(
        <MatrixScene
          matrix={matrix}
          focus={null}
          topDown={false}
          areaDataId={0}
          bus={bus}
          showPermissions={false}
          eventFileId={null}
          showEvents
          destName={() => undefined}
          paintEnabled
          tool="select"
          onTileClick={noop}
          onBuildings={noop}
          history={{ current: 0 }}
          onEdited={noop}
          onError={noop}
          onInspect={noop}
          onStatus={noop}
          selectedMap={null}
          selectedBuilding={null}
          onSelectBuilding={noop}
          onBuildingMoved={noop}
          selectedEvent={null}
          onSelectEvent={noop}
          selectedCell={null}
          onSelectCell={noop}
          onEventMoved={noop}
          onEventsLocked={noop}
          faultyEvents={[]}
          draft={d}
        />,
      ),
    );
  /** Lets the load of the matrix finish (no maps to fetch, the names are mocked). */
  const settle = () => act(() => new Promise<void>((resolve) => setTimeout(resolve, 0)));
  const host = () => container.querySelector<HTMLElement>("[data-draft]");

  it("host_carries_data_draft_and_its_count", async () => {
    show(null);
    await settle();
    expect(host()).not.toBeNull();
    expect(host()?.getAttribute("data-draft")).toBe("");
    expect(host()?.getAttribute("data-draft-count")).toBe("0");
    show(draft);
    await settle();
    expect(host()?.getAttribute("data-draft-count")).toBe("3");
    expect(host()?.getAttribute("data-draft-count")).toBe(String(draftCount(draft, matrix.index)));
    show(null);
    await settle();
    expect(host()?.getAttribute("data-draft-count")).toBe("0");
  });

  it("draws_a_draft_given_before_the_load_once_the_scene_is_built", async () => {
    show(draft);
    // The load of the matrix has not put it into the scene yet: nothing is drawn.
    expect(host()?.getAttribute("data-draft-count")).toBe("0");
    await settle();
    expect(host()?.getAttribute("data-draft-count")).toBe("3");
  });
});
