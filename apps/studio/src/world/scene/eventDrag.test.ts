import { beforeEach, describe, expect, it, vi } from "vitest";
import { getEvents, updateEvent } from "../../api";
import type { EventsDto, MatrixDto } from "../../api";
import type { EventRef } from "../events";
import { MARKER_Y, moveMarker, pickMarker } from "../markers";
import type { MarkerSet } from "../markers";
import * as THREE from "three";
import { OVERLAY_Y } from "./content";
import { createEventDrag } from "./eventDrag";
import { fakePicker, fakeRefs, settle, slantedView, surfaceKit } from "./dragTestKit.testkit";
import type { Picker } from "./picking";
import type { SceneHost } from "./SceneHost";

vi.mock("../../api", async (importOriginal) => ({ ...(await importOriginal<typeof import("../../api")>()), updateEvent: vi.fn(), getEvents: vi.fn() }));
vi.mock("../markers", async (importOriginal) => ({
  ...(await importOriginal<typeof import("../markers")>()),
  pickMarker: vi.fn(),
  moveMarker: vi.fn(),
  buildMarkers: vi.fn(() => ({ group: { visible: true } })),
  disposeMarkers: vi.fn(),
  selectMarker: vi.fn(),
  markFaulty: vi.fn(),
}));

const matrix: MatrixDto = { index: 0, name: "m", width: 4, height: 4, maps: [], headers: null, altitudes: null };
const set = {} as MarkerSet;
const selected: EventRef = { file: 1, kind: "warp", index: 2 };
const hitOn = (index: number) => ({ kind: "warp", data: { index, x: 10, z: 10 } }) as never;

function setup(options: { locked?: string | null; select?: EventRef | null; paintEnabled?: boolean; altitudes?: number[]; matrix?: MatrixDto; picker?: Picker } = {}) {
  const events = { event_file_id: 1, locked: options.locked ?? null } as EventsDto;
  const h = {
    selectedEvent: options.select === undefined ? selected : options.select,
    faulty: [],
    events,
    markers: set,
    matrix: options.matrix ?? options.altitudes ? { ...matrix, altitudes: options.altitudes } : matrix,
    scene: { add: vi.fn(), remove: vi.fn() },
    render: vi.fn(),
    refreshHover: vi.fn(),
    disposed: false,
  } as unknown as SceneHost;
  const { refs, callbacks } = fakeRefs({ paintEnabled: options.paintEnabled ?? true });
  const report = vi.fn();
  const kit = surfaceKit();
  const picker = options.picker ?? fakePicker();
  const planePoint = vi.spyOn(picker, "planePoint");
  const drag = createEventDrag(h, picker, kit.surface, refs, report);
  drag.attach();
  return { h, callbacks, report, drag, planePoint, ...kit };
}

beforeEach(() => {
  vi.mocked(updateEvent).mockReset().mockResolvedValue(undefined as never);
  vi.mocked(getEvents).mockReset().mockResolvedValue({ event_file_id: 1, locked: null, objects: [] } as unknown as EventsDto);
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

  it("drags on the plane of the height of the cell the marker stands in", () => {
    // The marker is on tile (10, 10) of cell (0, 0); cell (0, 0) has altitude 4 (two tiles up).
    const raised = setup({ altitudes: [4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] });
    raised.press(100, 100);
    expect(raised.planePoint.mock.calls[0][2].constant).toBeCloseTo(-(MARKER_Y + 2), 5);
    const flat = setup();
    flat.press(100, 100);
    expect(flat.planePoint.mock.calls[0][2].constant).toBeCloseTo(-MARKER_Y, 5);
  });

  describe("across cells of another height", () => {
    // Cell 0 on the ground, cell 1 five tiles up (altitude 10), seen from a slanted camera.
    const view = (altitudes: number[]) => slantedView({ index: 0, name: "m", width: 2, height: 1, maps: [1, 2], headers: null, altitudes });
    const drag = (altitudes: number[], from: [number, number], to: [number, number]) => {
      const { h, picker, through } = view(altitudes);
      const at = (t: [number, number]) => {
        const cellHeight = (altitudes[Math.floor(t[0] / 32)] ?? 0) * 0.5;
        return through(t[0] + 0.5, OVERLAY_Y + cellHeight, t[1] + 0.5);
      };
      vi.mocked(pickMarker).mockReturnValue({ kind: "warp", data: { index: 2, x: from[0], z: from[1] } } as never);
      const t = setup({ matrix: h.matrix as MatrixDto, picker });
      const start = at(from);
      const end = at(to);
      t.press(start.x, start.y);
      t.moveTo(end.x, end.y);
      return t;
    };

    it("lands on exactly the tile under the pointer when dragged up onto a higher cell", () => {
      drag([0, 10], [26, 5], [36, 7]);
      expect(moveMarker).toHaveBeenLastCalledWith(set, { kind: "warp", index: 2 }, { x: 36, z: 7 });
    });

    it("lands on exactly the tile under the pointer when dragged down onto a lower cell", () => {
      drag([0, 10], [36, 7], [26, 5]);
      expect(moveMarker).toHaveBeenLastCalledWith(set, { kind: "warp", index: 2 }, { x: 26, z: 5 });
      drag([10, 0], [26, 5], [36, 7]);
      expect(moveMarker).toHaveBeenLastCalledWith(set, { kind: "warp", index: 2 }, { x: 36, z: 7 });
    });
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

  describe("an object shown as a sprite", () => {
    const object = { index: 3, local_id: 3, graphics_id: 1, movement_type: 0, script: 0, dir: 1, trainer_type: 0, sight: 0, hidden_flag: 0, x: 12, z: 13, y: 0 };
    const objectEvents: EventsDto = { event_file_id: 1, locked: null, bg: [], objects: [object], warps: [], coords: [] };

    /** A drag over the real sprite marker of the object: the mocked marker functions run the real ones. */
    async function spriteSetup() {
      const real = await vi.importActual<typeof import("../markers")>("../markers");
      const realSet = real.buildMarkers({ ...matrix, maps: [1, 1, 1, 1] }, objectEvents);
      const sprite = [...realSet.entries.keys()].find((o) => o instanceof THREE.Sprite) as THREE.Sprite;
      vi.mocked(pickMarker).mockReturnValue({ kind: "object", data: object } as never);
      vi.mocked(moveMarker).mockImplementation(real.moveMarker);
      const t = setup({ select: { file: 1, kind: "object", index: 3 } });
      t.h.markers = realSet;
      return { ...t, sprite, realSet };
    }

    it("moves the sprite with the pointer in whole tiles and writes the new tile once on release", async () => {
      const t = await spriteSetup();
      t.press(100, 100);
      t.moveTo(105.2, 98.4);
      expect(t.sprite.position.toArray()).toEqual([17.5, 0, 11.5]);
      expect(updateEvent).not.toHaveBeenCalled();
      t.release(105.2, 98.4);
      expect(updateEvent).toHaveBeenCalledTimes(1);
      expect(updateEvent).toHaveBeenCalledWith(1, "object", 3, { x: 17, z: 11 });
      await settle();
      expect(t.callbacks.onEventMoved).toHaveBeenCalledWith(1);
    });

    it("puts the sprite back on its tile on Escape", async () => {
      const t = await spriteSetup();
      t.press(100, 100);
      t.moveTo(106, 100);
      expect(t.sprite.position.x).toBe(18.5);
      t.escape();
      expect(t.sprite.position.toArray()).toEqual([12.5, 0, 13.5]);
      expect(updateEvent).not.toHaveBeenCalled();
    });
  });
});
