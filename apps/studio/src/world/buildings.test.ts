import { describe, expect, it } from "vitest";
import {
  buildingFallbackBox,
  buildingKey,
  buildingModelIds,
  buildingTransform,
  isTypingTarget,
  newBuildingPosition,
  sameBuildings,
  snapHalfTile,
  wantsDelete,
} from "./buildings";

const b = (model_id: number) => ({ model_id, x: 0, y: 0, z: 0, sx: 1, sy: 1, sz: 1, rotation: ["00000000", "00000000", "00000000"] as [string, string, string] });

describe("buildings", () => {
  it("places a building model with the map scale times its own scale", () => {
    const t = buildingTransform({ model_id: 1, x: -4, y: 0, z: 6, sx: 2, sy: 1, sz: 3 }, { x: 64, z: 32 });
    expect(t.position).toEqual([76, 0, 54]);
    expect(t.scale).toEqual([2 / 16, 1 / 16, 3 / 16]);
  });

  it("lifts the placeholder box by half its height, sized by the scale factors", () => {
    expect(buildingFallbackBox({ x: 0, y: 2, z: 0, sx: 2, sy: 1, sz: 3 })).toEqual({ offsetY: 0.5, size: [2, 1, 3] });
  });

  it("snaps to half tiles", () => {
    expect(snapHalfTile(1.26)).toBe(1.5);
    expect(snapHalfTile(1.24)).toBe(1);
    expect(snapHalfTile(-0.3)).toBe(-0.5);
    expect(Object.is(snapHalfTile(-0.2), 0)).toBe(true);
  });

  it("keys a building by its map and index", () => {
    expect(buildingKey(12, 3)).toBe("12:3");
  });

  it("lists each model id once, in order of first use", () => {
    expect(buildingModelIds([b(7), b(3), b(7), b(0)])).toEqual([7, 3, 0]);
    expect(buildingModelIds([])).toEqual([]);
  });

  it("puts a new building at the centre of the hovered tile", () => {
    // Tile (16, 16) is the first one right/below the map centre.
    expect(newBuildingPosition(16, 16)).toEqual({ x: 0.5, z: 0.5 });
    expect(newBuildingPosition(0, 31)).toEqual({ x: -15.5, z: 15.5 });
  });

  it("does not treat keys typed into form fields as scene shortcuts", () => {
    expect(isTypingTarget({ tagName: "INPUT" })).toBe(true);
    expect(isTypingTarget({ tagName: "TEXTAREA" })).toBe(true);
    expect(isTypingTarget({ tagName: "SELECT" })).toBe(true);
    expect(isTypingTarget({ tagName: "DIV", isContentEditable: true })).toBe(true);
    expect(isTypingTarget({ tagName: "CANVAS" })).toBe(false);
    expect(isTypingTarget(null)).toBe(false);
  });

  it("deletes on Delete or Backspace, but never while typing or with a modifier", () => {
    const key = (k: string, extra: object = {}) => ({ key: k, target: { tagName: "BODY" }, metaKey: false, ctrlKey: false, altKey: false, ...extra });
    expect(wantsDelete(key("Delete"))).toBe(true);
    expect(wantsDelete(key("Backspace"))).toBe(true);
    expect(wantsDelete(key("a"))).toBe(false);
    expect(wantsDelete(key("Delete", { target: { tagName: "INPUT" } }))).toBe(false);
    expect(wantsDelete(key("Delete", { metaKey: true }))).toBe(false);
  });

  it("compares building lists field by field", () => {
    expect(sameBuildings([b(1), b(2)], [b(1), b(2)])).toBe(true);
    expect(sameBuildings([b(1)], [b(1), b(2)])).toBe(false);
    expect(sameBuildings([b(1)], [b(2)])).toBe(false);
    expect(sameBuildings([b(1)], [{ ...b(1), z: 0.5 }])).toBe(false);
    expect(sameBuildings([b(1)], [{ ...b(1), rotation: ["00000000", "00000001", "00000000"] }])).toBe(false);
    expect(sameBuildings([], [])).toBe(true);
  });
});
