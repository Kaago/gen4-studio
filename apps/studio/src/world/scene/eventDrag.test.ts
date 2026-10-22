import { beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents, updateEvent } from "../../api";
import type { EventsDto, MatrixDto } from "../../api";
import type { EventRef } from "../events";
import { moveMarker, pickMarker } from "../markers";
import type { MarkerSet } from "../markers";
import { createEventDrag } from "./eventDrag";
import { fakePicker, fakeRefs, settle, surfaceKit } from "./dragTestKit";
import type { SceneHost } from "./SceneHost";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), updateEvent: vi.fn(), getEvents: vi.fn() }));
vi.mock("../markers", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../markers")>()),
  pickMarker: vi.fn(),
  moveMarker: vi.fn(),
  buildMarkers: vi.fn(() => ({ group: { visible: true } })),
  disposeMarkers: vi.fn(),
  selectMarker: vi.fn(),
}));

const matrix: MatrixDto = { index: 0, name: "m", width: 4, height: 4, maps: [], headers: null, altitudes: null };
const set = {} as MarkerSet;
const selected: EventRef = { file: 1, kind: "warp", index: 2 };
const hitOn = (index: number) => ({ kind: "warp", data: { index, x: 10, z: 10 } }) as never;

function setup(options: { locked?: string | null; select?: EventRef | null; paintEnabled?: boolean } = {}) {
  const events = { event_file_id: 1, locked: options.locked ?? null } as EventsDto;
  const h = {
    selectedEvent: options.select === undefined ? selected : options.select,
    events,
    markers: set,
    matrix,
    scene: { add: vi.fn(), remove: vi.fn() },
    render: vi.fn(),
    refreshHover: vi.fn(),
    disposed: false,
  } as unknown as SceneHost;
  const { refs, callbacks } = fakeRefs({ paintEnabled: options.paintEnabled ?? true });
  const report = vi.fn();
  const kit = surfaceKit();
  const drag = createEventDrag(h, fakePicker(), kit.surface, refs, report);
  drag.attach();
  return { h, callbacks, report, drag, ...kit };
}

beforeEach(() => {
  vi.mocked(updateEvent).mockReset().mockResolvedValue(undefined as never);
  vi.mocked(getEvents).mockReset().mockResolvedValue({ event_file_id: 1, locked: null } as EventsDto);
  vi.mocked(pickMarker).mockReset().mockReturnValue(hitOn(2));
  vi.mocked(moveMarker).mockReset();
});

describe("createEventDrag", () => {
  it("only begins on the selected marker", () => {
    const own = setup();
    own.press(100, 100);
    expect(own.drag.dragging).toBe(true);

    vi.mocked(pickMarker).mockReturnValue(hitOn(5));
    const other = setup();
    other.press(100, 100);
    expect(other.drag.dragging).toBe(false);

    vi.mocked(pickMarker).mockReturnValue(null);
    const none = setup();
    none.press(100, 100);
    expect(none.drag.dragging).toBe(false);
  });

  it("does not begin without a selection, in a locked file, or while painting is disabled", () => {
    for (const options of [{ select: null }, { locked: "read-only" }, { paintEnabled: false }, { select: { file: 9, kind: "warp", index: 2 } as EventRef }]) {
      const t = setup(options);
      t.press(100, 100);
      expect(t.drag.dragging).toBe(false);
    }
  });

  it("previews the move in whole tiles and writes it once when the pointer is released", async () => {
    const t = setup();
    t.press(100, 100);
    t.moveTo(103.4, 100);
    t.moveTo(105.2, 98);
    expect(updateEvent).not.toHaveBeenCalled();
    expect(moveMarker).toHaveBeenLastCalledWith(set, { kind: "warp", index: 2 }, { x: 15, z: 8 });
    t.release(105.2, 98);
    t.release(105.2, 98);
    expect(updateEvent).toHaveBeenCalledTimes(1);
    expect(updateEvent).toHaveBeenCalledWith(1, "warp", 2, { x: 15, z: 8 });
    await settle();
    expect(t.callbacks.onEventMoved).toHaveBeenCalledWith(1);
    // The events are read again and shown (the markers are rebuilt from what the backend holds).
    expect(getEvents).toHaveBeenCalledWith(1);
    expect(t.report).toHaveBeenCalledTimes(1);
  });

  it("puts the marker back on Escape and writes nothing", () => {
    const t = setup();
    t.press(100, 100);
    t.moveTo(106, 100);
    t.escape();
    expect(updateEvent).not.toHaveBeenCalled();
    expect(moveMarker).toHaveBeenLastCalledWith(set, { kind: "warp", index: 2 }, { x: 10, z: 10 });
    expect(t.drag.dragging).toBe(false);
  });

  it("puts the marker back and reports when the write is refused", async () => {
    vi.mocked(updateEvent).mockRejectedValue("refused");
    const t = setup();
    t.press(100, 100);
    t.moveTo(106, 100);
    t.release(106, 100);
    await settle();
    expect(moveMarker).toHaveBeenLastCalledWith(set, { kind: "warp", index: 2 }, { x: 10, z: 10 });
    expect(t.callbacks.onError).toHaveBeenCalledWith("refused");
    expect(t.callbacks.onEventMoved).not.toHaveBeenCalled();
  });
});
