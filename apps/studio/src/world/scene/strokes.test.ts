// @vitest-environment jsdom
import * as THREE from "three";
import { describe, expect, it, vi } from "vitest";
import type { MatrixDto } from "../../api";
import { flatGround } from "../ground";
import { lineCells, rectCells } from "../paint/ops";
import type { Cell, PaintSub } from "../paint/ops";
import { FakeTarget, fakePicker, fakeRefs } from "./dragTestKit.testkit";
import type { Picker } from "./picking";
import { SceneHost } from "./SceneHost";
import { attachStrokes } from "./strokes";
import type { PaintCallbacks } from "./strokes";

/** Matrix 0, 3 × 1: map 5 (the target), map 7 beside it, an empty cell. One pixel is one tile. */
const matrix: MatrixDto = { index: 0, name: "m", width: 3, height: 1, maps: [5, 7, null], headers: null, altitudes: null };

/** A picker over `matrix` with one pixel per tile: `pickTile` on loaded maps, `surfacePoint` over every matrix cell. */
function pixelPicker(): Picker {
  const picker = fakePicker();
  vi.mocked(picker.pickTile).mockImplementation((x, y) => {
    const col = Math.floor(x / 32);
    const row = Math.floor(y / 32);
    if (x < 0 || y < 0 || col >= matrix.width || row >= matrix.height) return null;
    const mapId = matrix.maps[row * matrix.width + col];
    if (mapId === null || mapId === undefined) return null;
    return { mapId, data: {} as never, col, row, x: Math.floor(x) % 32, y: Math.floor(y) % 32 };
  });
  vi.mocked(picker.surfacePoint).mockImplementation((x, y) => (x < 0 || y < 0 || x >= 96 || y >= 32 ? null : { x: x + 0.5, z: y + 0.5 }));
  return picker;
}

/** A host part strokes need, with the real pointer mapping of `SceneHost` on fake controls. */
function fakeHost(canvas: FakeTarget) {
  const h = Object.create(SceneHost.prototype) as SceneHost;
  const slanted = new THREE.PerspectiveCamera();
  Object.assign(h, {
    renderer: { domElement: canvas },
    matrix,
    ground: flatGround(matrix),
    paintPreview: new THREE.Group(),
    slanted,
    camera: slanted,
    controls: { mouseButtons: { LEFT: THREE.MOUSE.ROTATE, MIDDLE: THREE.MOUSE.DOLLY, RIGHT: THREE.MOUSE.PAN }, touches: { ONE: THREE.TOUCH.ROTATE } },
    render: vi.fn(),
  });
  return h;
}

function setup(sub: PaintSub | null = "brush", target: number | null = 5) {
  const canvas = new FakeTarget();
  const h = fakeHost(canvas);
  const paint: PaintCallbacks = { sub, target, onStroke: vi.fn(), onPick: vi.fn(), onSelectCell: vi.fn() };
  const { refs } = fakeRefs({ tool: "paint", paint });
  const detach = attachStrokes(h, pixelPicker(), refs);
  const ev = (x: number, y: number, over: { button?: number; shiftKey?: boolean; altKey?: boolean } = {}) => ({
    clientX: x,
    clientY: y,
    button: over.button ?? 0,
    pointerId: 1,
    shiftKey: over.shiftKey ?? false,
    altKey: over.altKey ?? false,
    stopPropagation: vi.fn(),
  });
  return {
    h,
    paint,
    refs,
    detach,
    down: (x: number, y: number, over?: { button?: number; shiftKey?: boolean; altKey?: boolean }) => canvas.emit("pointerdown", ev(x, y, over)),
    move: (x: number, y: number) => canvas.emit("pointermove", ev(x, y)),
    up: (x: number, y: number) => canvas.emit("pointerup", ev(x, y)),
    cancel: () => canvas.emit("pointercancel", ev(0, 0)),
    stroke: () => vi.mocked(paint.onStroke).mock.calls.map(([s]) => s),
  };
}

const plates = (h: SceneHost) => h.paintPreview.children.filter((c) => c instanceof THREE.InstancedMesh) as THREE.InstancedMesh[];

describe("attachStrokes", () => {
  it("a_drag_is_one_stroke_of_distinct_cells", () => {
    const t = setup();
    t.down(2, 2);
    t.move(3, 2);
    t.move(3, 2);
    t.move(2, 2);
    t.up(4, 2);
    expect(t.stroke()).toHaveLength(1);
    const [stroke] = t.stroke();
    expect(stroke.map).toBe(5);
    expect(stroke.sub).toBe("brush");
    expect(stroke.lower).toBe(false);
    expect(stroke.cells).toEqual([[2, 2], [3, 2], [4, 2]]);
  });

  it("fast_moves_fill_the_gaps", () => {
    const t = setup();
    t.down(0, 0);
    t.move(10, 5);
    // The preview follows while the pointer moves.
    expect(plates(t.h)[0].count).toBe(lineCells([0, 0], [10, 5]).length);
    t.up(10, 5);
    expect(t.stroke()[0].cells).toEqual(lineCells([0, 0], [10, 5]));
  });

  it("a_stroke_stays_on_its_map", () => {
    const t = setup("height");
    t.down(30, 10, { shiftKey: true });
    t.move(45, 10); // map 7
    t.move(70, 20); // the empty cell
    t.move(120, 10); // beside the matrix
    t.move(20, 25); // back onto map 5
    t.up(90, 40); // beside the matrix
    const [stroke] = t.stroke();
    expect(stroke.map).toBe(5);
    expect(stroke.lower).toBe(true);
    expect(stroke.cells.length).toBeGreaterThan(2);
    expect(stroke.cells.every(([x, z]) => x >= 0 && x <= 31 && z >= 0 && z <= 31)).toBe(true);
    expect(stroke.cells).toContainEqual([31, 10]);
    expect(stroke.cells).toContainEqual([20, 25]);
    expect(new Set(stroke.cells.map((c) => c.join())).size).toBe(stroke.cells.length);
  });

  it("pointerdown_on_another_map_selects_it_and_paints_nothing", () => {
    const t = setup();
    t.down(40, 10);
    t.move(42, 10);
    t.up(42, 10);
    expect(t.paint.onSelectCell).toHaveBeenCalledWith({ col: 1, row: 0 });
    expect(t.paint.onStroke).not.toHaveBeenCalled();
    expect(t.h.paintPreview.children).toHaveLength(0);
  });

  it("right_click_picks", () => {
    const t = setup("height");
    t.down(3, 4, { button: 2 });
    t.up(3, 4);
    expect(t.paint.onPick).toHaveBeenCalledWith(5, [3, 4] satisfies Cell);
    const pipette = setup("pick");
    pipette.down(6, 7);
    pipette.up(6, 7);
    expect(pipette.paint.onPick).toHaveBeenCalledWith(5, [6, 7]);
    expect(t.paint.onStroke).not.toHaveBeenCalled();
    expect(pipette.paint.onStroke).not.toHaveBeenCalled();
  });

  it("rect_reports_its_corners_and_draws_the_dashed_frame", () => {
    const t = setup("rect");
    t.down(2, 2);
    t.move(5, 4);
    const frame = t.h.paintPreview.children.find((c) => c instanceof THREE.LineSegments) as THREE.LineSegments | undefined;
    expect(frame?.material).toBeInstanceOf(THREE.LineDashedMaterial);
    expect(plates(t.h)[0].count).toBe(12);
    // A corner beyond the map stays on its edge.
    t.move(40, 4);
    t.up(40, 4);
    const [stroke] = t.stroke();
    expect(stroke.cells).toEqual(rectCells([2, 2], [31, 4]));
    expect(stroke.cells[0]).toEqual([2, 2]);
    expect(stroke.cells[stroke.cells.length - 1]).toEqual([31, 4]);
    // The frame goes on release; the plates stay until the answer.
    expect(t.h.paintPreview.children.some((c) => c instanceof THREE.LineSegments)).toBe(false);
    expect(plates(t.h)).toHaveLength(1);
    expect(plates(t.h)[0].userData.previewId).toBe(stroke.preview);
  });

  it("fill_reports_the_clicked_cell", () => {
    const t = setup("fill");
    t.down(8, 9);
    t.move(12, 9);
    t.up(12, 9);
    expect(t.stroke()[0].cells).toEqual([[8, 9]]);
  });

  it("cancel_drops_the_stroke_and_the_preview", () => {
    const t = setup();
    t.down(2, 2);
    t.move(6, 2);
    expect(plates(t.h)).toHaveLength(1);
    t.cancel();
    t.up(6, 2);
    expect(t.paint.onStroke).not.toHaveBeenCalled();
    expect(t.h.paintPreview.children).toHaveLength(0);
  });

  it("left_button_does_not_orbit_while_painting", () => {
    const t = setup();
    const buttons = t.h.controls.mouseButtons;
    t.down(2, 2);
    expect(buttons.LEFT).toBeNull();
    expect(buttons.MIDDLE).toBe(THREE.MOUSE.PAN);
    expect(buttons.RIGHT).toBeNull();
    t.up(2, 2);
    // ⌥ turns the camera instead of painting.
    t.down(3, 3, { altKey: true });
    expect(buttons.LEFT).toBe(THREE.MOUSE.ROTATE);
    t.up(3, 3);
    expect(t.stroke()).toHaveLength(1);
    // Leaving the paint sub-tools gives the camera its mapping back.
    t.refs.callbacks.current.paint = { ...t.paint, sub: null };
    t.down(2, 2);
    expect(buttons.LEFT).toBe(THREE.MOUSE.ROTATE);
    expect(buttons.MIDDLE).toBe(THREE.MOUSE.DOLLY);
    expect(buttons.RIGHT).toBe(THREE.MOUSE.PAN);
    expect(t.stroke()).toHaveLength(1);
  });
});
